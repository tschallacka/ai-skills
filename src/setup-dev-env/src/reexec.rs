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

/// B402: the running binary carries its own crate list (`plan::plan()`)
/// compiled in from whatever source it was last built from. A pull that adds
/// a crate updates src/setup-dev-env/src/plan.rs on disk, but the process
/// already running from the OLD binary never re-reads it -- the build loop
/// below rebuilds and restages this very crate like any other row, replacing
/// bin/<triple>/setup-dev-env on disk, yet keeps enumerating its own
/// already-loaded, now-stale plan for every row after it. The new crate is
/// built only on the NEXT invocation, which is the gap this closes: rebuild
/// and restage the self-hosting row FIRST, on its own, then re-exec the
/// freshly staged binary with the original arguments before the real build
/// loop ever runs, so that loop always enumerates the plan this exact source
/// tree declares. Guarded by one env var so the re-exec'd process does not
/// repeat this step forever; a failure to rebuild is reported but does not
/// block the run, which simply continues against the binary already in
/// memory -- exactly today's behavior, not a new way to fail.
pub fn maybe_reexec_after_self_rebuild(repo_root: &Path, triple: &str, exe_suffix: &str) {
    if env::var_os("SETUP_DEV_ENV_SELF_REFRESHED").is_some() {
        return;
    }
    // No src/setup-dev-env crate in this tree (a test fixture built around a
    // handful of dummy crates, say) means there is no self to refresh and
    // nothing freshly staged to re-exec into; carry on with the process
    // already running, exactly as if this step did not exist.
    if !repo_root
        .join("src")
        .join(SELF_BINARY_NAME)
        .join("Cargo.toml")
        .is_file()
    {
        return;
    }
    println!("setup-dev-env: refreshing itself first\n");
    if matches!(
        crate::stage::build_and_stage_one(
            repo_root,
            triple,
            exe_suffix,
            SELF_BINARY_NAME,
            SELF_BINARY_NAME,
        ),
        crate::stage::StepOutcome::Failed
    ) {
        eprintln!(
            "setup-dev-env.sh: could not rebuild itself first; continuing with the binary \
             already running (it may not know about a crate added since it was last built)"
        );
        return;
    }

    let self_path = repo_root
        .join("bin")
        .join(triple)
        .join(format!("{SELF_BINARY_NAME}{exe_suffix}"));
    let args: Vec<String> = env::args().skip(1).collect();

    let mut command = Command::new(&self_path);
    command.args(&args).env("SETUP_DEV_ENV_SELF_REFRESHED", "1");

    #[cfg(unix)]
    {
        let error = command.exec();
        eprintln!("setup-dev-env.sh: could not re-exec the freshly built binary: {error}");
        std::process::exit(1);
    }
    #[cfg(not(unix))]
    {
        let status = command.status().unwrap_or_else(|error| {
            eprintln!("setup-dev-env.sh: could not run the freshly built binary: {error}");
            std::process::exit(1);
        });
        std::process::exit(status.code().unwrap_or(1));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn scratch(tag: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("setup-dev-env-reexec-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    // Both tests below only ever reach a guard that returns before the
    // self-rebuild (a real cargo build) or the re-exec (which would replace
    // this very test process) could run at all -- if either guard failed to
    // fire, the un-mocked `Command::exec`/`cargo build` behind it would hang
    // or panic the test, not silently pass.

    #[test]
    fn already_refreshed_returns_without_touching_a_crate_that_would_otherwise_build() {
        let dir = scratch("already-refreshed");
        fs::create_dir_all(dir.join("src/setup-dev-env")).unwrap();
        fs::write(
            dir.join("src/setup-dev-env/Cargo.toml"),
            "[package]\nname=\"x\"\n",
        )
        .unwrap();
        // SAFETY: single-threaded test process; no other thread reads env vars here.
        unsafe {
            env::set_var("SETUP_DEV_ENV_SELF_REFRESHED", "1");
        }
        maybe_reexec_after_self_rebuild(&dir, "x86_64-unknown-linux-musl", "");
        unsafe {
            env::remove_var("SETUP_DEV_ENV_SELF_REFRESHED");
        }
        assert!(!dir
            .join("bin/x86_64-unknown-linux-musl/setup-dev-env")
            .exists());
    }

    #[test]
    fn no_self_crate_in_this_tree_returns_without_attempting_a_build() {
        let dir = scratch("no-self-crate");
        // Deliberately no src/setup-dev-env anywhere under dir.
        maybe_reexec_after_self_rebuild(&dir, "x86_64-unknown-linux-musl", "");
        assert!(!dir
            .join("bin/x86_64-unknown-linux-musl/setup-dev-env")
            .exists());
    }
}
