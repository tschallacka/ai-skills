// MODE: DEV
// PACKAGE: PROD

//! Builds every crate under src/ for the host's own target triple into
//! bin/<triple>, stages the generated shell artifacts on build-if-missing
//! terms, and wires the pre-push git hook. Includes the self-hosting
//! bootstrap: the plan's own row for `setup-dev-env` closes that loop.

mod generated;
mod markers;
mod plan;
mod platform;
mod reexec;
mod stage;
mod triple;

use std::path::Path;

// The literal script file name, not the compiled binary's own bare name.
// Used in two messages (unknown-argument, no-house-target-covers); a third
// use of this name is the nix re-exec target, handled separately.
const PROGRAM: &str = "setup-dev-env.sh";

// The exact usage text this prints (em-dashes included) -- deliberately NOT
// including additional CI/release-triple context, which --help never
// prints either.
const USAGE: &str = "\
setup-dev-env.sh \u{2014} build the crates under src/ into a working local tree.

A fresh clone carries Rust source but no binaries: nothing machine-produced is
committed, and the skills look for compiled helpers that are not there.
run-tests.sh is a shim over the compiled run-tests and exits 69 without it, so
an unbuilt tree cannot run the suite at all. This builds each binary listed by
--list for THIS machine into one bin/<target triple> at the repository root,
which is where the skills look, so a local tree runs the same code a target
does.

It also builds the generated shell artifacts a clean checkout lacks \u{2014} the five
plan-*-lib.sh that planning/scripts/*.sh source, and planning/REVIEWER.md \u{2014} on
the same build-if-missing terms, so one run leaves a tree that works rather
than one whose compiled half works.

Usage:
  setup-dev-env.sh              # build everything for this host
  setup-dev-env.sh --list       # print what would be built, and where
  setup-dev-env.sh --check      # report what is present or missing; build nothing
  setup-dev-env.sh --help

";

#[derive(PartialEq, Eq)]
enum Mode {
    Build,
    List,
    Check,
}

fn usage(code: i32) -> ! {
    print!("{USAGE}");
    std::process::exit(code);
}

fn parse_args(args: &[String]) -> Mode {
    let mut mode = Mode::Build;
    for arg in args {
        match arg.as_str() {
            "--list" => mode = Mode::List,
            "--check" => mode = Mode::Check,
            "-h" | "--help" => usage(0),
            other => {
                eprintln!("{PROGRAM}: unknown argument: {other}");
                usage(64);
            }
        }
    }
    mode
}

fn is_present(path: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::metadata(path)
            .map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
    }
    #[cfg(not(unix))]
    {
        path.is_file()
    }
}

fn run() -> i32 {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mode = parse_args(&args);

    let repo_root = match triple::discover_repo_root(reexec::SELF_BINARY_NAME) {
        Ok(root) => root,
        Err(message) => {
            eprintln!("{PROGRAM}: {message}");
            return 69;
        }
    };

    let host_triple = match triple::host_triple() {
        Ok(t) => t,
        Err(_) => {
            eprintln!("{PROGRAM}: no house target covers this host; nothing to build here");
            return 69;
        }
    };
    let exe_suffix = if host_triple.ends_with("windows-msvc") {
        ".exe"
    } else {
        ""
    };

    if mode == Mode::List || mode == Mode::Check {
        println!("host target: {host_triple}\n");
        for row in plan::plan() {
            let (crate_name, binary) = row;
            let dest = format!("bin/{host_triple}/{binary}{exe_suffix}");
            if mode == Mode::Check {
                let state = if is_present(&repo_root.join(&dest)) {
                    "present"
                } else {
                    "MISSING"
                };
                println!("  {crate_name:<16} -> {dest:<52} {state}");
                if plan::stages_into_planning_scripts(&repo_root, binary) {
                    let sibling = format!("planning/scripts/{binary}{exe_suffix}");
                    let state = if is_present(&repo_root.join(&sibling)) {
                        "present"
                    } else {
                        "MISSING"
                    };
                    println!("  {crate_name:<16} -> {sibling:<52} {state}");
                }
            } else {
                println!("  {crate_name:<16} -> {dest}");
                if plan::stages_into_planning_scripts(&repo_root, binary) {
                    println!("  {crate_name:<16} -> planning/scripts/{binary}{exe_suffix}");
                }
            }
        }
        return 0;
    }

    // --list and --check return above this point, so they still cost no
    // nix, cargo, or git at all.
    if let Err(message) = plan::check_stray_src_dirs(&repo_root) {
        eprint!("{message}");
        return 70;
    }

    reexec::maybe_reexec(PROGRAM, &repo_root, &host_triple);
    // maybe_reexec never returns when a re-exec happens; reaching here
    // means we are already inside the nix shell (or a marker was set).

    let token = markers::new_token();
    if let Err(error) = markers::write_started(&repo_root, &token) {
        eprintln!("{PROGRAM}: could not write .setup-dev-env.started: {error}");
        return 70;
    }

    println!("setup-dev-env: building for {host_triple}\n");
    let outcome = stage::run(&repo_root, &host_triple, exe_suffix);

    generated::build_if_missing(&repo_root);
    generated::wire_pre_push_hook(&repo_root);

    println!("\n{} binary/binaries built.", outcome.built);
    if !outcome.failed.is_empty() {
        eprintln!("failed:{}", outcome.failed.join(" "));
        return 70;
    }

    // Reached only once every crate above built: the tree is complete, so
    // record this run's own token as finished.
    if let Err(error) = markers::write_finished(&repo_root, &token) {
        eprintln!("{PROGRAM}: could not write .setup-dev-env.finished: {error}");
        return 70;
    }

    println!(
        "\nThe helpers put this directory on PATH themselves when they load, so rjq and\n\
         plan-crypt resolve here with nothing further to do. For an interactive shell\n\
         that wants them too:\n\n\
         \x20\x20export PATH=\"{}/bin/{host_triple}:$PATH\"",
        repo_root.display()
    );

    0
}

fn main() {
    std::process::exit(run());
}
