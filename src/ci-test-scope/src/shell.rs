// MODE: DEV
// PACKAGE: PROD
//! Which `bash` to run a shell script with.
//!
//! On unix that is `bash`, found through PATH. On Windows a bare
//! `Command::new("bash")` is NOT: Rust looks in the system directories before
//! PATH, so it finds `C:\Windows\System32\bash.exe` -- the WSL launcher, which
//! only prints "Windows Subsystem for Linux has no installed distributions" --
//! ahead of the Git for Windows bash the user actually has. Walk PATH
//! ourselves and skip the launcher.

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

/// A script path in the form bash itself reads. On Windows that is forward
/// slashes, which every bash accepts there, where a backslash path is one
/// escape character away from a different path. Elsewhere a backslash is a
/// legal file-name character and the path is left exactly as it is.
pub fn script_arg(path: &Path) -> String {
    let text = path.to_string_lossy().into_owned();
    if cfg!(windows) {
        text.replace('\\', "/")
    } else {
        text
    }
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

    #[test]
    fn a_script_path_is_handed_to_bash_in_the_form_bash_reads() {
        assert_eq!(
            script_arg(Path::new("/repo/run-tests.sh")),
            "/repo/run-tests.sh"
        );
        #[cfg(windows)]
        assert_eq!(
            script_arg(Path::new(r"D:\a\repo\run-tests.sh")),
            "D:/a/repo/run-tests.sh"
        );
    }
}
