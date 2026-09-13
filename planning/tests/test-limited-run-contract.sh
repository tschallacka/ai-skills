#!/usr/bin/env bash
# MODE: DEV
# test-limited-run-contract — resource-limited-testing's wrapper contract:
# the macOS memory cap degrades to a warning, not a refusal, when memlimit is
# absent.
#
# Usage: test-limited-run-contract.sh
#
# The macOS assertions run on any host: `uname` and `memlimit` are stubbed on
# PATH, because a gate that is only exercised on a mac is a gate nobody runs.
set -euo pipefail
# shellcheck source=planning/tests/lib-test.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib-test.sh"
t_begin

export LC_ALL=C

repo_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
wrapper="$repo_dir/resource-limited-testing/scripts/limited-run.sh"
temporary_root="$(mktemp -d "${TMPDIR:-/tmp}/limited-run-contract.XXXXXX")"
trap 'rm -rf "$temporary_root"' EXIT

note_fail() { printf 'limited-run: %s\n' "$1" >&2; t_record "$1"; }

# ─────────────────────────────────────────────────────────────────────────────
# Stub PATH: uname reports whatever $STUB_UNAME_S says, and the limiters record
# their full argv instead of running. nice is outermost in the macOS chain, so
# its log line holds the whole composed command.
# ─────────────────────────────────────────────────────────────────────────────
stub_bin="$temporary_root/bin"
mkdir -p "$stub_bin"

write_stub() {
    local name="$1"
    cat >"$stub_bin/$name" <<'STUB'
#!/usr/bin/env bash
printf '%s %s\n' "${0##*/}" "$*" >>"$STUB_LOG"
STUB
    chmod +x "$stub_bin/$name"
}
write_stub nice
write_stub cpulimit
write_stub memlimit

cat >"$stub_bin/uname" <<'STUB'
#!/usr/bin/env bash
case "${1:-}" in
    -s) printf '%s\n' "$STUB_UNAME_S" ;;
    -m) printf '%s\n' "${STUB_UNAME_M:-arm64}" ;;
    *) exec /usr/bin/uname "$@" ;;
esac
STUB
chmod +x "$stub_bin/uname"

# Run the wrapper with the stubs in front of PATH. $1 is the reported OS, $2 the
# space-separated stub names to keep available; the rest is the argument list.
run_wrapper() {
    local os="$1" keep="$2"
    shift 2
    local bin="$temporary_root/run-bin"
    rm -rf "$bin"
    mkdir -p "$bin"
    local name
    cp "$stub_bin/uname" "$bin/uname"
    for name in $keep; do
        cp "$stub_bin/$name" "$bin/$name"
    done
    : >"$temporary_root/log"
    set +e
    STUB_LOG="$temporary_root/log" STUB_UNAME_S="$os" \
        STUB_UNAME_M="${STUB_ARCH:-arm64}" PATH="$bin:$PATH" \
        "$BASH" "$wrapper" "$@" >"$temporary_root/out" 2>"$temporary_root/err"
    RUN_RC=$?
    set -e
    RUN_LOG="$(cat "$temporary_root/log")"
    RUN_ERR="$(cat "$temporary_root/err")"
}

# ── Usage and unsupported-OS refusals ────────────────────────────────────────
run_wrapper Linux '' 2G
[ "$RUN_RC" -eq 64 ] || note_fail "too few arguments exited $RUN_RC, expected 64"

run_wrapper Linux '' 2G 400 not-a-separator true
[ "$RUN_RC" -eq 64 ] || note_fail "a missing -- separator exited $RUN_RC, expected 64"

run_wrapper Plan9 '' 2G 400 -- true
[ "$RUN_RC" -eq 69 ] || note_fail "an unsupported OS exited $RUN_RC, expected 69"
case "$RUN_ERR" in
    *'Unsupported operating system'*) ;;
    *) note_fail "an unsupported OS did not say so: $RUN_ERR" ;;
esac

# ── Linux nested caps: an existing lower ulimit is already protective ────────
cat >"$stub_bin/true" <<'STUB'
#!/usr/bin/env bash
exit 0
STUB
chmod +x "$stub_bin/true"
if (ulimit -v 262144) >/dev/null 2>&1; then
    nested_bin="$temporary_root/nested-bin"
    mkdir -p "$nested_bin"
    cp "$stub_bin/uname" "$nested_bin/uname"
    cp "$stub_bin/true" "$nested_bin/true"
    ln -s "$BASH" "$nested_bin/bash"
    set +e
    (
        ulimit -v 262144
        STUB_LOG="$temporary_root/log" STUB_UNAME_S=Linux PATH="$nested_bin" \
            "$BASH" "$wrapper" 512M 400 -- true
    ) >"$temporary_root/nested.out" 2>"$temporary_root/nested.err"
    RUN_RC=$?
    set -e
    [ "$RUN_RC" -eq 0 ] || note_fail "a nested lower Linux ulimit exited $RUN_RC, expected 0"
    case "$(cat "$temporary_root/nested.err")" in
        *'keeping existing virtual-memory limit'*) ;;
        *) note_fail "a nested lower Linux ulimit did not report the kept cap: $(cat "$temporary_root/nested.err")" ;;
    esac
