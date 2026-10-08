// MODE: DEV
// PACKAGE: PROD
//! Line-level diffing and hunk-based merge construction backing
//! `resolve_external`'s `merge` resolution (see `ai-text-editor-server.rs`).

/// Which of the two inputs to [`diff_hunks`] failed to decode as UTF-8.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffSide {
    Base,
    Side,
}

/// Failure mode for [`diff_hunks`]. Base or side bytes that are not valid
/// UTF-8 is an explicit error, never a panic or a silent lossy decode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum DiffError {
    #[error("{0:?} input is not valid UTF-8")]
    InvalidUtf8(DiffSide),
}

/// One changed region relative to a shared base: `base_start`/`base_end` are
/// 1-based, inclusive line numbers naming the base range this hunk replaces,
/// and `replacement` is the side's own replacement lines for that range.
///
/// A pure insertion -- content added with no base line removed -- is
/// represented as `base_end == base_start - 1`: a zero-length range
/// immediately before `base_start`, consuming zero base lines. This is not a
/// special case the rest of this module branches on; [`hunks_conflict`] and
/// [`apply_merge`] both handle it uniformly through the same base_start/
/// base_end arithmetic ordinary non-empty ranges use.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hunk {
    pub base_start: usize,
    pub base_end: usize,
    pub replacement: Vec<String>,
}

/// Splits `text` into lines, each line carrying its own trailing `\n` (or
/// `\r\n`, since the `\r` is simply part of the line's own content) where one
/// followed it in the source, and the final line carrying none when `text`
/// has no trailing newline. An empty `text` splits to zero lines, never one
/// spurious empty line. Concatenating the result reproduces `text` exactly
/// (`lines.concat() == text`), which is what lets a later rejoin in
/// `apply_merge` round-trip CRLF and a missing final trailing newline without
/// any extra bookkeeping.
fn split_lines(text: &str) -> Vec<&str> {
    if text.is_empty() {
        return Vec::new();
    }
    let mut lines = Vec::new();
    let mut start = 0;
    for (i, byte) in text.bytes().enumerate() {
        if byte == b'\n' {
            lines.push(&text[start..=i]);
            start = i + 1;
        }
    }
    if start < text.len() {
        lines.push(&text[start..]);
    }
    lines
}

/// Given base bytes and one side's bytes, returns the set of changed hunks
/// relative to base. Decodes both inputs as UTF-8 (returning
/// `Err(DiffError::InvalidUtf8(..))` naming the failing side if either does
/// not decode, never panicking or lossily substituting), then diffs them
/// line by line using `similar`, coalescing its output into whole-line
/// `Hunk`s.
pub fn diff_hunks(base: &[u8], side: &[u8]) -> Result<Vec<Hunk>, DiffError> {
    let base_str = std::str::from_utf8(base).map_err(|_| DiffError::InvalidUtf8(DiffSide::Base))?;
    let side_str = std::str::from_utf8(side).map_err(|_| DiffError::InvalidUtf8(DiffSide::Side))?;

    let base_lines = split_lines(base_str);
    let side_lines = split_lines(side_str);

    let diff = similar::TextDiff::from_slices(&base_lines, &side_lines);

    let mut hunks = Vec::new();
    for op in diff.ops() {
        if op.tag() == similar::DiffTag::Equal {
            continue;
        }
        let old_range = op.old_range();
        let new_range = op.new_range();
        let base_start = old_range.start + 1;
        let base_end = if old_range.is_empty() {
            old_range.start
        } else {
            old_range.end
        };
        let replacement: Vec<String> = side_lines[new_range]
            .iter()
            .map(|s| s.to_string())
            .collect();
        hunks.push(Hunk {
            base_start,
            base_end,
            replacement,
        });
    }
    Ok(hunks)
}

/// Returns true if any hunk in `a` conflicts with any hunk in `b`. The
/// general rule is `x.base_start <= y.base_end + 1 && y.base_start <=
/// x.base_end + 1` -- equivalent to overlap-or-adjacent (a zero-line gap)
/// for ordinary non-empty ranges, and uniform for a pure-insertion hunk's
/// zero-length range (`base_end == base_start - 1`) with no special-casing.
/// Two insertions conflict only when their `base_start` values are
/// identical, since any distinct position already leaves a full unchanged
/// base line between them, which this same formula correctly reports as not
/// conflicting.
pub fn hunks_conflict(a: &[Hunk], b: &[Hunk]) -> bool {
    a.iter().any(|x| {
        b.iter()
            .any(|y| x.base_start <= y.base_end + 1 && y.base_start <= x.base_end + 1)
    })
}

