#!/usr/bin/env bash
# MODE: DEV
# test-limited-run-contract — resource-limited-testing's wrapper contract, and
# the installer gate that keeps its macOS memory cap from being absent.
#
# Usage: test-limited-run-contract.sh
#
# The macOS assertions run on any host: `uname` and `memlimit` are stubbed on
# PATH, because a gate that is only exercised on a mac is a gate nobody runs.
# Root-level install.sh is tested from here for the same reason
# test-installer-manifest.sh is: run-tests.sh only discovers planning/tests.
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
run_installer() {
    local keep="$1"
    shift
    local bin="$temporary_root/install-bin"
    rm -rf "$bin"
    mkdir -p "$bin"
    local name
    cp "$stub_bin/uname" "$bin/uname"
    for name in $keep; do
        cp "$stub_bin/$name" "$bin/$name"
    done
    set +e
    STUB_LOG="$temporary_root/log" STUB_UNAME_S=Darwin \
        STUB_UNAME_M="${STUB_ARCH:-arm64}" PATH="$bin:$PATH" \
        AI_SKILLS_NO_SPLASH=1 "$BASH" "$repo_dir/install.sh" "$@" \
        >"$temporary_root/iout" 2>"$temporary_root/ierr" </dev/null
    RUN_RC=$?
    set -e
    RUN_ERR="$(cat "$temporary_root/ierr")"
}

target="$temporary_root/skills"
mkdir -p "$target"
run_installer '' --skill resource-limited-testing --target "$target" --yes
[ "$RUN_RC" -eq 0 ] \
    || note_fail "the installer refused Darwin without memlimit (exit $RUN_RC): $RUN_ERR"
[ -f "$target/resource-limited-testing/scripts/limited-run.sh" ] \
    || note_fail 'the soft requirement blocked the install instead of warning'
case "$RUN_ERR" in
    *'memlimit (soft requirement of resource-limited-testing)'*) ;;
    *) note_fail "the installer did not name the missing memlimit: $RUN_ERR" ;;
esac
case "$RUN_ERR" in
    *'memlimit-installer.sh'*) ;;
    *) note_fail "the installer printed no memlimit install hint: $RUN_ERR" ;;
esac
# The warning must name the capability that is lost, not only the tool.
case "$RUN_ERR" in
    *'RAM cap'*) ;;
    *) note_fail "the warning did not say what memlimit buys: $RUN_ERR" ;;
esac
# And the summary must carry the same warning on the installed line.
RUN_OUT="$(cat "$temporary_root/iout")"
case "$RUN_OUT" in
    *"Installed: $target/resource-limited-testing"*'warning: memlimit missing'*) ;;
    *) note_fail "the summary did not flag the degraded install: $RUN_OUT" ;;
esac

rm -rf "$target"
mkdir -p "$target"
run_installer '' --install-skill resource-limited-testing \
    --target "$target" --approval yes
[ "$RUN_RC" -eq 0 ] \
    || note_fail "the CLI install refused Darwin without memlimit (exit $RUN_RC)"

rm -rf "$target"
mkdir -p "$target"
run_installer memlimit --install-skill resource-limited-testing \
    --target "$target" --approval yes
[ "$RUN_RC" -eq 0 ] || note_fail "the CLI install with memlimit exited $RUN_RC: $RUN_ERR"
[ -f "$target/resource-limited-testing/scripts/limited-run.sh" ] \
    || note_fail 'the accepted installer did not install the wrapper'

# ── Intel macOS: memlimit is arm64-only, so it must not be demanded there ────
STUB_ARCH=x86_64 run_wrapper Darwin 'nice cpulimit' 2G 400 -- true
case "$RUN_ERR" in
    *'Apple Silicon only'*) ;;
    *) note_fail "an Intel Mac was not told memlimit is Apple Silicon only: $RUN_ERR" ;;
esac
case "$RUN_ERR" in
    *memlimit-installer.sh*)
        note_fail 'an Intel Mac was told to install an arm64-only tool' ;;
esac
case "$RUN_LOG" in
    *memlimit*) note_fail 'an Intel Mac invoked memlimit' ;;
esac

# And the installer must not require it there, or the skill becomes uninstallable
# on a machine where its degraded path still works.
# Extract the real runtime_requirements() and call it under a stubbed uname, so
# this asserts the shipped function rather than a paraphrase of it.
requirements_fn="$temporary_root/runtime-requirements.sh"
awk '/^runtime_requirements\(\)/,/^}/' "$repo_dir/install.sh" >"$requirements_fn"
[ -s "$requirements_fn" ] || note_fail 'could not extract runtime_requirements() from install.sh'

requires_for_arch() {
    STUB_UNAME_S=Darwin STUB_UNAME_M="$1" PATH="$stub_bin:$PATH" \
        "$BASH" -c '. "$1"; runtime_requirements resource-limited-testing' _ "$requirements_fn"
}

intel_requires="$(requires_for_arch x86_64)"
[ -z "$intel_requires" ] \
    || note_fail "Intel macOS still requires '$intel_requires' (memlimit is arm64-only)"

arm_requires="$(requires_for_arch arm64)"
[ "$arm_requires" = memlimit ] \
    || note_fail "Apple Silicon macOS should require memlimit, got '$arm_requires'"

[ "$(t_failures)" -eq 0 ] || exit 1
printf '%s\n' 'test-limited-run-contract: PASS'
