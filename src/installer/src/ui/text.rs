// MODE: DEV
// PACKAGE: PROD
//! Shared text-shaping helpers for the picker's fixed-width frame:
//! `pad` (exact-width, hard-truncates when it must), `wrap` (lossless
//! word-wrap, used for content that must never be cut), and `overflow`
//! (bounded word-wrap: wraps onto extra lines first, and truncates only
//! the last of them if it still doesn't fit). `layout` needs these to size
//! the frame before it renders (how many physical rows does the title bar
//! actually need at this width?); `render` and `uninstall_picker` need them
//! to build the content itself. Both live under `ui/`, so this is a
//! sibling rather than living in either -- `layout` cannot depend on
//! `render` without a cycle (`render` already depends on `layout::Layout`).

/// Pads `text` to exactly `width` cells, or hard-truncates it to fit,
/// ending in `...` when it must cut. An ellipsis reads unambiguously as
/// "there was more, cut here" at a glance; a bare `~` (this function's
/// previous marker) does not carry that meaning on sight and, in this
/// picker, doubles as the unrelated Degraded-skill suffix -- ambiguous on
/// both counts. Never used for content where the cut text itself matters
/// (see `overflow`'s and `wrap`'s own docs for the alternative).
pub(crate) fn pad(text: &str, width: usize) -> String {
    if text.chars().count() > width {
        return if width == 0 {
            String::new()
        } else if width < 4 {
            ".".repeat(width)
        } else {
            format!("{}...", &text[..char_byte_index(text, width - 3)])
        };
    }
    format!("{text:<width$}")
}

/// Byte offset of the `n`th character of `text`, or its length when shorter.
fn char_byte_index(text: &str, n: usize) -> usize {
    text.char_indices().nth(n).map_or(text.len(), |(i, _)| i)
}

/// Like `pad`, but measured by displayed CHARACTER count rather than byte
/// length -- safe for a line that may carry multi-byte UTF-8 (a Unicode
/// box-drawing border, a `progress_bar` fill character), where `pad`'s own
/// byte-length measurement would either miscount the padding needed or
/// panic slicing mid-character on truncation (the exact hazard
/// `wizard.rs`'s `is_precomposed_line` already documents for this same
/// glyph-width reason -- reproduced live: `progress_bar`'s `█` fill
/// character, at 3 bytes but 1 display column, panicked exactly this way
/// before `render.rs`'s dependency table and this caller both switched to
/// this function). Never truncates with an ellipsis the way `pad` does:
/// every caller here builds a line already known to be at most `width`
/// characters, so overflow is not the case this needs to handle
/// gracefully.
pub(crate) fn pad_display(text: &str, width: usize) -> String {
    let len = text.chars().count();
    if len >= width {
        text.chars().take(width).collect()
    } else {
        format!("{text}{}", " ".repeat(width - len))
    }
}

