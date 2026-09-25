// MODE: DEV
// PACKAGE: PROD

//! Where a Windows host differs from a unix one, for the calls this crate
//! makes to tools and scripts.
//!
//! - `Command::new("bash")` is not resolved through PATH first on Windows:
//!   Rust searches the system directories before it, so the name finds
//!   `C:\Windows\System32\bash.exe`, the WSL launcher, which fails without a
//!   distro, ahead of the Git for Windows bash a caller put first on PATH.
//! - A file is only a program when it ends in `.exe`.
//! - A `.sh` file cannot be started; it has to be handed to bash.

use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Where `program` would run from on PATH, or None. Walks PATH itself and
/// applies the platform's executable suffix; `bash` skips the WSL launcher.
pub fn resolve(program: &str) -> Option<PathBuf> {
    let name = format!("{program}{}", env::consts::EXE_SUFFIX);
    let path = env::var_os("PATH")?;
    env::split_paths(&path)
        .map(|dir| dir.join(&name))
        .find(|candidate| candidate.is_file() && !(program == "bash" && is_wsl_launcher(candidate)))
}

pub fn which(program: &str) -> bool {
    resolve(program).is_some()
}

fn is_wsl_launcher(path: &Path) -> bool {
    let lowered = path
        .to_string_lossy()
        .to_ascii_lowercase()
        .replace('/', "\\");
    lowered.contains("\\windows\\system32\\") || lowered.contains("\\windowsapps\\")
}

/// The bash to run `-c` snippets and scripts with, resolved through PATH.
pub fn bash() -> Command {
    Command::new(resolve("bash").unwrap_or_else(|| PathBuf::from("bash")))
}

/// A command that runs the script at `path`. Unix starts the file itself
/// (its `#!` line and mode bits stay in charge); Windows cannot start a
/// script, so it goes through bash, given the path with forward slashes.
pub fn script_command(path: &Path) -> Command {
    #[cfg(windows)]
    {
        let mut command = bash();
        command.arg(path.to_string_lossy().replace('\\', "/"));
        command
    }
    #[cfg(not(windows))]
    {
        Command::new(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_wsl_launcher_is_recognised_by_its_directory() {
        assert!(is_wsl_launcher(Path::new(
            "C:\\Windows\\System32\\bash.exe"
        )));
        assert!(is_wsl_launcher(Path::new(
            "C:/Users/x/AppData/Local/Microsoft/WindowsApps/bash.exe"
        )));
        assert!(!is_wsl_launcher(Path::new(
            "C:\\Program Files\\Git\\bin\\bash.exe"
        )));
    }

    #[test]
    fn a_program_that_is_not_on_path_is_not_found() {
        assert!(!which("definitely-not-a-real-program-name"));
    }
}
