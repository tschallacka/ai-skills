// MODE: DEV
// PACKAGE: PROD

//! Nix-shell entry belongs to the compiled binary too, not just bash: the
//! wiring in setup-dev-env.sh is inserted before the script's own nix-shell
//! dance, so this crate is fully responsible for replicating
//! require_nix/the SETUP_DEV_ENV_IN_NIX/IN_NIX_SHELL detection and the
//! re-exec itself. Reexecs directly into this same compiled binary a second
//! time (now inside the nix shell) rather than bouncing back through bash
//! first.
//!
//! Uses pre-push-check's own reexec.rs as a starting point, not an exact
//! mirror: this crate's current_exe()-failure fallback is fully qualified
//! as `<repo_root>/bin/<triple>/setup-dev-env` via SELF_BINARY_NAME, unlike
//! pre-push-check's own bare-name, PATH-reliant fallback -- more robust
//! since it does not depend on this binary's own directory being on PATH.

use std::env;
use std::path::Path;
use std::process::Command;

#[cfg(unix)]
use std::os::unix::process::CommandExt;

pub const SELF_BINARY_NAME: &str = "setup-dev-env";

use crate::platform::which;

/// Prints the require_nix-style message and exits 69 if nix is required and
/// absent; execs into `nix develop` and never returns if a re-exec is
/// needed and nix is present; returns otherwise (already inside the flake,
/// or a marker is already set).
pub fn maybe_reexec(program: &str, repo_root: &Path, triple: &str) {
    if env::var_os("SETUP_DEV_ENV_IN_NIX").is_some() || env::var_os("IN_NIX_SHELL").is_some() {
        return;
    }
    // nix does not run natively on Windows, so there is no shell to enter and
    // requiring one would refuse every Windows host. The toolchain there is
    // the rustup one CI and a developer both install, pinned by
    // rust-toolchain.toml.
    if cfg!(windows) {
        return;
    }
    if !which("nix") {
        eprintln!(
            "{program}: nix is required and is not installed.\n\n\
             The crates are built by the toolchain flake.nix pins -- the newest stable rust\n\
             the locked nixpkgs offers, with the five house targets. Any other cargo produces\n\
             a different artifact from the one CI and a release ship, so this script will not\n\
             fall back to one.\n\n\
             Install nix, then re-run this script:\n\n\
             \x20\x20Determinate Nix (recommended; this is what the repository is developed on)\n\
             \x20\x20\x20\x20curl -fsSL https://install.determinate.systems/nix | sh -s -- install\n\n\
             \x20\x20Upstream multi-user install\n\
             \x20\x20\x20\x20sh <(curl -L https://nixos.org/nix/install) --daemon\n\n\
             Both need a new shell afterwards so the profile script is sourced. Verify with:\n\n\
             \x20\x20nix --version\n\n\
             If nix is installed but not on PATH, source its profile:\n\n\
             \x20\x20. /nix/var/nix/profiles/default/etc/profile.d/nix-daemon.sh"
        );
        std::process::exit(69);
    }
    // Fully qualified: repo_root/bin/<triple>/setup-dev-env, not the bare
    // SELF_BINARY_NAME -- so this fallback does not depend on this binary's
    // own directory being on PATH inside the nix develop --command env
    // invocation.
    let self_path = env::current_exe()
        .unwrap_or_else(|_| repo_root.join("bin").join(triple).join(SELF_BINARY_NAME));
    let args: Vec<String> = env::args().skip(1).collect();

    let mut command = Command::new("nix");
    command
        .arg("develop")
        .arg(repo_root)
        .arg("--command")
        .arg("env")
        .arg("SETUP_DEV_ENV_IN_NIX=1")
        .arg(&self_path)
        .args(&args);

    #[cfg(unix)]
    {
        let error = command.exec();
        eprintln!("{program}: could not exec nix develop: {error}");
        std::process::exit(1);
    }
    #[cfg(not(unix))]
    {
        let status = command.status().unwrap_or_else(|error| {
            eprintln!("{program}: could not run nix develop: {error}");
            std::process::exit(1);
        });
        std::process::exit(status.code().unwrap_or(1));
    }
}
