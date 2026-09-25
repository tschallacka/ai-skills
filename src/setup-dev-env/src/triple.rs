// MODE: DEV
// PACKAGE: PROD

//! Host target-triple resolution: the Windows OS arm also accepts
//! `Windows_NT` (B94), not just `MINGW*`/`MSYS*`/`CYGWIN*`.

use std::env;
#[cfg(not(windows))]
use std::process::Command;

/// Pure branch logic, split out from the real `uname` call so every branch
/// (including both `Err` cases) is directly unit-testable without faking a
/// subprocess.
pub fn resolve_triple(os: &str, arch: &str) -> Result<String, String> {
    match os {
        "Linux" => match arch {
            "x86_64" | "amd64" => Ok("x86_64-unknown-linux-musl".to_string()),
            "aarch64" | "arm64" => Ok("aarch64-unknown-linux-musl".to_string()),
            _ => Err(format!("no house target covers Linux:{arch}")),
        },
        "Darwin" => match arch {
            "x86_64" => Ok("x86_64-apple-darwin".to_string()),
            "arm64" | "aarch64" => Ok("aarch64-apple-darwin".to_string()),
            _ => Err(format!("no house target covers Darwin:{arch}")),
        },
        // MINGW*/MSYS*/CYGWIN* covers a POSIX-ish shell on Windows;
        // Windows_NT covers a non-POSIX shell (cmd, PowerShell), which a
        // POSIX uname never reports, but no host the msvc binary actually
        // serves should be refused for reporting it (B94).
        _ if os.starts_with("MINGW")
            || os.starts_with("MSYS")
            || os.starts_with("CYGWIN")
            || os == "Windows_NT" =>
        {
            match arch {
                "x86_64" | "amd64" => Ok("x86_64-pc-windows-msvc".to_string()),
                _ => Err(format!("no house target covers {os}:{arch}")),
            }
        }
        _ => Err(format!("no house target covers {os}:{arch}")),
    }
}

/// Shells to real `uname -s`/`uname -m`, falling back to "unknown" if
/// either fails, and resolves the result via `resolve_triple`.
pub fn host_triple() -> Result<String, String> {
    // A Windows host is told by the compiler, not asked: `uname` is only there
    // when Git for Windows' usr/bin happens to be on PATH, and a plain cmd or
    // PowerShell session has none.
    #[cfg(windows)]
    {
        resolve_triple("Windows_NT", std::env::consts::ARCH)
    }
    #[cfg(not(windows))]
    {
        let os = uname_field("-s");
        let arch = uname_field("-m");
        resolve_triple(&os, &arch)
    }
}

#[cfg(not(windows))]
fn uname_field(flag: &str) -> String {
    Command::new("uname")
        .arg(flag)
        .output()
        .ok()
        .filter(|out| out.status.success())
        .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown".to_string())
}

/// PLANNING_SKILL_ROOT, exported unconditionally by
/// plan_exec_compiled_binary_if_present before every exec of this binary;
/// when absent or empty (a direct/standalone invocation outside the wired
/// exec path, such as a test), fall back to a location-anchored resolution
/// from the running binary's own path, walking up for the nearest ancestor
/// containing planning/scripts. Deliberately never uses git rev-parse: this
/// resolution needs no git for any mode, and a git-ancestry resolution has
/// a concrete failure mode in this repository's own nested tool copies
/// under benchmark/results/, where git rev-parse --show-toplevel resolves
/// to the OUTER repository root instead.
pub fn discover_repo_root(self_binary_name: &str) -> Result<std::path::PathBuf, String> {
    if let Ok(root) = env::var("PLANNING_SKILL_ROOT") {
        if !root.is_empty() {
            return Ok(std::path::PathBuf::from(root));
        }
    }
    let self_path =
        env::current_exe().unwrap_or_else(|_| std::path::PathBuf::from(self_binary_name));
    let mut dir = self_path
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    loop {
        if dir.join("planning/scripts").is_dir() {
            return Ok(dir);
        }
        let Some(parent) = dir.parent() else {
            return Err(format!(
                "could not locate the repository root from {}",
                self_path.display()
            ));
        };
        dir = parent.to_path_buf();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linux_x86_64_resolves() {
        assert_eq!(
            resolve_triple("Linux", "x86_64").unwrap(),
            "x86_64-unknown-linux-musl"
        );
    }

    #[test]
    fn linux_aarch64_resolves() {
        assert_eq!(
            resolve_triple("Linux", "aarch64").unwrap(),
            "aarch64-unknown-linux-musl"
        );
    }

    #[test]
    fn darwin_x86_64_resolves() {
        assert_eq!(
            resolve_triple("Darwin", "x86_64").unwrap(),
            "x86_64-apple-darwin"
        );
    }

    #[test]
    fn darwin_arm64_resolves() {
        assert_eq!(
            resolve_triple("Darwin", "arm64").unwrap(),
            "aarch64-apple-darwin"
        );
    }

    #[test]
    fn mingw_x86_64_resolves() {
        assert_eq!(
            resolve_triple("MINGW64_NT", "x86_64").unwrap(),
            "x86_64-pc-windows-msvc"
        );
    }

    #[test]
    fn windows_nt_reporting_host_resolves_b94_parity() {
        assert_eq!(
            resolve_triple("Windows_NT", "x86_64").unwrap(),
            "x86_64-pc-windows-msvc"
        );
    }

    #[test]
    fn unknown_os_is_err() {
        assert!(resolve_triple("Plan9", "x86_64").is_err());
    }

    #[test]
    fn unknown_arch_is_err() {
        assert!(resolve_triple("Linux", "riscv64").is_err());
    }
}
