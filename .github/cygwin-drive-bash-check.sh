#!/usr/bin/env bash
# MODE: DEV
# cygwin-drive-bash-check.sh — T84a (goal 04-cygwin-ci-leg, W22): spawn a
# real Cygwin bash through the just-built interactive-shell wrapper and
# confirm an echoed command's output actually appears on screen, proving
# openpty/fork/exec/ioctl really work on Cygwin, not just that the binary
# links. Invoked as a real committed file, not generated inline, because
# GitHub Actions' own auto-generated shell:-bash wrapper script corrupted
# when run through Cygwin's bash (a literal embedded newline landed inside
# its own "set -eo pipefail" startup line) -- a real file with plain LF line
# endings sidesteps whatever that generation step does differently.
set -euo pipefail

release="$GITHUB_WORKSPACE/target/x86_64-pc-cygwin/release"
wrapper="$release/interactive-shell"
[ -x "$wrapper" ] || wrapper="$wrapper.exe"
input="$release/interactive-shell-input"
[ -x "$input" ] || input="$input.exe"

session="cygwin-ci-$$"
"$wrapper" --session "$session" --cols 80 --rows 24 --idle-timeout 15 -- bash \
    >"$RUNNER_TEMP/wrapper.jsonl" 2>"$RUNNER_TEMP/wrapper.log" &
wrapper_pid=$!
trap 'kill "$wrapper_pid" 2>/dev/null || true' EXIT

# Poll for the socket/discovery file session_socket() creates, rather than a
# fixed sleep: openpty/fork/exec plus Cygwin's own process startup has no
# fixed latency worth guessing at.
for _ in $(seq 1 50); do
    "$input" --session "$session" view >/dev/null 2>&1 && break
    sleep 0.2
done

"$input" --session "$session" text 'echo cygwin-conpty-check-12345'
"$input" --session "$session" key ENTER
sleep 1
screen="$("$input" --session "$session" view)"
echo "$screen"
case "$screen" in
    *cygwin-conpty-check-12345*) ;;
    *)
        echo "::error::expected echo output never appeared on screen"
        cat "$RUNNER_TEMP/wrapper.log" 2>/dev/null || true
        exit 1
        ;;
esac

"$input" --session "$session" shutdown >/dev/null 2>&1 || true
wait "$wrapper_pid" 2>/dev/null || true
echo "interactive-shell spawned and drove a real Cygwin bash"
