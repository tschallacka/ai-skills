// MODE: DEV
// PACKAGE: PROD
//! `# COVERS: <path> <path> ...` marker scanning and matching. A test may
//! declare what it covers with this marker in its own first 5 lines; a
//! changed path "hits" a COVERS entry when it equals the entry or begins
//! with "<entry>/" (a directory entry covers everything under it, a file
//! entry covers only itself). A test with NO marker is UNDECLARED and
//! always runs; a marker whose own entries match nothing in the change set
//! (including an empty entry list) means the item is EXCLUDED, not run.

use std::fs;
use std::path::Path;

const MARKER_PREFIX: &str = "# COVERS: ";

/// Reads only the first 5 lines of `item` (a repo-relative path) looking for
/// the first line beginning with `# COVERS: `, matching bash's own
/// `sed -n '1,5{/^# COVERS: /p;}' | head -1` exactly: a marker on line 6 or
/// later is never honoured, and only the first matching line within the
/// window counts. A path that is not a regular file (a crate directory, or
/// anything `fs::read_to_string` cannot open) has no marker at all.
pub fn read_marker(repo_root: &Path, item: &str) -> Option<String> {
    let full_path = repo_root.join(item);
    if !full_path.is_file() {
        return None;
    }
    let text = fs::read_to_string(&full_path).ok()?;
    text.lines()
        .take(5)
        .find(|line| line.starts_with(MARKER_PREFIX))
        .map(|line| line[MARKER_PREFIX.len()..].to_string())
}

/// Whether any entry in `marker_value` (a space/tab/newline-separated list)
/// hits any path in `changed` -- exact match, or `<entry>/` as a directory
/// prefix. An empty or all-blank `marker_value` matches nothing.
pub fn covers_hits(marker_value: &str, changed: &[String]) -> bool {
    for entry in marker_value.split_whitespace() {
        for path in changed {
            if path == entry || path.starts_with(&format!("{entry}/")) {
                return true;
            }
        }
    }
    false
}

pub struct Selection {
    pub selected: Vec<String>,
    pub excluded_count: u64,
}

