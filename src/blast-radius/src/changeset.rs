// MODE: DEV
// PACKAGE: PROD
use crate::git;
use std::path::Path;

/// The change set: explicit positional paths verbatim, or a transform of
/// `git status --porcelain` when none are given.
///
/// AR-71: callers must compute both the "no changes to analyse" decision and
/// the changed-path(s) banner count from [`non_blank_count`], never from
/// `Vec::len()` directly -- a blank entry (e.g. an explicit empty positional
/// argument) must not count as a changed path, matching bash's own
/// `grep -c .` semantics.
pub fn changed_paths(repo_root: &Path, explicit: &[String]) -> Vec<String> {
    if !explicit.is_empty() {
        return explicit.to_vec();
    }
    git::status_porcelain(repo_root)
        .lines()
        .map(transform_porcelain_line)
        .collect()
}

pub fn non_blank_count(paths: &[String]) -> usize {
    paths.iter().filter(|p| !p.is_empty()).count()
}

/// Reproduces the real awk transform: strip the 2-character status prefix
/// and the following run of spaces, and for a rename line (containing the
/// literal substring `" -> "`) keep only the text after the LAST such
/// occurrence.
fn transform_porcelain_line(line: &str) -> String {
    let rest = if line.chars().count() >= 2 {
        let mut chars = line.chars();
        chars.next();
        chars.next();
        chars.as_str()
    } else {
        ""
    };
    let rest = rest.trim_start_matches(' ');
    match rest.rfind(" -> ") {
        Some(idx) => rest[idx + 4..].to_string(),
        None => rest.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_ordinary_modified_line_strips_the_status_prefix() {
        assert_eq!(
            transform_porcelain_line(" M path/to/file.txt"),
            "path/to/file.txt"
        );
        assert_eq!(transform_porcelain_line("?? newfile.txt"), "newfile.txt");
        assert_eq!(transform_porcelain_line("MM file.txt"), "file.txt");
    }

    #[test]
    fn a_rename_line_keeps_only_the_text_after_the_last_arrow() {
        assert_eq!(transform_porcelain_line("R  old -> new"), "new");
        assert_eq!(
            transform_porcelain_line("R  a -> b -> c"),
            "c",
            "must keep the text after the LAST arrow, not the first"
        );
    }

    #[test]
    fn non_blank_count_ignores_blank_entries() {
        assert_eq!(
            non_blank_count(&["a".to_string(), "".to_string(), "b".to_string()]),
            2
        );
        assert_eq!(non_blank_count(&["".to_string()]), 0);
        assert_eq!(non_blank_count(&[]), 0);
    }
}
