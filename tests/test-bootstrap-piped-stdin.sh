#!/usr/bin/env bash
# MODE: DEV
# test-bootstrap-piped-stdin.sh — the documented `curl ... | bash` one-liner
# must deliver a REAL keystroke to the compiled installer's interactive
# prompts, not a leftover line of bootstrap.sh's own source text.
#
# Measured for real (B381): bash reading bootstrap.sh FROM STDIN (the shape
# `curl ... | bash` actually is -- no script file argument, so bash treats
# stdin as its own script source) still needs to read more of that same
# script after any earlier statement that reassigns fd 0. A bare `exec <
# /dev/tty` placed before the final handoff starves that reading and bash
# then tries to parse the next REAL keystroke as more shell source. The fix
# is putting the `< /dev/tty` redirect directly on the exec statement that
# hands off to the compiled installer -- the last thing bash ever does, so
# it never needs its own script again after it runs.
#
# This needs a REAL pty: the bug is specifically about what a process gets
# when it opens /dev/tty, which has no meaning against a plain pipe. `script`
# allocates one. The keystroke must arrive SEPARATELY IN TIME from
# bootstrap.sh's own bytes -- concatenating them defeats the point, since a
# real `curl | bash` delivers the whole script before a person can type
# anything -- so the timed instructions here are the test.
set -euo pipefail
export LC_ALL=C

tests_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$tests_dir/.." && pwd)"
# shellcheck source=planning/tests/lib-test.sh
source "$repo_root/planning/tests/lib-test.sh"
t_begin

if ! command -v script >/dev/null 2>&1; then
    t_skip 'test-bootstrap-piped-stdin: no `script` on PATH to allocate a pty'
fi

# util-linux's script takes `-c COMMAND OUTFILE`; BSD/macOS's takes
# `OUTFILE COMMAND...` with no -c at all. Probed once, since --version is
# itself the distinguishing feature (BSD script has no such flag).
# PORTABILITY(pipefail-grep-q): a case match on a captured string, not
# `cmd | grep -q`, which under pipefail can report the upstream command's
# SIGPIPE death instead of grep's own answer once -q stops reading early.
is_util_linux=0
case "$(script --version 2>&1)" in
    *[Uu]til-linux*) is_util_linux=1 ;;
esac

work="$(mktemp -d "${TMPDIR:-/tmp}/bootstrap-piped-stdin.XXXXXX")"
trap 'rm -rf "$work"' EXIT

# A stub release tarball: its "installer" reads exactly one line and reports
# it verbatim, so the assertion is purely about what byte sequence a real
# keystroke arrives as -- not about the real binary or a network fetch.
payload_dir="$work/payload"
mkdir -p "$payload_dir"
stub="$payload_dir/installer"
{
    printf '#!/usr/bin/env bash\n'
    printf 'read -r answer\n'
    printf 'printf '\''GOT:%%s\\n'\'' "$answer" > "$RESULT_LOG"\n'
} > "$stub"
chmod +x "$stub"
tar -czf "$work/release.tar.gz" -C "$payload_dir" installer

release_url="file://$work/release.tar.gz"
if command -v cygpath >/dev/null 2>&1; then
    release_url="file:///$(cygpath -m "$work")/release.tar.gz"
fi

result_log="$work/result.log"
typescript="$work/typescript.log"

run_piped() {
    rm -f "$result_log"
    # The delayed keystroke goes in on script's OWN stdin (relayed into the
    # pty); the pipe reading bootstrap.sh is entirely internal to the
    # command script runs, so it never touches those relayed bytes.
    local piped_cmd="RESULT_LOG='$result_log' AI_SKILLS_NO_SPLASH=1 AI_SKILLS_RELEASE_URL='$release_url' bash -c \"cat '$repo_root/installer/bootstrap.sh' | bash\""
    if [ "$is_util_linux" -eq 1 ]; then
        ( sleep 1; printf 'hello\n' ) | script -qec "$piped_cmd" "$typescript" >/dev/null 2>&1 || true
    else
        ( sleep 1; printf 'hello\n' ) | script -q "$typescript" bash -c "$piped_cmd" >/dev/null 2>&1 || true
    fi
}

run_piped
t_assert_eq 'a real keystroke reaches the compiled installer, not a line of bootstrap.sh itself' \
    "$(cat "$result_log" 2>/dev/null || printf '(nothing written)')" 'GOT:hello'

t_end
