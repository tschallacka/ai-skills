// MODE: DEV
//! register-command keeps the registry itself, with serde_json, and never runs
//! rjq. It must therefore list, add and remove entries on a PATH that holds no
//! rjq at all (B393): an agent shell often has only the system directories, and
//! the refusal sent that agent to a package manager for a tool it never used.

use std::fs;
use std::process::Command;

#[test]
fn list_succeeds_with_no_rjq_on_path() {
    let dir = std::env::temp_dir().join(format!(
        "register-command-without-rjq-{}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("commands.json"), "{}\n").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_register-command"))
        .env("PATH", "")
        .arg(&dir)
        .arg("--list")
        .output()
        .unwrap();

    let _ = fs::remove_dir_all(&dir);
    assert!(
        output.status.success(),
        "exit {:?}, stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
}
