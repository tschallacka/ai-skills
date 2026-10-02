// MODE: DEV
// PACKAGE: PROD
//! Which workspace crates a change set touches: the second path component of
//! any changed path whose first component is exactly `src`, matching bash's
//! own `awk -F/ '$1 == "src" && NF >= 2 && $2 != "" { print $2 }'`.

use std::collections::BTreeSet;

pub fn extract_changed_crates(changed: &[String]) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for path in changed {
        let mut parts = path.splitn(3, '/');
        let first = parts.next().unwrap_or("");
        let second = parts.next().unwrap_or("");
        if first == "src" && !second.is_empty() {
            out.insert(second.to_string());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(paths: &[&str]) -> Vec<String> {
        paths.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn a_plain_crate_path_is_extracted() {
        let out = extract_changed_crates(&v(&["src/rjq/src/main.rs"]));
        assert_eq!(out.into_iter().collect::<Vec<_>>(), vec!["rjq".to_string()]);
    }

    #[test]
    fn bare_src_with_no_second_component_is_excluded() {
        assert!(extract_changed_crates(&v(&["src"])).is_empty());
    }

    #[test]
    fn a_trailing_slash_with_no_crate_name_is_excluded() {
        assert!(extract_changed_crates(&v(&["src/"])).is_empty());
    }

    #[test]
    fn a_double_slash_is_excluded() {
        assert!(extract_changed_crates(&v(&["src//foo"])).is_empty());
    }

    #[test]
    fn a_non_src_path_is_excluded() {
        assert!(extract_changed_crates(&v(&["README.md", "docs/src/x"])).is_empty());
    }

    #[test]
    fn duplicates_across_files_in_the_same_crate_collapse() {
        let out = extract_changed_crates(&v(&["src/rjq/src/main.rs", "src/rjq/Cargo.toml"]));
        assert_eq!(out.into_iter().collect::<Vec<_>>(), vec!["rjq".to_string()]);
    }

    #[test]
    fn multiple_crates_are_sorted() {
        let out = extract_changed_crates(&v(&["src/zzz/a.rs", "src/aaa/b.rs"]));
        assert_eq!(
            out.into_iter().collect::<Vec<_>>(),
            vec!["aaa".to_string(), "zzz".to_string()]
        );
    }
}
