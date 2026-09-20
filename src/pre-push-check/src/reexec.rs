// MODE: DEV
// PACKAGE: PROD

//! Git runs hooks with the caller's environment, which may provide a
//! different cargo than the repository's pinned toolchain. Mirrors
//! pre-push-check.sh's own header: re-enter the flake once, using the same
//! two marker variables the bash original checks, before any gate that shells
//! to cargo, shellcheck, or bash32 runs.

use std::env;
use std::process::Command;

#[cfg(unix)]
use std::os::unix::process::CommandExt;

// The bash original derives its own name from `${0##*/}`, which in every
// real invocation (the git hook, a direct call) is "pre-push-check.sh" --
// the literal file name, not the compiled binary's own bare name.
const PROGRAM: &str = "pre-push-check.sh";
// The compiled binary's own file name, used only to resolve a fallback exec
// target if this process cannot read its own path -- must NOT carry the
// ".sh" suffix, since no such file exists for the compiled binary.
const SELF_BINARY_NAME: &str = "pre-push-check";

use crate::platform::which;

/// Exits 69 if nix is required and absent; execs into `nix develop` and never
/// returns if a re-exec is needed and nix is present; returns otherwise
/// (already inside the flake, or a marker is already set).
pub fn maybe_reexec(repo_root: &std::path::Path) {
    if env::var_os("AI_SKILLS_PREPUSH_IN_NIX").is_some() || env::var_os("IN_NIX_SHELL").is_some() {
        return;
    }
    // nix does not run natively on Windows, so there is no flake to enter and
    // requiring one would refuse every Windows host; the toolchain there is
    // the rustup one rust-toolchain.toml pins.
    if cfg!(windows) {
        return;
    }
    if !which("nix") {
        eprintln!("{PROGRAM}: nix develop .#default is required for Rust pre-push checks");
        std::process::exit(69);
    }
    let self_path = env::current_exe().unwrap_or_else(|_| SELF_BINARY_NAME.into());
    let args: Vec<String> = env::args().skip(1).collect();

    let mut command = Command::new("nix");
    command
        .arg("develop")
        .arg(repo_root)
        .arg("--command")
        .arg("env")
        .arg("AI_SKILLS_PREPUSH_IN_NIX=1")
        .arg(&self_path)
        .args(&args);

    #[cfg(unix)]
    {
        let error = command.exec();
        eprintln!("{PROGRAM}: could not exec nix develop: {error}");
        std::process::exit(1);
    }
    #[cfg(not(unix))]
    {
        let status = command.status().unwrap_or_else(|error| {
            eprintln!("{PROGRAM}: could not run nix develop: {error}");
            std::process::exit(1);
        });
        std::process::exit(status.code().unwrap_or(1));
    }
}
