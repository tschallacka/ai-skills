// MODE: DEV
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

fn unique_dir(tag: &str) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let mut dir = std::env::temp_dir();
    dir.push(format!(
        "generate-profile-content-flow-{tag}-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join(".agents/profiles")).unwrap();
    fs::create_dir_all(dir.join("planning/scripts")).unwrap();
    dir
}

/// Placed at `<root>/planning/scripts/role-context`, the exact path
/// `role_context_binary()` requires (see its own doc comment for why).
fn write_fake_role_context(dir: &Path, payload: &str) -> PathBuf {
    let script = dir.join("planning/scripts/role-context");
    let body = format!(
        "#!/bin/sh\ncat <<'EOF'\n# role-context fixture (Fixture) - page 1/1\n{payload}\nEOF\n"
    );
    fs::write(&script, body).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&script).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&script, perms).unwrap();
    }
    script
}

fn write_profile(
    dir: &Path,
    persona: &str,
    name: &str,
    description: &str,
    instructions: &str,
) -> PathBuf {
    let path = dir.join(".agents/profiles").join(format!("{persona}.json"));
    let json = serde_json::json!({
        "name": name,
        "description": description,
        "instructions": instructions,
    });
    fs::write(&path, serde_json::to_string_pretty(&json).unwrap()).unwrap();
    path
}

#[test]
fn check_then_write_then_check_again_via_the_real_binary() {
    let dir = unique_dir("e2e");
    write_fake_role_context(
        &dir,
        "# Role context: fixture (Fixture)\n\n# Voice (fixture): Be direct.\n\n===== ROLES.md =====\ninitial content\n",
    );
    let path = write_profile(
        &dir,
        "fixture",
        "fixture",
        "A hand-authored blurb.",
        "stale",
    );
    let exe = env!("CARGO_BIN_EXE_generate-profile-content");

    let check = Command::new(exe)
        .args(["fixture", "--check"])
        .current_dir(&dir)
        .output()
        .expect("run generate-profile-content --check");
    assert!(
        !check.status.success(),
        "expected --check to report drift on a stale profile"
    );

    let write = Command::new(exe)
        .args(["fixture", "--write"])
        .current_dir(&dir)
        .output()
        .expect("run generate-profile-content --write");
    assert!(
        write.status.success(),
        "write failed: {}",
        String::from_utf8_lossy(&write.stderr)
    );

    let recheck = Command::new(exe)
        .args(["fixture", "--check"])
        .current_dir(&dir)
        .output()
        .expect("run generate-profile-content --check (recheck)");
    assert!(
        recheck.status.success(),
        "expected --check to be clean after --write: {}",
        String::from_utf8_lossy(&recheck.stdout)
    );

    let spec: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(spec["name"], "fixture");
    assert_eq!(spec["description"], "A hand-authored blurb.");
    assert!(spec["instructions"]
        .as_str()
        .unwrap()
        .contains("initial content"));
}

#[test]
fn missing_persona_argument_is_refused() {
    let dir = unique_dir("missing-arg");
    let exe = env!("CARGO_BIN_EXE_generate-profile-content");
    let output = Command::new(exe)
        .arg("--check")
        .current_dir(&dir)
        .output()
        .expect("run generate-profile-content");
    assert!(!output.status.success());
}

#[test]
fn check_and_write_together_is_refused() {
    let dir = unique_dir("both-flags");
    write_profile(&dir, "fixture", "fixture", "d", "i");
    let exe = env!("CARGO_BIN_EXE_generate-profile-content");
    let output = Command::new(exe)
        .args(["fixture", "--check", "--write"])
        .current_dir(&dir)
        .output()
        .expect("run generate-profile-content");
    assert!(!output.status.success());
}