fi

# ── Darwin selects memlimit when it is present ───────────────────────────────
run_wrapper Darwin 'nice memlimit' 2G 400 -- my-command --flag
[ "$RUN_RC" -eq 0 ] || note_fail "the memlimit path exited $RUN_RC, expected 0"
case "$RUN_LOG" in
    'nice -n 10 memlimit 2147483648 -- my-command --flag') ;;
    *) note_fail "the memlimit path composed: $RUN_LOG" ;;
esac
case "$RUN_ERR" in
    *'not enforced'*) note_fail 'the memlimit path still printed the degraded warning' ;;
esac

# cpulimit composes inside memlimit, so its child stays in the capped tree.
run_wrapper Darwin 'nice memlimit cpulimit' 2G 400 -- my-command
case "$RUN_LOG" in
    'nice -n 10 memlimit 2147483648 -- cpulimit --limit=400 -- my-command') ;;
    *) note_fail "memlimit and cpulimit composed: $RUN_LOG" ;;
esac

# ── Darwin degrades, naming memlimit, when it is absent ──────────────────────
run_wrapper Darwin 'nice cpulimit' 2G 400 -- my-command
[ "$RUN_RC" -eq 0 ] || note_fail "the degraded path exited $RUN_RC, expected 0"
case "$RUN_ERR" in
    *memlimit*) ;;
    *) note_fail "the degraded path did not name memlimit: $RUN_ERR" ;;
esac
case "$RUN_ERR" in
    *'github.com/pingiun/memlimit'*) ;;
    *) note_fail "the degraded path did not say how to install memlimit: $RUN_ERR" ;;
esac
case "$RUN_LOG" in
    *memlimit*) note_fail "the degraded path invoked memlimit anyway: $RUN_LOG" ;;
esac
case "$RUN_LOG" in
    'cpulimit --limit=400 -- nice -n 10 my-command') ;;
    *) note_fail "the degraded path composed: $RUN_LOG" ;;
esac

# ── Suffix translation: memlimit is handed bytes, so it cannot drift ──────────
assert_bytes() {
    local size="$1" expected="$2"
    run_wrapper Darwin 'nice memlimit' "$size" 400 -- my-command
    case "$RUN_LOG" in
        "nice -n 10 memlimit $expected -- my-command") ;;
        *) note_fail "$size became: $RUN_LOG (expected $expected bytes)" ;;
    esac
}
assert_bytes 2G 2147483648
assert_bytes 512M 536870912
assert_bytes 64K 65536

# A size memlimit cannot read is refused here rather than passed through.
run_wrapper Darwin 'nice memlimit' 2Gi 400 -- my-command
[ "$RUN_RC" -eq 64 ] || note_fail "an unsupported size exited $RUN_RC, expected 64"

# ── The installer's soft requirement ─────────────────────────────────────────
# memlimit is a soft requirement of resource-limited-testing: on Darwin without
# it the skill still installs, because the wrapper degrades to nice + cpulimit,
# and the run warns which capability is lost instead of refusing.
#
# The bash install.sh version of this section stubbed `uname` on PATH to make
# a Linux CI runner answer as Darwin/arm64 or Darwin/x86_64, then ran the real
# installer against that stub and asserted its stdout/stderr wording -- because
# install.sh's own `runtime_requirements()`/condition matching genuinely
# called `uname` at runtime, a stub could fake the host it saw.
#
# The Rust installer's equivalent (src/installer/src/requirements.rs) cannot
# be driven the same way: `host_os()`/`host_arch()` resolve at COMPILE time
# (`cfg!(target_os = "macos")`, `std::env::consts::ARCH`), not by shelling out
# to `uname`, so no PATH stub on this (Linux) host can make a compiled binary
# answer as Darwin/arm64. There is no end-to-end substitute to write here.
#
# What condition_applies()/host_os()/host_arch() actually do -- matching a
# requires.tsv row's `Darwin:arm64`-style condition against an OS/arch pair,
# including the exact "arm64 only, not Intel" case this section used to pin
# -- is covered directly in requirements.rs's own unit tests
# (a_wildcard_condition_always_matches, alternation_matches_either_side, and
# resource-limited-testing/requires.tsv's own memlimit row is read by
# requirements_for's tests elsewhere in that file), parameterized by literal
# os/arch strings rather than a stubbed uname. That is strictly more direct:
# it asserts the matching logic itself, not a Linux runner's ability to
# impersonate a Mac.

[ "$(t_failures)" -eq 0 ] || exit 1
printf '%s\n' 'test-limited-run-contract: PASS'