/// Word-wraps to `width`, hyphenating a token wider than the pane. Never
/// drops a byte of `text` -- wrapping only ever adds a line break (and, for
/// an unbreakable token, a hyphen), unlike `pad`, which discards whatever
/// doesn't fit. Use this directly, with no line cap, for content that must
/// never be truncated regardless of how many lines it takes -- a
/// destructive-action confirmation naming the exact path it will remove,
/// for instance.
pub(crate) fn wrap(text: &str, width: usize) -> Vec<String> {
    let width = width.max(8);
    let mut lines = Vec::new();
    let mut remaining = text;
    while !remaining.is_empty() {
        if remaining.chars().count() <= width {
            lines.push(remaining.to_string());
            break;
        }
        let candidate = &remaining[..char_byte_index(remaining, width)];
        match candidate.rfind(' ') {
            Some(split) if split > 0 => {
                lines.push(candidate[..split].to_string());
                remaining = remaining[split..].trim_start_matches(' ');
            }
            _ => {
                let cut = char_byte_index(remaining, width - 1);
                lines.push(format!("{}-", &remaining[..cut]));
                remaining = &remaining[cut..];
            }
        }
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

/// Wraps `text` to `width`, using at most `max_lines` physical lines. For
/// non-critical content where a line cap is acceptable (chrome such as the
/// title/hint bars, or a single list row): wrapping is tried first, so
/// short overruns just take a second line instead of losing text outright,
/// and `pad`'s `...` truncation -- when it happens at all -- lands only on
/// the FINAL line, never an earlier one. Critical content should call
/// `wrap` directly instead, uncapped, so it is never truncated at all.
pub(crate) fn overflow(text: &str, width: usize, max_lines: usize) -> Vec<String> {
    if max_lines == 0 {
        return Vec::new();
    }
    let wrapped = wrap(text, width);
    if wrapped.len() <= max_lines {
        return wrapped.iter().map(|l| pad(l, width)).collect();
    }
    let mut out: Vec<String> = wrapped[..max_lines - 1]
        .iter()
        .map(|l| pad(l, width))
        .collect();
    let remainder = wrapped[max_lines - 1..].join(" ");
    out.push(pad(&remainder, width));
    out
}

/// True for a line `wrap`/`pad` cannot safely measure by byte length alone:
/// an escape-sequence-carrying line (its invisible SGR bytes would inflate
/// the count and wrap/truncate it too early -- ASCII, so never a panic risk,
/// just a wrong width) or a genuinely multi-byte one (a box-drawing divider
/// or border glyph, e.g. `─` at 3 UTF-8 bytes per display column -- a panic
/// risk too, since byte-slicing mid-character is undefined behavior `wrap`'s
/// own `&remaining[..width]` will hit the instant such a line is long enough
/// to need "wrapping" by its BYTE count). Either way the line is assumed to
/// already be exactly as wide as its builder intended, and every caller that
/// pre-builds a fully composed, already-exactly-sized line (a colored
/// button, a reverse-video cursor row, a Unicode divider) uses this to skip
/// re-wrapping/re-padding it. Shared by `wizard` and `render` rather than
/// defined twice, since both build precomposed lines the same way.
pub(crate) fn is_precomposed_line(line: &str) -> bool {
    line.contains('\x1b') || line.len() != line.chars().count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_precomposed_line_flags_an_escape_sequence() {
        assert!(is_precomposed_line("\x1b[7mhi\x1b[0m"));
    }

    #[test]
    fn is_precomposed_line_flags_multi_byte_utf8() {
        assert!(is_precomposed_line("──"));
    }

    #[test]
    fn is_precomposed_line_is_false_for_plain_ascii() {
        assert!(!is_precomposed_line("plain text"));
    }

    #[test]
    fn pad_truncates_with_an_ellipsis_not_a_tilde() {
        let padded = pad("a very long line of text", 10);
        assert_eq!(padded.len(), 10);
        assert!(padded.ends_with("..."));
        assert!(!padded.contains('~'));
    }

    #[test]
    fn pad_leaves_short_text_untouched_and_space_padded() {
        assert_eq!(pad("hi", 5), "hi   ");
    }

    #[test]
    fn wrap_hyphenates_a_token_wider_than_the_pane() {
        let lines = wrap("supercalifragilisticexpialidocious", 10);
        assert!(lines.len() > 1);
        assert!(lines[0].ends_with('-'));
    }

    #[test]
    fn wrap_breaks_on_whole_words_when_it_can() {
        let lines = wrap("one two three four", 8);
        for line in &lines {
            assert!(!line.contains("  "));
        }
        assert_eq!(lines.join(" "), "one two three four");
    }

    /// A 3-byte arrow straddling the cut point must not panic, and every
    /// line must fit in `width` displayed characters, not bytes.
    #[test]
    fn wrap_and_pad_measure_and_cut_by_character_not_byte() {
        for width in 8..40 {
            for lead in 0..12 {
                let text = format!("{}x \u{2192} /a/very/long/path/name/here", "y".repeat(lead));
                for line in wrap(&text, width) {
                    assert!(line.chars().count() <= width, "{width}: {line:?}");
                }
                let token = format!("{}\u{2192}{}", "y".repeat(lead), "z".repeat(40));
                assert!(wrap(&token, width)
                    .iter()
                    .all(|l| l.chars().count() <= width));
                assert_eq!(
                    pad(&text, width).chars().count(),
                    width,
                    "{width}: {text:?}"
                );
            }
        }
        assert_eq!(pad("a\u{2192}b", 5), "a\u{2192}b  ");
    }

    #[test]
    fn wrap_never_drops_a_byte_no_matter_how_long_the_text() {
        let text = "word ".repeat(200);
        let lines = wrap(text.trim(), 12);
        let rejoined = lines.join(" ");
        assert_eq!(rejoined, text.trim());
    }

    #[test]
    fn overflow_returns_wrapped_lines_untouched_when_they_fit_the_cap() {
        let lines = overflow("one two three", 20, 3);
        assert_eq!(lines, vec![pad("one two three", 20)]);
    }

    #[test]
    fn overflow_truncates_only_the_final_line_never_an_earlier_one() {
        // Long enough that it needs more than 3 lines of plain wrapping,
        // so the cap must bite.
        let text = "alpha beta gamma delta epsilon zeta eta theta iota kappa";
        let lines = overflow(text, 10, 3);
        assert_eq!(lines.len(), 3, "lines were: {lines:?}");
        assert!(!lines[0].contains("..."), "first line truncated: {lines:?}");
        assert!(
            !lines[1].contains("..."),
            "second line truncated: {lines:?}"
        );
        assert!(
            lines[2].ends_with("..."),
            "third line was not truncated: {lines:?}"
        );
    }

    #[test]
    fn overflow_with_a_cap_of_one_behaves_like_plain_pad() {
        let text = "alpha beta gamma delta epsilon";
        let lines = overflow(text, 10, 1);
        assert_eq!(lines, vec![pad(text, 10)]);
    }

    #[test]
    fn overflow_with_zero_max_lines_yields_nothing() {
        assert!(overflow("anything", 10, 0).is_empty());
    }
}
