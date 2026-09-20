// MODE: DEV
// PACKAGE: PROD

//! Suite/crate discovery and work-item filtering, mirroring run-tests.sh's
//! own find+sort shelling exactly -- including its own deliberate collation
//! choices (no locale override for shell-test discovery, LC_ALL=C pinned
//! only for crate discovery, per B203) -- rather than reimplementing find or
//! sort as Rust logic. The one exception is Windows, which has no find or
//! sort that mean what these arguments say: there the same two listings are
//! read straight from the directories and sorted by bytes.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
#[cfg(not(windows))]
use std::process::{Command, Stdio};

pub const SUITES: [&str; 6] = [
    "tests",
    "planning/tests",
    "editor-gate-plugin/tests",
    "tui-hint-plugin/tests",
    "agent-identity-plugin/tests",
    ".github/tests",
];

pub const BENCHMARK_SUITE: &str = "benchmark/planning/tests";

#[cfg(not(windows))]
fn find_piped_to_sort(find: Command, sort: Command) -> Vec<String> {
    let mut find = find;
    let Ok(mut find_child) = find.stdout(Stdio::piped()).spawn() else {
        return Vec::new();
    };
    let Some(find_stdout) = find_child.stdout.take() else {
        let _ = find_child.wait();
        return Vec::new();
    };
    let mut sort = sort;
    let output = sort.stdin(Stdio::from(find_stdout)).output();
    let _ = find_child.wait();
    match output {
        Ok(out) => String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter(|l| !l.is_empty())
            .map(str::to_string)
            .collect(),
        Err(_) => Vec::new(),
    }
}

/// The names directly inside `dir`, byte-sorted, for which `keep` is true.
///
/// Windows has no `find` or `sort` worth shelling out to: `Command::new`
/// resolves those names to System32's `find.exe` and `sort.exe`, which are
/// unrelated tools that reject GNU arguments, so discovery is done here.
#[cfg_attr(not(windows), allow(dead_code))]
fn sorted_children(dir: &Path, keep: impl Fn(&Path, &str) -> bool) -> Vec<String> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            keep(&entry.path(), &name).then_some(name)
        })
        .collect();
    names.sort();
    names
}

/// Discover test scripts in one suite directory. Returned with forward
/// slashes, like everything else this module hands to the work-item list.
#[cfg_attr(not(windows), allow(dead_code))]
fn discover_native(dir: &Path) -> Vec<String> {
    sorted_children(dir, |path, name| {
        name.starts_with("test-") && name.ends_with(".sh") && path.is_file()
    })
    .into_iter()
    .map(|name| format!("{}/{name}", dir.to_string_lossy().replace('\\', "/")))
    .collect()
}

/// Every workspace crate directory (repo-relative, e.g. "src/foo"),
/// byte-sorted, which is the C-locale order the unix path pins (B203).
#[cfg_attr(not(windows), allow(dead_code))]
fn discover_crates_native(repo_root: &Path) -> Vec<String> {
    sorted_children(&repo_root.join("src"), |path, _| {
        path.join("Cargo.toml").is_file()
    })
    .into_iter()
    .map(|name| format!("src/{name}"))
    .collect()
}

#[cfg(windows)]
pub fn discover(dir: &Path) -> Vec<String> {
    discover_native(dir)
}

#[cfg(windows)]
pub fn discover_crates(repo_root: &Path) -> Vec<String> {
    discover_crates_native(repo_root)
}

/// Discover test scripts in one suite directory, sorted with NO locale
/// override -- the original's own `discover()` relies on ambient collation,
/// stable per host, not pinned to C the way crate discovery is.
#[cfg(not(windows))]
pub fn discover(dir: &Path) -> Vec<String> {
    let find = {
        let mut c = Command::new("find");
        c.arg(dir).args([
            "-maxdepth",
            "1",
            "-type",
            "f",
            "-name",
            "test-*.sh",
            "-print",
        ]);
        c
    };
    find_piped_to_sort(find, Command::new("sort"))
}

