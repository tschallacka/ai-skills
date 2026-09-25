// MODE: DEV
// PACKAGE: PROD

//! script_list: every shell script in the tree, in the order `find . -name
//! '*.sh' -type f | LC_ALL=C sort` would list them.
//!
//! This used to shell out to `find` and `sort`. On Windows those names resolve
//! to System32's own `find.exe` and `sort.exe` -- different programs that
//! answer "FIND: Parameter format not correct" -- because Rust searches the
//! system directories before PATH, so the catalogue silently came out empty of
//! every marker. Walking the tree here needs no external program and behaves
//! the same everywhere.

use std::fs;
use std::path::Path;

/// Directories `find` was told to leave out, relative to the repository root:
/// benchmark/results and testing-stories/runs (gitignored run output, the
/// latter holding whole copies of the repo that `run-story.sh` leaves
/// behind), .git, .plans and .claude. Nothing beneath any of them counts.
const EXCLUDED_DIRS: &[&str] = &[
    "benchmark/results",
    "testing-stories/runs",
    ".git",
    ".plans",
    ".claude",
];

/// This crate's own wired script name, excluded wherever it appears.
const EXCLUDED_NAME: &str = "generate-portability.sh";

/// Every `*.sh` regular file under repo_root except the exclusions above --
/// byte-order sorted on the whole forward-slash path (which is what
/// `LC_ALL=C sort` does to `./dir/file.sh` lines: `a.sh` sorts before `a/b.sh`
/// because `.` is below `/`, not component by component) with no leading `./`.
///
/// Symlinks are neither followed nor listed, matching `find -type f`.
pub fn script_list(repo_root: &Path) -> Vec<String> {
    let mut found = Vec::new();
    walk(repo_root, "", &mut found);
    found.sort();
    found
}

fn walk(dir: &Path, relative: &str, found: &mut Vec<String>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let path = if relative.is_empty() {
            name.clone()
        } else {
            format!("{relative}/{name}")
        };
        // file_type() reports the entry itself, not what a symlink points at.
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        if kind.is_dir() {
            if !EXCLUDED_DIRS.contains(&path.as_str()) {
                walk(&entry.path(), &path, found);
            }
        } else if kind.is_file() && name.ends_with(".sh") && name != EXCLUDED_NAME {
            found.push(path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tree(tag: &str, files: &[&str]) -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!(
            "generate-portability-discovery-{tag}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        for file in files {
            let path = root.join(file);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, "#!/usr/bin/env bash\n").unwrap();
        }
        root
    }

    #[test]
    fn lists_shell_scripts_sorted_by_whole_path_in_byte_order() {
        // `.` (0x2e) is below `/` (0x2f): a.sh before a/b.sh, and a-b.sh
        // (`-` is 0x2d) before both.
        let root = tree(
            "order",
            &["a/b.sh", "a.sh", "a-b.sh", "z.sh", "B.sh", "notes.md"],
        );
        assert_eq!(
            script_list(&root),
            ["B.sh", "a-b.sh", "a.sh", "a/b.sh", "z.sh"]
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn leaves_out_the_excluded_trees_and_this_crates_own_wrapper() {
        let root = tree(
            "excluded",
            &[
                "keep.sh",
                "planning/scripts/generate-portability.sh",
                "benchmark/results/run/x.sh",
                "benchmark/other/y.sh",
                "testing-stories/runs/1/z.sh",
                ".git/hooks/pre-push.sh",
                ".plans/p/a.sh",
                ".claude/hooks/b.sh",
            ],
        );
        assert_eq!(script_list(&root), ["benchmark/other/y.sh", "keep.sh"]);
        let _ = fs::remove_dir_all(&root);
    }

    #[cfg(unix)]
    #[test]
    fn a_symlink_is_neither_followed_nor_listed() {
        let root = tree("symlink", &["real/a.sh"]);
        std::os::unix::fs::symlink(root.join("real/a.sh"), root.join("link.sh")).unwrap();
        std::os::unix::fs::symlink(root.join("real"), root.join("linked-dir")).unwrap();
        assert_eq!(script_list(&root), ["real/a.sh"]);
        let _ = fs::remove_dir_all(&root);
    }
}
