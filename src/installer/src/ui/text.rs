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
    if text.len() > width {
        return if width == 0 {
            String::new()
        } else if width < 4 {
            ".".repeat(width)
        } else {
            format!("{}...", &text[..width - 3])
        };
    }
    format!("{text:<width$}")
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
        if remaining.len() <= width {
            lines.push(remaining.to_string());
            break;
        }
        let candidate = &remaining[..width];
        match candidate.rfind(' ') {
            Some(split) if split > 0 => {
                lines.push(candidate[..split].to_string());
                remaining = remaining[split..].trim_start_matches(' ');
            }
            _ => {
                lines.push(format!("{}-", &remaining[..width - 1]));
                remaining = &remaining[width - 1..];
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

#[cfg(test)]
mod tests {
    use super::*;

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