/// Discover every workspace crate directory (repo-relative, e.g. "src/foo"),
/// sorted under LC_ALL=C (B203: an ambient UTF-8 locale reorders
/// src/ai-text-editor-mcp vs src/ai-text-editor/ relative to a C locale).
#[cfg(not(windows))]
pub fn discover_crates(repo_root: &Path) -> Vec<String> {
    let find = {
        let mut c = Command::new("find");
        c.arg(repo_root.join("src")).args([
            "-mindepth",
            "2",
            "-maxdepth",
            "2",
            "-type",
            "f",
            "-name",
            "Cargo.toml",
            "-print",
        ]);
        c
    };
    let mut sort = Command::new("sort");
    sort.env("LC_ALL", "C");
    let repo_prefix = format!("{}/", repo_root.display());
    find_piped_to_sort(find, sort)
        .into_iter()
        .map(|line| {
            let stripped = line.strip_prefix(&repo_prefix).unwrap_or(&line);
            stripped
                .strip_suffix("/Cargo.toml")
                .unwrap_or(stripped)
                .to_string()
        })
        .collect()
}

/// The one list of "what counts as a test": every discovered shell test
/// (repo-relative path), in suite order, then every discovered crate
/// directory, in that fixed two-block concatenation.
pub fn build_work_items(repo_root: &Path) -> Vec<String> {
    let mut items = Vec::new();
    for suite in SUITES.iter().chain(std::iter::once(&BENCHMARK_SUITE)) {
        for path in discover(&repo_root.join(suite)) {
            let relative = Path::new(&path)
                .strip_prefix(repo_root)
                // Work items are always written with forward slashes, the
                // form --select-file lines and the shard logic compare.
                .map(|p| p.to_string_lossy().replace('\\', "/"))
                .unwrap_or(path);
            items.push(relative);
        }
    }
    items.extend(discover_crates(repo_root));
    items
}

/// --select-file: keep only items named exactly (one per line) in `path`.
pub fn apply_select_file(items: Vec<String>, path: &Path) -> std::io::Result<Vec<String>> {
    let content = fs::read_to_string(path)?;
    let wanted: HashSet<&str> = content.lines().collect();
    Ok(items
        .into_iter()
        .filter(|item| wanted.contains(item.as_str()))
        .collect())
}

/// --shard: deterministic position-modulo-total over the already-filtered list.
pub fn apply_shard(items: Vec<String>, index: usize, total: usize) -> Vec<String> {
    items
        .into_iter()
        .enumerate()
        .filter(|(i, _)| i % total == index)
        .map(|(_, item)| item)
        .collect()
}

/// Splits the final work-item list back into (shell test absolute paths,
/// crate directories), matching the original's own `src/*` case statement.
pub fn split_tests_and_crates(repo_root: &Path, items: &[String]) -> (Vec<PathBuf>, Vec<String>) {
    let mut tests = Vec::new();
    let mut crates = Vec::new();
    for item in items {
        if item.starts_with("src/") {
            crates.push(item.clone());
        } else {
            tests.push(repo_root.join(item));
        }
    }
    (tests, crates)
}

