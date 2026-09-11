// MODE: DEV
// PACKAGE: PROD
//! Resolve the running host to one of the five target triples this repository
//! ships binaries for. Mirrors `normalize_platform()` in `install.sh` (B94's
//! Windows_NT/MINGW*/MSYS*/CYGWIN* handling included) so the two stay in
//! agreement while both exist.

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    X86_64UnknownLinuxMusl,
    Aarch64UnknownLinuxMusl,
    X86_64AppleDarwin,
    Aarch64AppleDarwin,
    X86_64PcWindowsMsvc,
}

impl Target {
    pub const ALL: [Target; 5] = [
        Target::X86_64UnknownLinuxMusl,
        Target::Aarch64UnknownLinuxMusl,
        Target::X86_64AppleDarwin,
        Target::Aarch64AppleDarwin,
        Target::X86_64PcWindowsMsvc,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            Target::X86_64UnknownLinuxMusl => "x86_64-unknown-linux-musl",
            Target::Aarch64UnknownLinuxMusl => "aarch64-unknown-linux-musl",
            Target::X86_64AppleDarwin => "x86_64-apple-darwin",
            Target::Aarch64AppleDarwin => "aarch64-apple-darwin",
            Target::X86_64PcWindowsMsvc => "x86_64-pc-windows-msvc",
        }
    }

    pub fn is_windows(&self) -> bool {
        matches!(self, Target::X86_64PcWindowsMsvc)
    }

    /// Parse a target triple string back into a `Target` (round-trips `as_str`).
    pub fn parse(s: &str) -> Option<Target> {
        Target::ALL.into_iter().find(|t| t.as_str() == s)
    }
}

impl fmt::Display for Target {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug)]
pub struct UnsupportedHost {
    pub os: String,
    pub arch: String,
}

impl fmt::Display for UnsupportedHost {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unsupported host: {} {}", self.os, self.arch)
    }
}

impl std::error::Error for UnsupportedHost {}

/// Resolve `(uname -s, uname -m)` to a shipped target triple. Takes the two
/// strings rather than calling `uname` itself so the match logic is testable
/// without a subprocess, the same reason `install.sh` accepts
/// `PLAN_OVERVIEW_TEST_OS`/`_ARCH` overrides.
pub fn resolve(os: &str, arch: &str) -> Result<Target, UnsupportedHost> {
    let arch_is_x86_64 = matches!(arch, "x86_64" | "amd64" | "AMD64");
    let arch_is_aarch64 = matches!(arch, "aarch64" | "arm64");

    let windows_like = os == "Windows_NT"
        || os.starts_with("MINGW")
        || os.starts_with("MSYS")
        || os.starts_with("CYGWIN");

    match () {
        _ if os == "Linux" && arch_is_x86_64 => Ok(Target::X86_64UnknownLinuxMusl),
        _ if os == "Linux" && arch_is_aarch64 => Ok(Target::Aarch64UnknownLinuxMusl),
        _ if os == "Darwin" && arch_is_x86_64 => Ok(Target::X86_64AppleDarwin),
        _ if os == "Darwin" && arch_is_aarch64 => Ok(Target::Aarch64AppleDarwin),
        // B94: Git Bash reports MINGW64_NT-..., MSYS2 reports MSYS_NT-...,
        // Cygwin reports CYGWIN_NT-...; only cmd/PowerShell report the bare
        // Windows_NT. Matched by prefix for the same reason install.sh's glob
        // case does: this must refuse no host those report.
        _ if windows_like && arch_is_x86_64 => Ok(Target::X86_64PcWindowsMsvc),
        _ => Err(UnsupportedHost {
            os: os.to_string(),
            arch: arch.to_string(),
        }),
    }
}

/// Resolve the actual running host via `uname`-equivalent values from `std`.
/// There is no `uname(2)` in `std`, so this shells out to `uname` on
/// Unix-like hosts (present on every target `resolve` accepts except
/// cmd/PowerShell, which report their platform through `std::env::consts`
/// instead).
pub fn current() -> Result<Target, UnsupportedHost> {
    if cfg!(windows) {
        // std::env::consts::ARCH is the compiled target's arch, which is fine
        // here: this binary itself only ever ships for x86_64-pc-windows-msvc.
        return resolve("Windows_NT", std::env::consts::ARCH);
    }
    let uname = |flag: &str| -> String {
        std::process::Command::new("uname")
            .arg(flag)
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
            .unwrap_or_default()
    };
    resolve(&uname("-s"), &uname("-m"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linux_x86_64() {
        assert_eq!(
            resolve("Linux", "x86_64").unwrap(),
            Target::X86_64UnknownLinuxMusl
        );
    }

    #[test]
    fn linux_aarch64_and_arm64_alias() {
        assert_eq!(
            resolve("Linux", "aarch64").unwrap(),
            Target::Aarch64UnknownLinuxMusl
        );
        assert_eq!(
            resolve("Linux", "arm64").unwrap(),
            Target::Aarch64UnknownLinuxMusl
        );
    }

    #[test]
    fn darwin_both_arches() {
        assert_eq!(
            resolve("Darwin", "x86_64").unwrap(),
            Target::X86_64AppleDarwin
        );
        assert_eq!(
            resolve("Darwin", "arm64").unwrap(),
            Target::Aarch64AppleDarwin
        );
    }

    #[test]
    fn windows_variants() {
        for os in [
            "Windows_NT",
            "MINGW64_NT-10.0",
            "MSYS_NT-10.0",
            "CYGWIN_NT-10.0",
        ] {
            assert_eq!(
                resolve(os, "x86_64").unwrap(),
                Target::X86_64PcWindowsMsvc,
                "{os}"
            );
        }
    }

    #[test]
    fn unsupported_host_is_refused_not_guessed() {
        assert!(resolve("Plan9", "x86_64").is_err());
        assert!(resolve("Linux", "riscv64").is_err());
    }

    #[test]
    fn as_str_round_trips_through_parse() {
        for t in Target::ALL {
            assert_eq!(Target::parse(t.as_str()), Some(t));
        }
        assert_eq!(Target::parse("bogus-triple"), None);
    }
}
