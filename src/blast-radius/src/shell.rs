// MODE: DEV
// PACKAGE: PROD
//! Which `bash` to run a coupling check with.
//!
//! On unix that is `bash`, found through PATH. On Windows a bare
//! `Command::new("bash")` is NOT: Rust looks in the system directories before
//! PATH, so it finds `C:\Windows\System32\bash.exe` -- the WSL launcher, which
//! only prints "Windows Subsystem for Linux has no installed distributions" --
//! ahead of the Git for Windows bash the user actually has. Every check then
//! "failed" with that message (UTF-16, so NUL-riddled) as its reason. Walk
//! PATH ourselves and skip the launcher.

use std::path::{Path, PathBuf};

pub fn bash() -> PathBuf {
    #[cfg(windows)]
    {
        if let Some(path) = std::env::var_os("PATH") {
            for dir in std::env::split_paths(&path) {
                let candidate = dir.join("bash.exe");
                if candidate.is_file() && !is_wsl_launcher(&candidate) {
                    return candidate;
                }
            }
        }
    }
    PathBuf::from("bash")
}

/// True for the WSL launcher stubs Windows ships as `bash.exe`: the one in
/// System32 and the app-execution alias under WindowsApps.
#[cfg_attr(not(windows), allow(dead_code))]
fn is_wsl_launcher(path: &Path) -> bool {
    let lowered = path.to_string_lossy().to_ascii_lowercase();
    lowered.contains("\\windows\\system32\\") || lowered.contains("\\windowsapps\\")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_wsl_launchers_are_recognised_and_git_bash_is_not() {
        assert!(is_wsl_launcher(Path::new(r"C:\Windows\System32\bash.exe")));
        assert!(is_wsl_launcher(Path::new(r"C:\Windows\system32\BASH.EXE")));
        assert!(is_wsl_launcher(Path::new(
            r"C:\Users\me\AppData\Local\Microsoft\WindowsApps\bash.exe"
        )));
        assert!(!is_wsl_launcher(Path::new(
            r"C:\Program Files\Git\bin\bash.exe"
        )));
    }
}
