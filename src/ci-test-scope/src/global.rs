// MODE: DEV
// PACKAGE: PROD
//! The global-input check: any changed path that can alter what ANY test
//! exercises forces `scope=full`. First matching path in the CHANGE SET's
//! own order wins (not alphabetical, not pattern order), matching bash's
//! own `while read` loop over the diff exactly.

/// `src/ci-test-scope/*` is the self-protection extension this goal adds:
/// once this crate exists, a change to its own source must be exercised in
/// full, exactly like a change to `.github/*` already is -- otherwise a bug
/// in the compiled selector could validate itself via its own narrowed,
/// unproven logic.
pub fn find_global_hit(changed: &[String]) -> Option<String> {
    for path in changed {
        if path.is_empty() {
            continue;
        }
        let is_exact_hit = matches!(
            path.as_str(),
            "Cargo.toml"
                | "Cargo.lock"
                | "rust-toolchain.toml"
                | "flake.nix"
                | "flake.lock"
                | "run-tests.sh"
                | "planning/tests/lib-test.sh"
        );
        if is_exact_hit || path.starts_with(".github/") || path.starts_with("src/ci-test-scope/") {
            return Some(path.clone());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(paths: &[&str]) -> Vec<String> {
        paths.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn exact_manifest_names_hit() {
        for name in [
            "Cargo.toml",
            "Cargo.lock",
            "rust-toolchain.toml",
            "flake.nix",
            "flake.lock",
        ] {
            assert_eq!(find_global_hit(&v(&[name])), Some(name.to_string()));
        }
    }

    #[test]
    fn run_tests_sh_hits() {
        assert_eq!(
            find_global_hit(&v(&["run-tests.sh"])),
            Some("run-tests.sh".to_string())
        );
    }

    #[test]
    fn lib_test_sh_hits() {
        assert_eq!(
            find_global_hit(&v(&["planning/tests/lib-test.sh"])),
            Some("planning/tests/lib-test.sh".to_string())
        );
    }

    #[test]
    fn a_manifest_shaped_name_in_a_subdirectory_does_not_hit() {
        assert_eq!(find_global_hit(&v(&["src/foo/Cargo.toml"])), None);
    }

    #[test]
    fn a_run_tests_shaped_name_in_a_subdirectory_does_not_hit() {
        assert_eq!(find_global_hit(&v(&["scripts/run-tests.sh"])), None);
    }

    #[test]
    fn dot_github_prefix_hits() {
        assert_eq!(
            find_global_hit(&v(&["src/rjq/src/main.rs", ".github/ci-test-scope.sh"])),
            Some(".github/ci-test-scope.sh".to_string())
        );
    }

    #[test]
    fn self_protection_new_case_hits() {
        assert_eq!(
            find_global_hit(&v(&["src/ci-test-scope/src/main.rs"])),
            Some("src/ci-test-scope/src/main.rs".to_string())
        );
    }

    #[test]
    fn an_ordinary_crate_under_src_does_not_hit() {
        assert_eq!(find_global_hit(&v(&["src/rjq/src/main.rs"])), None);
    }

    #[test]
    fn first_matching_path_in_change_set_order_wins() {
        assert_eq!(
            find_global_hit(&v(&["README.md", "run-tests.sh", "Cargo.toml"])),
            Some("run-tests.sh".to_string())
        );
    }

    #[test]
    fn no_hit_returns_none() {
        assert_eq!(find_global_hit(&v(&["README.md", "docs/x.md"])), None);
    }

    #[test]
    fn blank_paths_are_skipped() {
        assert_eq!(
            find_global_hit(&v(&["", "Cargo.toml"])),
            Some("Cargo.toml".to_string())
        );
    }
}
