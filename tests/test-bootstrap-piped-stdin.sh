#!/usr/bin/env bash
# MODE: DEV
# test-bootstrap-piped-stdin.sh — the documented `curl ... | sh` one-liner
# must deliver a REAL keystroke to the compiled installer's interactive
# prompts, not a leftover line of bootstrap.sh's own source text.
#
# Measured for real (B381): a shell reading bootstrap.sh FROM STDIN (the
# shape `curl ... | sh` actually is -- no script file argument, so the
# shell treats stdin as its own script source) still needs to read more of
# that same script after any earlier statement that reassigns fd 0. A bare
# `exec < /dev/tty` placed before the final handoff starves that reading
# and the shell then tries to parse the next REAL keystroke as more shell
# source. The fix is putting the `< /dev/tty` redirect directly on the exec
# statement that hands off to the compiled installer -- the last thing the
# shell ever does, so it never needs its own script again after it runs.
# bootstrap.sh itself is pure POSIX sh (`#!/usr/bin/env sh`); this test
# pipes it through the real `sh` on this machine's PATH (dash), the
# interpreter the documented one-liner actually invokes.
#
# This needs a REAL pty: the bug is specifically about what a process gets
# when it opens /dev/tty, which has no meaning against a plain pipe. `script`
# allocates one. The keystroke must arrive SEPARATELY IN TIME from
# bootstrap.sh's own bytes -- concatenating them defeats the point, since a
# real `curl | bash` delivers the whole script before a person can type
# anything -- so this test does not send it until the compiled installer
# itself is the process actually holding the tty (see `ready_marker`
# below), rather than guessing a delay long enough with a plain `sleep`.
# A fixed sleep raced bootstrap.sh's own runtime on a loaded CI runner: it
# genuinely failed this way (not a defect in bootstrap.sh -- `GOT:` empty
# meant the keystroke arrived too early, while bootstrap.sh's own `sh` was
# still the process reading the tty), which is exactly the class of flake a
# readiness signal, not a timeout, is supposed to make impossible.
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
# itself the distinguishing feature (BSD script has no such flag) --
# BSD/macOS script does not recognize `--version` and exits nonzero for it.
# Confirmed live (macOS CI): `lib-test.sh`'s ERR trap still fires and prints
# its own "command failed: script --version 2>&1" diagnostic for that
# (`set -e` does not actually abort here -- a command substitution used as
# a case's own subject is exempt, confirmed locally under both a current
# bash and this repo's own bash 3.2 floor), but the printed line is pure
# noise that looks like a real failure at a glance. `|| true` on the
# substitution makes the expected case explicit and silences it: a nonzero
# exit here means only "not util-linux", exactly what the case's own
# no-match default (`is_util_linux` staying 0) already means.
# PORTABILITY(pipefail-grep-q): a case match on a captured string, not
# `cmd | grep -q`, which under pipefail can report the upstream command's
# SIGPIPE death instead of grep's own answer once -q stops reading early.
is_util_linux=0
case "$(script --version 2>&1 || true)" in
    *[Uu]til-linux*) is_util_linux=1 ;;
esac

work="$(mktemp -d "${TMPDIR:-/tmp}/bootstrap-piped-stdin.XXXXXX")"
trap 'rm -rf "$work"' EXIT

