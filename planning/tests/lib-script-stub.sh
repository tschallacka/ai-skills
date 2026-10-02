#!/usr/bin/env bash
# MODE: DEV
# lib-script-stub.sh — make a bash-script stand-in runnable by a NATIVE program
# on Windows.
#
# A test that fakes `gh`, `glab` or the like writes a bash script into a scratch
# directory and puts that directory first on PATH. When the caller is bash that
# is enough, on every platform. When the caller is a native program -- a
# compiled ci-failures.exe running `gh` -- it is not on Windows: CreateProcess
# starts only real executables, and a Rust `Command::new("gh")` looks for
# `gh.exe`, so the script is never found (the real gh answers instead) or
# answers "%1 is not a valid Win32 application".
#
# The fix is the one tests/rust-support/script_stub.rs uses for the Rust tests:
# put a ten-line executable in front of the script. It is compiled once with
# rustc, copied to <name>.exe next to the script, and when it runs it starts
# `bash <name>` with the same arguments, passing stdio and the exit code
# through. On every other platform this library does nothing.
#
# Usage, after sourcing:
#   stub_prepare_shim "$scratch_dir"      # once, in the main shell (not a $(...))
#   ... write "$stub_bin/gh" ...
#   stub_install_exe "$stub_bin" gh

stub_on_windows() {
    case "$(uname -s 2>/dev/null)" in
        MINGW*|MSYS*|CYGWIN*) return 0 ;;
    esac
    return 1
}

# Builds the shim into <scratch_dir> and remembers where. A no-op off Windows.
# Call it directly, not inside $(...): the path is kept in a shell variable.
stub_prepare_shim() {
    stub_on_windows || return 0
    local dir="$1"
    [ -n "${STUB_SHIM_EXE:-}" ] && [ -f "$STUB_SHIM_EXE" ] && return 0
    command -v rustc >/dev/null 2>&1 || {
        echo "lib-script-stub: rustc is required to build the script stub shim on Windows" >&2
        return 1
    }
    mkdir -p "$dir"
    cat >"$dir/script-stub-shim.rs" <<'RUST'
use std::path::PathBuf;
use std::process::{exit, Command};

// Rust searches the system directories before PATH, so a bare "bash" is
// System32's WSL launcher; walk PATH and skip it.
fn bash() -> PathBuf {
    if let Some(path) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path) {
            let candidate = dir.join("bash.exe");
            let lowered = candidate.to_string_lossy().to_ascii_lowercase();
            let launcher =
                lowered.contains("\\windows\\system32\\") || lowered.contains("\\windowsapps\\");
            if candidate.is_file() && !launcher {
                return candidate;
            }
        }
    }
    PathBuf::from("bash")
}

fn main() {
    let exe = std::env::current_exe().expect("own path");
    let script = exe.with_extension("");
    let script = script.to_string_lossy().replace('\\', "/");
    let status = Command::new(bash())
        .arg(&script)
        .args(std::env::args_os().skip(1))
        .status()
        .unwrap_or_else(|error| {
            eprintln!("script stub: cannot run bash {script}: {error}");
            exit(127)
        });
    exit(status.code().unwrap_or(1));
}
RUST
    rustc --edition 2021 -O -o "$dir/script-stub-shim.exe" "$dir/script-stub-shim.rs" >&2 || {
        echo "lib-script-stub: building the script stub shim failed" >&2
        return 1
    }
    STUB_SHIM_EXE="$dir/script-stub-shim.exe"
}

# Makes the script at <dir>/<name> startable as <name> by a native program from
# a PATH containing <dir>. Write the script first.
stub_install_exe() {
    stub_on_windows || return 0
    local dir="$1" name="$2"
    [ -n "${STUB_SHIM_EXE:-}" ] || {
        echo "lib-script-stub: stub_prepare_shim was not called" >&2
        return 1
    }
    cp "$STUB_SHIM_EXE" "$dir/$name.exe"
}

# A stand-in for a real executable: a symlink where those work, a copy on
# Windows, where `ln -s` makes a shortcut file a native loader cannot open.
stub_link_or_copy() {
    if stub_on_windows; then
        cp "$1" "$2"
    else
        ln -s "$1" "$2"
    fi
}