/// Validates and parses a `--shard I/N` argument exactly as the bash
/// original's own case-pattern validation does: both non-negative integers,
/// total at least 1, index strictly less than total.
pub fn parse_shard(program: &str, spec: &str) -> Result<(usize, usize), String> {
    let malformed =
        || format!("{program}: --shard wants I/N, both non-negative integers, got {spec}");
    let parts: Vec<&str> = spec.splitn(2, '/').collect();
    if parts.len() != 2 {
        return Err(malformed());
    }
    let valid_digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    if !valid_digits(parts[0]) || !valid_digits(parts[1]) {
        return Err(malformed());
    }
    let index: usize = parts[0].parse().map_err(|_| malformed())?;
    let total: usize = parts[1].parse().map_err(|_| malformed())?;
    if total == 0 {
        return Err(format!("{program}: --shard total must be at least 1"));
    }
    if index >= total {
        return Err(format!(
            "{program}: --shard index {index} is out of range for {total} shards"
        ));
    }
    Ok((index, total))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch_tree(tag: &str) -> PathBuf {
        let root =
            std::env::temp_dir().join(format!("run-tests-native-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("tests")).unwrap();
        for name in ["test-b.sh", "test-a.sh", "helper.sh", "test-c.txt"] {
            fs::write(root.join("tests").join(name), "").unwrap();
        }
        fs::create_dir_all(root.join("tests/test-dir.sh")).unwrap();
        for krate in ["zed", "alpha", "alpha-mcp"] {
            fs::create_dir_all(root.join("src").join(krate)).unwrap();
            fs::write(root.join("src").join(krate).join("Cargo.toml"), "").unwrap();
        }
        fs::create_dir_all(root.join("src/not-a-crate")).unwrap();
        root
    }

    #[test]
    fn native_discovery_lists_only_sorted_test_scripts_with_forward_slashes() {
        let root = scratch_tree("scripts");
        let found = discover_native(&root.join("tests"));
        let names: Vec<&str> = found
            .iter()
            .map(|path| path.rsplit('/').next().unwrap())
            .collect();
        assert_eq!(names, vec!["test-a.sh", "test-b.sh"]);
        assert!(found.iter().all(|path| !path.contains('\\')), "{found:?}");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn native_crate_discovery_is_byte_sorted_and_needs_a_manifest() {
        let root = scratch_tree("crates");
        // The C-locale order B203 pins: "alpha" before "alpha-mcp" before "zed".
        assert_eq!(
            discover_crates_native(&root),
            vec!["src/alpha", "src/alpha-mcp", "src/zed"]
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn select_file_keeps_only_listed_items() {
        let dir = std::env::temp_dir().join(format!("run-tests-select-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let file = dir.join("select.txt");
        fs::write(&file, "a\nc\n").unwrap();
        let items = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let kept = apply_select_file(items, &file).unwrap();
        assert_eq!(kept, vec!["a".to_string(), "c".to_string()]);
    }

    #[test]
    fn select_file_naming_absent_items_has_no_effect() {
        let dir =
            std::env::temp_dir().join(format!("run-tests-select-absent-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let file = dir.join("select.txt");
        fs::write(&file, "zzz\n").unwrap();
        let items = vec!["a".to_string(), "b".to_string()];
        let kept = apply_select_file(items, &file).unwrap();
        assert!(kept.is_empty());
    }

    #[test]
    fn shard_keeps_deterministic_positions() {
        let items: Vec<String> = (0..6).map(|i| i.to_string()).collect();
        let shard0 = apply_shard(items.clone(), 0, 2);
        let shard1 = apply_shard(items, 1, 2);
        assert_eq!(shard0, vec!["0", "2", "4"]);
        assert_eq!(shard1, vec!["1", "3", "5"]);
    }

    #[test]
    fn split_tests_and_crates_uses_src_prefix() {
        let root = Path::new("/repo");
        let items = vec![
            "planning/tests/test-foo.sh".to_string(),
            "src/foo".to_string(),
        ];
        let (tests, crates) = split_tests_and_crates(root, &items);
        assert_eq!(
            tests,
            vec![PathBuf::from("/repo/planning/tests/test-foo.sh")]
        );
        assert_eq!(crates, vec!["src/foo".to_string()]);
    }

    #[test]
    fn shard_rejects_malformed_spec() {
        assert!(parse_shard("run-tests.sh", "abc").is_err());
        assert!(parse_shard("run-tests.sh", "1/2/3").is_err());
        assert!(parse_shard("run-tests.sh", "").is_err());
        assert!(parse_shard("run-tests.sh", "1/").is_err());
    }

    #[test]
    fn shard_rejects_zero_total() {
        let err = parse_shard("run-tests.sh", "0/0").unwrap_err();
        assert!(err.contains("--shard total must be at least 1"));
    }

    #[test]
    fn shard_rejects_out_of_range_index() {
        let err = parse_shard("run-tests.sh", "2/2").unwrap_err();
        assert!(err.contains("--shard index 2 is out of range for 2 shards"));
    }

    #[test]
    fn shard_accepts_valid_spec() {
        assert_eq!(parse_shard("run-tests.sh", "0/2").unwrap(), (0, 2));
    }
}