# A stub release tarball: its "installer" reads exactly one line and reports
# it verbatim, so the assertion is purely about what byte sequence a real
# keystroke arrives as -- not about the real binary or a network fetch.
# `touch "$READY_MARKER"` is the stub's OWN first statement, so its very
# existence proves bootstrap.sh's handoff `exec ... < /dev/tty` already
# happened and THIS process (not bootstrap.sh's own `sh`, already replaced
# by that exec) now owns the tty -- the exact condition B381's bug was
# about. A keystroke sent any time after that is safe even if `read` itself
# hasn't executed yet: the kernel's own tty input queue buffers it either
# way, since a pty's input side does not require an active reader.
payload_dir="$work/payload"
mkdir -p "$payload_dir"
stub="$payload_dir/installer"
{
    printf '#!/usr/bin/env bash\n'
    printf 'touch "$READY_MARKER"\n'
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
ready_marker="$work/ready"
typescript="$work/typescript.log"

# Polls for the stub's own readiness signal instead of guessing how long
# bootstrap.sh takes to run -- 200 checks at 50ms is a 10s ceiling, an order
# of magnitude past anything bootstrap.sh (AI_SKILLS_NO_SPLASH=1, a local
# file:// tarball, no network) should ever take even on a loaded runner.
# Exits with a named failure on timeout rather than sending the keystroke
# anyway, which would just trade one flake for a more confusing one.
wait_for_stub_ready() {
    local i=0
    while [ ! -e "$ready_marker" ]; do
        i=$((i + 1))
        if [ "$i" -ge 200 ]; then
            printf 'test-bootstrap-piped-stdin: installer stub never signaled ready within 10s\n' >&2
            exit 1
        fi
        sleep 0.05
    done
}

run_piped() {
    rm -f "$result_log" "$ready_marker"
    # The keystroke goes in on script's OWN stdin (relayed into the pty);
    # the pipe reading bootstrap.sh is entirely internal to the command
    # script runs, so it never touches those relayed bytes.
    local piped_cmd="RESULT_LOG='$result_log' READY_MARKER='$ready_marker' AI_SKILLS_NO_SPLASH=1 AI_SKILLS_RELEASE_URL='$release_url' bash -c \"cat '$repo_root/installer/bootstrap.sh' | sh\""
    if [ "$is_util_linux" -eq 1 ]; then
        ( wait_for_stub_ready; printf 'hello\n' ) | script -qec "$piped_cmd" "$typescript" >/dev/null 2>&1 || true
    else
        ( wait_for_stub_ready; printf 'hello\n' ) | script -q "$typescript" bash -c "$piped_cmd" >/dev/null 2>&1 || true
    fi
}

# Retried rather than run once: measured (B386) to still occasionally come
# back empty on macOS specifically -- both its default-bash and bash-3.2 CI
# legs -- even with the readiness marker above already removing the
# guessed-delay race B381 was about. Investigated but not pinned down to an
# exact trigger: reproducing the SAME kind of freeze that reliably
# reproduced a DIFFERENT wall-clock race in B384 (SIGSTOP the relaying
# process past the point the keystroke should arrive, then SIGCONT) did NOT
# reproduce this one against this box's util-linux `script`, so this is not
# simply "script hasn't started relaying yet." The leading suspect is a
# documented macOS PTY kernel quirk instead: unread buffered data on a pty
# can be silently dropped around a process's exit/exec boundary, depending
# on whether S_CTTYREF (a controlling-terminal reference) was ever set
# (bugs.ruby-lang.org #20682, macOS 13.2) -- bootstrap.sh's own chain execs
# through several processes ending in an `exec ... < /dev/tty` handoff, any
# one of which is a candidate boundary, but which one (if any) is not
# confirmed without a real macOS machine to instrument. Retrying a handful
# of freshly spawned attempts is the same mitigation B384 used for its own
# (confirmed, different) race: it does not depend on understanding the
# exact mechanism, only on the drop being intermittent rather than
# deterministic, which the CI failure rate observed so far is consistent
# with -- roughly half of a handful of runs, never all of them.
for attempt in 1 2 3 4 5; do
    run_piped
    if [ "$(cat "$result_log" 2>/dev/null || printf '(nothing written)')" = 'GOT:hello' ]; then
        break
    fi
done
t_assert_eq 'a real keystroke reaches the compiled installer, not a line of bootstrap.sh itself' \
    "$(cat "$result_log" 2>/dev/null || printf '(nothing written)')" 'GOT:hello'

t_end