/// Applies the COVERS decision to every item in `items`, in the SAME order
/// `run-tests.sh --list-only` produced them (never re-sorted). `excluded_count`
/// counts only items that HAD a marker and were excluded by it -- an
/// undeclared (marker-less) item is never counted toward it even though it
/// was considered.
pub fn select(repo_root: &Path, items: &[String], changed: &[String]) -> Selection {
    let mut selected = Vec::new();
    let mut excluded_count = 0u64;
    for item in items {
        match read_marker(repo_root, item) {
            None => selected.push(item.clone()),
            Some(marker_value) => {
                if covers_hits(&marker_value, changed) {
                    selected.push(item.clone());
                } else {
                    excluded_count += 1;
                }
            }
        }
    }
    Selection {
        selected,
        excluded_count,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn scratch(tag: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("ci-test-scope-covers-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn v(paths: &[&str]) -> Vec<String> {
        paths.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn covers_hits_exact_match() {
        assert!(covers_hits(
            "src/bug-report/src/resolve.rs",
            &v(&["src/bug-report/src/resolve.rs"])
        ));
    }

    #[test]
    fn covers_hits_directory_prefix_match() {
        assert!(covers_hits("chat", &v(&["chat/SKILL.md"])));
    }

    #[test]
    fn covers_hits_does_not_match_a_same_prefixed_sibling() {
        // "chat" must not match "chat-proto/x" -- that is a different
        // top-level name that merely starts with the same characters, not a
        // path beneath "chat/".
        assert!(!covers_hits("chat", &v(&["chat-proto/src/lib.rs"])));
    }

    #[test]
    fn covers_hits_multiple_entries_any_one_matching_is_enough() {
        assert!(covers_hits(
            "src/todo/src/main.rs src/bug-report",
            &v(&["src/bug-report/src/resolve.rs"])
        ));
    }

    #[test]
    fn covers_hits_no_match_returns_false() {
        assert!(!covers_hits(
            "src/todo",
            &v(&["src/bug-report/src/resolve.rs"])
        ));
    }

    #[test]
    fn covers_hits_empty_entry_list_matches_nothing() {
        assert!(!covers_hits("", &v(&["anything"])));
        assert!(!covers_hits("   ", &v(&["anything"])));
    }

    #[test]
    fn read_marker_finds_a_marker_within_the_first_five_lines() {
        let dir = scratch("within-five");
        fs::write(
            dir.join("t.sh"),
            "#!/usr/bin/env bash\n# MODE: DEV\n# COVERS: src/foo src/bar\necho hi\n",
        )
        .unwrap();
        assert_eq!(
            read_marker(&dir, "t.sh"),
            Some("src/foo src/bar".to_string())
        );
    }

    #[test]
    fn read_marker_ignores_a_marker_on_line_six_or_later() {
        let dir = scratch("line-six");
        fs::write(dir.join("t.sh"), "1\n2\n3\n4\n5\n# COVERS: src/foo\n").unwrap();
        assert_eq!(read_marker(&dir, "t.sh"), None);
    }

    #[test]
    fn read_marker_returns_none_for_a_file_with_no_marker() {
        let dir = scratch("no-marker");
        fs::write(dir.join("t.sh"), "#!/usr/bin/env bash\necho hi\n").unwrap();
        assert_eq!(read_marker(&dir, "t.sh"), None);
    }

    #[test]
    fn read_marker_returns_none_for_a_directory() {
        let dir = scratch("a-directory");
        fs::create_dir_all(dir.join("src/some-crate")).unwrap();
        assert_eq!(read_marker(&dir, "src/some-crate"), None);
    }

    #[test]
    fn read_marker_returns_none_for_a_missing_path() {
        let dir = scratch("missing");
        assert_eq!(read_marker(&dir, "does-not-exist.sh"), None);
    }

    #[test]
    fn select_keeps_undeclared_items_unconditionally() {
        let dir = scratch("select-undeclared");
        fs::write(dir.join("t.sh"), "echo hi\n").unwrap();
        let result = select(&dir, &v(&["t.sh"]), &v(&["totally/unrelated.md"]));
        assert_eq!(result.selected, v(&["t.sh"]));
        assert_eq!(result.excluded_count, 0);
    }

    #[test]
    fn select_excludes_a_declared_item_whose_marker_matches_nothing() {
        let dir = scratch("select-excludes");
        fs::write(dir.join("t.sh"), "# COVERS: src/foo\necho hi\n").unwrap();
        let result = select(&dir, &v(&["t.sh"]), &v(&["docs/x.md"]));
        assert!(result.selected.is_empty());
        assert_eq!(result.excluded_count, 1);
    }

    #[test]
    fn select_excludes_a_declared_item_with_an_empty_entry_list() {
        let dir = scratch("select-empty-entries");
        fs::write(dir.join("t.sh"), "# COVERS: \necho hi\n").unwrap();
        let result = select(&dir, &v(&["t.sh"]), &v(&["docs/x.md"]));
        assert!(result.selected.is_empty());
        assert_eq!(result.excluded_count, 1);
    }

    #[test]
    fn select_keeps_a_declared_item_whose_marker_hits() {
        let dir = scratch("select-hits");
        fs::write(dir.join("t.sh"), "# COVERS: src/foo\necho hi\n").unwrap();
        let result = select(&dir, &v(&["t.sh"]), &v(&["src/foo/main.rs"]));
        assert_eq!(result.selected, v(&["t.sh"]));
        assert_eq!(result.excluded_count, 0);
    }

    #[test]
    fn select_preserves_input_order() {
        let dir = scratch("select-order");
        fs::write(dir.join("b.sh"), "echo b\n").unwrap();
        fs::write(dir.join("a.sh"), "echo a\n").unwrap();
        let result = select(&dir, &v(&["b.sh", "a.sh"]), &v(&["x"]));
        assert_eq!(result.selected, v(&["b.sh", "a.sh"]));
    }

    #[test]
    fn select_a_crate_directory_item_is_always_undeclared() {
        let dir = scratch("select-crate-dir");
        fs::create_dir_all(dir.join("src/some-crate")).unwrap();
        let result = select(&dir, &v(&["src/some-crate"]), &v(&["totally/unrelated.md"]));
        assert_eq!(result.selected, v(&["src/some-crate"]));
        assert_eq!(result.excluded_count, 0);
    }
}
