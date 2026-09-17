// MODE: DEV
// PACKAGE: PROD

//! Builds the three corpora the bash original's own tree.txt/scripts.txt/
//! markdown.txt equivalents represent, and the tree_has()/script_writes()
//! existence checks over them.
//!
//! Corpus 1 (tree): every regular file under repo_root, excluding any
//! directory named `.git` anywhere, and the exact top-level paths
//! benchmark/results, .plans, .claude (AR-125: never excludes the test
//! script's own path -- that exclusion belongs only to corpus 2).
//!
//! Corpus 2 (script-text): the concatenated text of every `.sh` file in
//! corpus 1, excluding only the real bash original's own source path
//! (AR-125).
//!
//! Corpus 3 (markdown-text): the concatenated text of every `.md` file in
//! corpus 1, with each of the four tracked documents' own path individually
//! excluded (AR-128) -- a distinct exclusion target from corpus 2's, since a
//! named artifact appears verbatim in its own originating document's raw
//! text and omitting this exclusion would make every genuinely-FAIL artifact
//! spuriously WARN.

use regex::Regex;
use std::path::{Path, PathBuf};

pub struct Corpora {
    /// Every regular file under repo_root (pruned), as repo-root-relative,
    /// forward-slash paths (matching how the bash original's own tree.txt
    /// stores absolute paths -- relative here is an implementation choice
    /// that preserves identical existence semantics).
    pub tree: Vec<String>,
    pub script_text: String,
    pub markdown_text: String,
}

fn relative_slash(repo_root: &Path, path: &Path) -> String {
    path.strip_prefix(repo_root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn walk_tree(repo_root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let prune_exact: [PathBuf; 3] = [
        repo_root.join("benchmark").join("results"),
        repo_root.join(".plans"),
        repo_root.join(".claude"),
    ];
    walk(repo_root, &prune_exact, &mut out);
    out
}

fn walk(dir: &Path, prune_exact: &[PathBuf; 3], out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_dir() {
            if entry.file_name() == ".git" || prune_exact.iter().any(|p| p == &path) {
                continue;
            }
            walk(&path, prune_exact, out);
        } else if file_type.is_file() {
            out.push(path);
        }
    }
}

pub fn build_corpora(repo_root: &Path, self_path: &Path, tracked_docs: &[&Path]) -> Corpora {
    let all_files = walk_tree(repo_root);
    let tree: Vec<String> = all_files
        .iter()
        .map(|p| relative_slash(repo_root, p))
        .collect();

    let script_text = all_files
        .iter()
        .filter(|p| p.extension().map(|e| e == "sh").unwrap_or(false))
        .filter(|p| p.as_path() != self_path)
        .filter_map(|p| std::fs::read_to_string(p).ok())
        .collect::<Vec<_>>()
        .join("");

    let markdown_text = all_files
        .iter()
        .filter(|p| p.extension().map(|e| e == "md").unwrap_or(false))
        .filter(|p| !tracked_docs.iter().any(|d| d == p))
        .filter_map(|p| std::fs::read_to_string(p).ok())
        .collect::<Vec<_>>()
        .join("");

    Corpora {
        tree,
        script_text,
        markdown_text,
    }
}

/// `name_regex`: escape `.`/`[` for a BRE-equivalent, and expand a literal
/// `*` into a path-safe (non-slash) wildcard.
fn name_regex(pattern: &str) -> String {
    let mut out = String::new();
    for c in pattern.chars() {
        match c {
            '.' | '[' => {
                out.push('\\');
                out.push(c);
            }
            '*' => out.push_str("[^/]*"),
            _ => out.push(c),
        }
    }
    out
}

/// AR-132: broadened for an asterisk-containing pattern OR one that begins
/// with a literal hyphen -- both use a suffix-anchored (any-prefix-then-name)
/// search within the last path segment; anything else needs an exact
/// basename match.
pub fn tree_has(tree: &[String], pattern: &str) -> bool {
    let escaped = name_regex(pattern);
    let re = if pattern.contains('*') || pattern.starts_with('-') {
        Regex::new(&format!("/[^/]*{escaped}$")).unwrap()
    } else {
        Regex::new(&format!("/{escaped}$")).unwrap()
    };
    tree.iter().any(|line| re.is_match(&format!("/{line}")))
}

/// A script "writes" a name if some line redirects to a path ending in it.
pub fn script_writes(script_text: &str, name: &str) -> bool {
    let escaped = name_regex(name);
    let re = Regex::new(&format!(r#">\s*"?[^"\s]*{escaped}"?(\s|$)"#)).unwrap();
    script_text.lines().any(|line| re.is_match(line))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_regex_escapes_dots_and_expands_asterisk() {
        assert_eq!(
            name_regex("validate-plan-*-lib.sh"),
            "validate-plan-[^/]*-lib\\.sh"
        );
    }

    #[test]
    fn tree_has_exact_basename_match() {
        let tree = vec!["planning/scripts/foo.sh".to_string()];
        assert!(tree_has(&tree, "foo.sh"));
        assert!(!tree_has(&tree, "bar.sh"));
    }

    #[test]
    fn tree_has_broadened_match_on_asterisk() {
        let tree = vec!["planning/scripts/validate-plan-goals-lib.sh".to_string()];
        assert!(tree_has(&tree, "validate-plan-*-lib.sh"));
    }

    #[test]
    fn tree_has_broadened_match_on_hyphen_leading_name() {
        let tree = vec!["planning/scripts/create-step-testing.sh".to_string()];
        assert!(tree_has(&tree, "-testing.sh"));
    }

    #[test]
    fn tree_has_repo_relative_not_basename_only() {
        let tree = vec![
            "brainstorm/SKILL.md".to_string(),
            "post-implementation-review/SKILL.md".to_string(),
        ];
        assert!(tree_has(&tree, "SKILL.md"));
    }

    #[test]
    fn script_writes_detects_a_redirect() {
        let text = "echo hi > out/foo.sh\n";
        assert!(script_writes(text, "foo.sh"));
        assert!(!script_writes(text, "bar.sh"));
    }
}
