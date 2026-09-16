// MODE: DEV
// PACKAGE: DEV
// Integration coverage that must run the real compiled binary as a
// subprocess, not the crate's own unit-test process: relocating the binary
// (and the cwd) OUTSIDE this repository is the only way to make BOTH of
// skill_root()'s ancestor-walk sources (current_exe() and current_dir())
// fail to find a planning/scripts directory, exercising AR-26's failure path
// end to end (exact exit code, exact stderr message) rather than only the
// pure resolution function `main.rs`'s unit tests already cover.
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn relocated_binary(tag: &str) -> PathBuf {
    let source = PathBuf::from(env!("CARGO_BIN_EXE_build-plan-libs"));
    let mut outside = env::temp_dir();
    outside.push(format!(
        "build-plan-libs-flow-{tag}-{}-{:?}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&outside).expect("create outside-repo scratch dir");
    let dest = outside.join(source.file_name().unwrap());
    fs::copy(&source, &dest).expect("copy the compiled binary outside the repo");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(&dest).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&dest, permissions).unwrap();
    }
    dest
}

#[test]
fn skill_root_failure_path_exits_69_with_the_exact_message() {
    let binary = relocated_binary("failure");
    let cwd = binary.parent().unwrap().to_path_buf();
    let output = Command::new(&binary)
        .current_dir(&cwd)
        .env_remove("PLANNING_SKILL_ROOT")
        .output()
        .expect("run the relocated binary");
    assert_eq!(output.status.code(), Some(69));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        stderr.trim_end(),
        "build-plan-libs.sh: could not locate the planning skill root"
    );
    fs::remove_dir_all(&cwd).ok();
}

#[test]
fn planning_skill_root_env_overrides_a_broken_ancestor_walk() {
    let binary = relocated_binary("success");
    let outside_cwd = binary.parent().unwrap().to_path_buf();

    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("crate is two levels under the repo root")
        .to_path_buf();
    assert!(repo_root.join("planning/scripts").is_dir());

    let output = Command::new(&binary)
        .current_dir(&outside_cwd)
        .env("PLANNING_SKILL_ROOT", &repo_root)
        .arg("--check")
        .output()
        .expect("run the relocated binary with PLANNING_SKILL_ROOT set");
    // Exit code is either 0 (up to date) or 1 (stale); both mean skill_root()
    // resolved and the real check ran -- 69 would mean resolution failed.
    assert_ne!(output.status.code(), Some(69));
    fs::remove_dir_all(&outside_cwd).ok();
}
