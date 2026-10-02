// MODE: DEV
//! Shared by several crates' integration tests through
//! `#[path = "../../../tests/rust-support/script_stub.rs"] mod script_stub;`
//! (the same way chat-client-rs reuses chat-server-rs's support module).
//!
//! Tests stand in for `gh`, `glab`, `role-context` and the like by writing a
//! bash script into a scratch directory and putting that directory on PATH.
//! A unix kernel runs such a file straight from its `#!` line. Windows does
//! not: `CreateProcess` will only start a real executable, and Rust's
//! `Command::new("gh")` looks for `gh.exe`, so the script answers
//! "%1 is not a valid Win32 application" or is simply never found.
//!
//! `install` puts an executable in front of the script on Windows: a
//! ten-line program, compiled once per test process, copied to `<name>.exe`
//! next to the script. When it runs it starts `bash <name>` with the same
//! arguments and passes stdio and the exit code through. On unix it does
//! nothing, because the script already runs.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::Command;

#[cfg(windows)]
const SHIM_SOURCE: &str = r#"
use std::path::PathBuf;
use std::process::{exit, Command};
fn bash() -> PathBuf {
    if let Some(path) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path) {
            let candidate = dir.join("bash.exe");
            let lowered = candidate.to_string_lossy().to_ascii_lowercase();
            let launcher = lowered.contains("\\windows\\system32\\") || lowered.contains("\\windowsapps\\");
            if candidate.is_file() && !launcher {
                return candidate;
            }
        }
    }
    PathBuf::from("bash")
}
fn main() {
    let exe = std::env::current_exe().expect("own path");
    let script = exe.with_extension("");
    let script = script.to_string_lossy().replace('\\', "/");
    let status = Command::new(bash())
        .arg(&script)
        .args(std::env::args_os().skip(1))
        .status()
        .unwrap_or_else(|error| {
            eprintln!("script stub: cannot run bash {script}: {error}");
            exit(127)
        });
    exit(status.code().unwrap_or(1));
}
"#;

/// The compiled shim, built once and reused by every stub in this process.
#[cfg(windows)]
fn shim() -> &'static Path {
    use std::sync::OnceLock;
    static SHIM: OnceLock<PathBuf> = OnceLock::new();
    SHIM.get_or_init(|| {
        let dir = std::env::temp_dir().join(format!("script-stub-shim-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("create the shim build directory");
        let source = dir.join("shim.rs");
        std::fs::write(&source, SHIM_SOURCE).expect("write the shim source");
        let out = dir.join("shim.exe");
        let status = Command::new("rustc")
            .args(["--edition", "2021", "-O", "-o"])
            .arg(&out)
            .arg(&source)
            .status()
            .expect("run rustc to build the script stub shim");
        assert!(status.success(), "building the script stub shim failed");
        out
    })
}

/// Makes the script at `dir/<name>` startable as `<name>` from a PATH that
/// contains `dir`. Call it after writing the script.
pub fn install(dir: &Path, name: &str) {
    #[cfg(windows)]
    {
        std::fs::copy(shim(), dir.join(format!("{name}.exe"))).expect("install the script stub");
    }
    #[cfg(not(windows))]
    {
        let _ = (dir, name);
    }
}

/// The `bash` a test should spawn. On unix that is just `bash`, resolved
/// through PATH. On Windows `Command::new("bash")` is NOT resolved through PATH
/// first: Rust searches the system directories before it, so it finds
/// `C:\Windows\System32\bash.exe` -- the WSL launcher, which fails without a
/// distro -- ahead of Git for Windows' bash. Walk PATH ourselves and skip the
/// launcher.
pub fn bash_program() -> PathBuf {
    #[cfg(windows)]
    {
        if let Some(path) = std::env::var_os("PATH") {
            for dir in std::env::split_paths(&path) {
                let candidate = dir.join("bash.exe");
                let lowered = candidate.to_string_lossy().to_ascii_lowercase();
                let is_wsl_launcher = lowered.contains("\\windows\\system32\\")
                    || lowered.contains("\\windowsapps\\");
                if candidate.is_file() && !is_wsl_launcher {
                    return candidate;
                }
            }
        }
    }
    PathBuf::from("bash")
}

/// A command that runs `script` under bash on every platform. Unix could
/// start the file directly; Windows cannot, and going through bash on both
/// keeps the two the same.
pub fn bash_script(script: &Path) -> Command {
    let mut command = Command::new(bash_program());
    command.arg(script.to_string_lossy().replace('\\', "/"));
    command
}

/// `dir` with `extra` in front of the current PATH, in the platform's own
/// separator.
pub fn path_with(extra: &Path) -> std::ffi::OsString {
    let mut parts: Vec<PathBuf> = vec![extra.to_path_buf()];
    parts.extend(std::env::split_paths(
        &std::env::var_os("PATH").unwrap_or_default(),
    ));
    std::env::join_paths(parts).expect("PATH entries contain no separator")
}
