// MODE: DEV
// PACKAGE: PROD
//! Resolve the running host to one of the five target triples this
//! repository ships binaries for (B94's Windows_NT/MINGW*/MSYS*/CYGWIN*
//! handling included).

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

#[derive(Debug, Clone)]
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
/// strings rather than calling `uname` itself so the match logic is
/// testable without a subprocess.
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
        // Windows_NT. Matched by prefix so this refuses no host those
        // report.
        _ if windows_like && arch_is_x86_64 => Ok(Target::X86_64PcWindowsMsvc),
        _ => Err(UnsupportedHost {
            os: os.to_string(),
            arch: arch.to_string(),
        }),
    }
}

/// Retries a possibly-flaky lookup (real use: spawning `uname`) up to three
/// times with a short backoff, returning the first success or an empty
/// string once every attempt has failed. Generic over the lookup itself so
/// the retry logic is unit-testable against a deterministically-flaky fake,
/// rather than depending on a real subprocess actually failing to prove it
/// works (B383: `Command::new("uname").output()` intermittently failed to
/// spawn under the parallel load of `cargo test`'s own thread pool on a
/// loaded macOS CI runner -- roughly 1 run in 7 -- and the old code mapped
/// that failure straight to an empty string, which `resolve` then reported
/// as `UnsupportedHost` on a host that plainly was supported).
fn retry_field_lookup(mut attempt: impl FnMut() -> Option<String>) -> String {
    for i in 0..3 {
        if let Some(value) = attempt() {
            return value;
        }
        if i < 2 {
            std::thread::sleep(std::time::Duration::from_millis(20 * (i + 1) as u64));
        }
    }
    String::new()
}

fn uname(flag: &str) -> String {
    let flag = flag.to_string();
    retry_field_lookup(move || {
        std::process::Command::new("uname")
            .arg(&flag)
            .output()
            .ok()
            .filter(|o| o.status.success())
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    })
}

/// Resolve the actual running host via `uname`-equivalent values from `std`.
/// There is no `uname(2)` in `std`, so this shells out to `uname` on
/// Unix-like hosts (present on every target `resolve` accepts except
/// cmd/PowerShell, which report their platform through `std::env::consts`
/// instead).
///
/// The host does not change mid-process, so a SUCCESSFUL answer is cached
/// permanently rather than re-spawning `uname` (twice) on every call -- a
/// caller like the installer's own test suite, where dozens of tests each
/// call this once, used to mean dozens of processes spawned concurrently
/// under `cargo test`'s thread pool. That contention is the other half of
/// B383, alongside `uname`'s own retry: fewer processes racing to spawn at
/// once means the retry has far less to recover from in the first place.
///
/// A FAILED resolution is never cached: caching it would let one transient
/// spawn failure (the exact thing the retry above exists to absorb) turn
/// into a permanent one for the rest of the process's life, on a host that
/// is plainly supported. Concurrent callers racing the very first
/// resolution may each spawn `uname` once more before the cache is warm --
/// harmless, since they all compute the same answer from the same host.
pub fn current() -> Result<Target, UnsupportedHost> {
    static CURRENT: std::sync::OnceLock<Target> = std::sync::OnceLock::new();
    if let Some(target) = CURRENT.get() {
        return Ok(*target);
    }
    let target = resolve_current()?;
    Ok(*CURRENT.get_or_init(|| target))
}

fn resolve_current() -> Result<Target, UnsupportedHost> {
    if cfg!(windows) {
        // std::env::consts::ARCH is the compiled target's arch, which is fine
        // here: this binary itself only ever ships for x86_64-pc-windows-msvc.
        return resolve("Windows_NT", std::env::consts::ARCH);
    }
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

    // B383: `current()`'s own transient-spawn-failure recovery is exercised
    // against a fake, deterministically-flaky lookup rather than a real
    // `uname` -- forcing the real subprocess to fail on demand is not
    // practical, and doing so would make the FIX's own test just as
    // dependent on real-world timing as the bug it closes was.

    #[test]
    fn retry_field_lookup_recovers_from_transient_failures() {
        let mut calls = 0;
        let value = retry_field_lookup(|| {
            calls += 1;
            if calls < 3 {
                None
            } else {
                Some("Darwin".to_string())
            }
        });
        assert_eq!(value, "Darwin");
        assert_eq!(calls, 3);
    }

    #[test]
    fn retry_field_lookup_succeeds_immediately_when_the_first_attempt_does() {
        let mut calls = 0;
        let value = retry_field_lookup(|| {
            calls += 1;
            Some("Linux".to_string())
        });
        assert_eq!(value, "Linux");
        assert_eq!(calls, 1);
    }

    #[test]
    fn retry_field_lookup_gives_up_after_three_attempts_and_returns_empty() {
        let mut calls = 0;
        let value = retry_field_lookup(|| {
            calls += 1;
            None
        });
        assert_eq!(value, "");
        assert_eq!(calls, 3);
    }
}
