// MODE: DEV
// PACKAGE: PROD
//! POSIX bash `case`-pattern matching (`*`, `?`, `[...]`) without extglob.
//! Deliberately does NOT implement `|` alternation inside one unquoted
//! pattern word (a real bash case-pattern feature) -- no row in this
//! repository's own real coupling.tsv uses it, and it is out of scope.

/// Matches `path` against a single bash case-pattern `glob`.
pub fn matches(path: &str, glob: &str) -> bool {
    let text: Vec<char> = path.chars().collect();
    let pattern: Vec<char> = glob.chars().collect();
    glob_match(&pattern, &text)
}

fn glob_match(pattern: &[char], text: &[char]) -> bool {
    let (pn, tn) = (pattern.len(), text.len());
    let mut pi = 0usize;
    let mut ti = 0usize;
    let mut star_pi: Option<usize> = None;
    let mut star_ti = 0usize;

    while ti < tn {
        if pi < pn {
            match pattern[pi] {
                '*' => {
                    star_pi = Some(pi);
                    star_ti = ti;
                    pi += 1;
                    continue;
                }
                '?' => {
                    pi += 1;
                    ti += 1;
                    continue;
                }
                '[' => {
                    if let Some((matched, next_pi)) = match_class(pattern, pi, text[ti]) {
                        if matched {
                            pi = next_pi;
                            ti += 1;
                            continue;
                        }
                        // Fall through to backtrack: the class parsed but did
                        // not match this character.
                    } else if pattern[pi] == text[ti] {
                        // Malformed class (no closing `]`): `[` is literal.
                        pi += 1;
                        ti += 1;
                        continue;
                    }
                }
                c if c == text[ti] => {
                    pi += 1;
                    ti += 1;
                    continue;
                }
                _ => {}
            }
        }
        if let Some(sp) = star_pi {
            pi = sp + 1;
            star_ti += 1;
            ti = star_ti;
        } else {
            return false;
        }
    }
    while pi < pn && pattern[pi] == '*' {
        pi += 1;
    }
    pi == pn
}

/// `pattern[start] == '['`. Returns `Some((matched, index_after_closing_bracket))`
/// on a well-formed class, or `None` if there is no closing `]` (bash then
/// treats the `[` as a literal character).
fn match_class(pattern: &[char], start: usize, ch: char) -> Option<(bool, usize)> {
    let mut i = start + 1;
    let mut negate = false;
    if i < pattern.len() && (pattern[i] == '!' || pattern[i] == '^') {
        negate = true;
        i += 1;
    }
    let class_start = i;
    // A literal `]` is allowed as the first character of the class.
    if i < pattern.len() && pattern[i] == ']' {
        i += 1;
    }
    while i < pattern.len() && pattern[i] != ']' {
        i += 1;
    }
    if i >= pattern.len() {
        return None;
    }
    let end = i;
    let mut matched = false;
    let mut j = class_start;
    while j < end {
        if j + 2 < end && pattern[j + 1] == '-' {
            let (lo, hi) = (pattern[j], pattern[j + 2]);
            if ch >= lo && ch <= hi {
                matched = true;
            }
            j += 3;
        } else {
            if pattern[j] == ch {
                matched = true;
            }
            j += 1;
        }
    }
    if negate {
        matched = !matched;
    }
    Some((matched, end + 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_literal_glob_with_no_wildcard_matches_only_itself() {
        assert!(matches("coupling.tsv", "coupling.tsv"));
        assert!(!matches("coupling.tsv.bak", "coupling.tsv"));
    }

    #[test]
    fn star_matches_any_run_including_across_slashes() {
        assert!(matches("todo/schema.v2.json", "todo/schema.*.json"));
        assert!(matches("bug-report/schema.v2.json", "*/schema.*.json"));
        assert!(matches("a/b/c/schema.x.json", "*/schema.*.json"));
    }

    #[test]
    fn question_mark_matches_exactly_one_character() {
        assert!(matches("a.txt", "?.txt"));
        assert!(!matches("ab.txt", "?.txt"));
        assert!(!matches(".txt", "?.txt"));
    }

    #[test]
    fn bracket_character_class_matches_one_of_its_members() {
        assert!(matches("file1.rs", "file[0-9].rs"));
        assert!(!matches("filea.rs", "file[0-9].rs"));
        assert!(matches("filea.rs", "file[a-z].rs"));
    }

    #[test]
    fn a_path_that_matches_nothing_returns_false() {
        assert!(!matches("unrelated/path.rs", "coupling.tsv"));
    }

    #[test]
    fn every_literal_glob_in_the_real_coupling_tsv_matches_itself() {
        // Real coupling.tsv rows are read directly rather than retyped, so
        // this test tracks the file's own real content instead of drifting.
        let repo_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|p| p.parent())
            .expect("src/blast-radius is two levels below the repo root");
        let coupling = repo_root.join("coupling.tsv");
        let contents = std::fs::read_to_string(&coupling)
            .unwrap_or_else(|e| panic!("reading {}: {e}", coupling.display()));
        for line in contents.lines() {
            let glob = line.split('\t').next().unwrap_or("");
            if glob.is_empty() || glob.starts_with('#') {
                continue;
            }
            // A literal glob (no wildcard characters) must match itself
            // exactly; a wildcard glob is exercised by the synthetic cases
            // above rather than re-derived here.
            if !glob.contains(['*', '?', '[']) {
                assert!(
                    matches(glob, glob),
                    "literal glob {glob:?} did not match itself"
                );
            }
        }
    }
}
