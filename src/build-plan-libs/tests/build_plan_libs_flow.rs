// MODE: DEV
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

/// A just-copied binary's own exec can race a fork() on another test thread
/// that still holds the copy's write handle open (ETXTBSY); retry rather
/// than fail the test on that transient error.
fn run_relocated(command: &mut Command) -> std::process::Output {
    let mut retries_left = 4;
    loop {
        match command.output() {
            Ok(output) => return output,
            Err(err) if err.raw_os_error() == Some(26) && retries_left > 0 => {
                retries_left -= 1;
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            Err(err) => panic!("run the relocated binary: {err}"),
        }
    }
}

#[test]
fn skill_root_failure_path_exits_69_with_the_exact_message() {
    let binary = relocated_binary("failure");
    let cwd = binary.parent().unwrap().to_path_buf();
    let mut command = Command::new(&binary);
    command.current_dir(&cwd).env_remove("PLANNING_SKILL_ROOT");
    let output = run_relocated(&mut command);
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

    // A synthetic root, not the real repository: skill_root() only checks
    // that <root>/planning/scripts is a directory (main.rs's own
    // skill_root_from), so a bare empty one is a sufficient env override.
    // The real repo's own planning/scripts is NOT a safe substitute here --
    // CI's native job runs this suite against a sparse checkout that omits
    // planning/scripts entirely (only planning/rust-migration.tsv and
    // planning/binaries.tsv are fetched), so relying on it made this test
    // fail identically on every native leg, not on a broken assumption
    // about the binary under test.
    let synthetic_root = env::temp_dir().join(format!(
        "build-plan-libs-flow-root-{}-{:?}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(synthetic_root.join("planning/scripts")).expect("create synthetic root");

    let mut command = Command::new(&binary);
    command
        .current_dir(&outside_cwd)
        .env("PLANNING_SKILL_ROOT", &synthetic_root)
        .arg("--check");
    let output = run_relocated(&mut command);
    // Exit code is either 0/1 (a real check ran) or 65 (render_library found
    // no lib/<group> directories under this bare synthetic root -- also
    // "resolution succeeded"); 69 would mean resolution failed, which is the
    // only outcome this test rules out.
    assert_ne!(output.status.code(), Some(69));
    fs::remove_dir_all(&outside_cwd).ok();
    fs::remove_dir_all(&synthetic_root).ok();
}
