// MODE: DEV
// PACKAGE: PROD

//! script_list: shells to real find+sort, matching discovery.rs's own
//! established goal-15 precedent for exact collation/exclusion-matching
//! fidelity, rather than reimplementing a directory walker in Rust.

use std::path::Path;
use std::process::{Command, Stdio};

/// Every *.sh file under repo_root except benchmark/results, .git, .plans,
/// .claude, and this crate's own wired script name -- LC_ALL=C sorted, with
/// the leading `./` stripped, matching bash's own `sed 's|^\./||'`.
pub fn script_list(repo_root: &Path) -> Vec<String> {
    let mut find = Command::new("find")
        .current_dir(repo_root)
        .args([
            ".",
            "-name",
            "*.sh",
            "-type",
            "f",
            "-not",
            "-path",
            "./benchmark/results/*",
            "-not",
            "-path",
            "./.git/*",
            "-not",
            "-path",
            "./.plans/*",
            "-not",
            "-path",
            "./.claude/*",
            "-not",
            "-name",
            "generate-portability.sh",
        ])
        .stdout(Stdio::piped())
        .spawn()
        .expect("failed to spawn find");
    let find_stdout = find.stdout.take().expect("find has no stdout");
    let sort_output = Command::new("sort")
        .env("LC_ALL", "C")
        .stdin(Stdio::from(find_stdout))
        .output()
        .expect("failed to run sort");
    let _ = find.wait();
    String::from_utf8_lossy(&sort_output.stdout)
        .lines()
        .filter_map(|line| line.strip_prefix("./").map(str::to_string))
        .collect()
}