/// Builds the correctly merged document from `base` plus two
/// already-confirmed-non-conflicting sets of hunks (the buffer's own and the
/// external side's). Infallible: by the time this is called, `base` has
/// already been successfully UTF-8-decoded twice, once by each of the two
/// `diff_hunks` calls that produced `buffer_hunks` and `external_hunks`, so a
/// decode failure is handled entirely upstream and cannot occur here.
///
/// Callers are responsible for calling [`hunks_conflict`] first and only
/// calling this when it returns `false`; this function does not re-check for
/// conflicts, and its behavior on conflicting hunks is unspecified.
///
/// # Panics
///
/// Panics if `base` is not valid UTF-8 -- a violation of the precondition
/// above, not an expected runtime outcome.
pub fn apply_merge(base: &[u8], buffer_hunks: &[Hunk], external_hunks: &[Hunk]) -> Vec<u8> {
    let base_str = std::str::from_utf8(base)
        .expect("apply_merge's base must already be valid UTF-8, decoded upstream by diff_hunks");
    let base_lines = split_lines(base_str);

    let mut hunks: Vec<&Hunk> = buffer_hunks.iter().chain(external_hunks.iter()).collect();
    hunks.sort_by_key(|hunk| hunk.base_start);

    let mut output = String::new();
    let mut pos = 1usize;
    for hunk in hunks {
        while pos < hunk.base_start {
            output.push_str(base_lines[pos - 1]);
            pos += 1;
        }
        for line in &hunk.replacement {
            output.push_str(line);
        }
        pos = hunk.base_end + 1;
    }
    while pos <= base_lines.len() {
        output.push_str(base_lines[pos - 1]);
        pos += 1;
    }
    output.into_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diff_hunks_reports_the_hunk_that_changed_including_line_ending_variants() {
        let base = b"one\ntwo\nthree\nfour\nfive\n";
        let side = b"one\nTWO\nthree\nfour\nfive\n";
        let hunks = diff_hunks(base, side).expect("valid UTF-8 on both sides");
        assert_eq!(
            hunks,
            vec![Hunk {
                base_start: 2,
                base_end: 2,
                replacement: vec!["TWO\n".to_string()],
            }]
        );

        // A base lacking a final trailing newline, and an unrelated unchanged
        // line carrying CRLF on both sides -- only the final, newline-less
        // line actually changes.
        let base = b"first\r\nsecond";
        let side = b"first\r\nTHIRD";
        let hunks = diff_hunks(base, side).expect("valid UTF-8 on both sides");
        assert_eq!(
            hunks,
            vec![Hunk {
                base_start: 2,
                base_end: 2,
                replacement: vec!["THIRD".to_string()],
            }]
        );

        // A pure insertion: content added with no base line removed.
        let base = b"one\ntwo\n";
        let side = b"one\nONE-POINT-FIVE\ntwo\n";
        let hunks = diff_hunks(base, side).expect("valid UTF-8 on both sides");
        assert_eq!(
            hunks,
            vec![Hunk {
                base_start: 2,
                base_end: 1,
                replacement: vec!["ONE-POINT-FIVE\n".to_string()],
            }]
        );

        // Invalid UTF-8 on the side.
        let base = b"one\n";
        let side: &[u8] = &[0xFF, 0xFE];
        assert_eq!(
            diff_hunks(base, side),
            Err(DiffError::InvalidUtf8(DiffSide::Side))
        );

        // Invalid UTF-8 on the base.
        let base: &[u8] = &[0xFF, 0xFE];
        let side = b"one\n";
        assert_eq!(
            diff_hunks(base, side),
            Err(DiffError::InvalidUtf8(DiffSide::Base))
        );
    }

    #[test]
    fn diff_hunks_is_empty_for_identical_input() {
        let text = b"same\ntext\n";
        assert_eq!(diff_hunks(text, text), Ok(Vec::new()));
    }

    fn hunk(base_start: usize, base_end: usize) -> Hunk {
        Hunk {
            base_start,
            base_end,
            replacement: Vec::new(),
        }
    }

    #[test]
    fn hunks_conflict_is_true_for_overlapping_hunks() {
        // Two ordinary hunks sharing base line 3.
        assert!(hunks_conflict(&[hunk(2, 3)], &[hunk(3, 3)]));

        // A pure-insertion hunk at the same base_start as an ordinary hunk.
        assert!(hunks_conflict(&[hunk(3, 2)], &[hunk(3, 4)]));

        // Two pure-insertion hunks at the exact same base position.
        assert!(hunks_conflict(&[hunk(5, 4)], &[hunk(5, 4)]));
    }

    #[test]
    fn hunks_conflict_is_true_for_adjacent_hunks() {
        // Two ordinary hunks with a zero-line gap (touching, not overlapping).
        assert!(hunks_conflict(&[hunk(2, 3)], &[hunk(4, 4)]));

        // A pure-insertion hunk positioned exactly one line from an ordinary
        // hunk's range (directly after it, zero gap).
        assert!(hunks_conflict(&[hunk(4, 4)], &[hunk(5, 4)]));

        // No insertion-vs-insertion sub-case belongs here (AR-20, cycle 6):
        // two zero-width ranges are adjacent only by identical position
        // (hunks_conflict_is_true_for_overlapping_hunks) or not conflicting
        // at all once their positions differ, including the tightest
        // one-line-apart boundary (hunks_conflict_is_false_for_separated_hunks).
    }

    #[test]
    fn hunks_conflict_is_false_for_separated_hunks() {
        // Two ordinary hunks separated by at least one unchanged line.
        assert!(!hunks_conflict(&[hunk(2, 2)], &[hunk(4, 4)]));

        // A pure-insertion hunk positioned at least two lines from an
        // ordinary hunk's range.
        assert!(!hunks_conflict(&[hunk(4, 3)], &[hunk(6, 6)]));

        // Two pure-insertion hunks at well-separated base positions.
        assert!(!hunks_conflict(&[hunk(2, 1)], &[hunk(8, 7)]));

        // The tightest possible distinct-position case for two insertions:
        // base_start values exactly one apart (AR-20, cycle 6, relocated
        // from the adjacent-hunks unit where it was wrongly asserted as
        // conflicting -- two zero-width ranges at different positions are
        // never adjacent, only identical or separated).
        assert!(!hunks_conflict(&[hunk(4, 3)], &[hunk(5, 4)]));
    }

    #[test]
    fn apply_merge_correctly_combines_hunks_with_differing_line_counts() {
        let base = b"A\nB\nC\nD\nE\n";

        // The AR-3 counter-example: a buffer hunk replacing base lines 1-2
        // with three lines (a net +1 line shift) and an external hunk
        // replacing base line 4 only -- base line 3 ("C") must survive as
        // untouched context, and base line 4 ("D") must be correctly
        // replaced rather than "C" being overwritten.
        let buffer_hunks = vec![Hunk {
            base_start: 1,
            base_end: 2,
            replacement: vec!["X\n".to_string(), "Y\n".to_string(), "Z\n".to_string()],
        }];
        let external_hunks = vec![Hunk {
            base_start: 4,
            base_end: 4,
            replacement: vec!["Dprime\n".to_string()],
        }];
        assert!(!hunks_conflict(&buffer_hunks, &external_hunks));
        let merged = apply_merge(base, &buffer_hunks, &external_hunks);
        assert_eq!(
            String::from_utf8(merged).unwrap(),
            "X\nY\nZ\nC\nDprime\nE\n"
        );

        // AR-8: a pure-insertion hunk on one side merged against an ordinary
        // hunk on the other, non-conflicting -- the inserted content must
        // appear exactly once, with no duplication and no infinite loop.
        let buffer_hunks = vec![Hunk {
            base_start: 3,
            base_end: 2,
            replacement: vec!["INSERTED\n".to_string()],
        }];
        let external_hunks = vec![Hunk {
            base_start: 5,
            base_end: 5,
            replacement: vec!["Eprime\n".to_string()],
        }];
        assert!(!hunks_conflict(&buffer_hunks, &external_hunks));
        let merged = apply_merge(base, &buffer_hunks, &external_hunks);
        assert_eq!(
            String::from_utf8(merged).unwrap(),
            "A\nB\nINSERTED\nC\nD\nEprime\n"
        );
    }
}
