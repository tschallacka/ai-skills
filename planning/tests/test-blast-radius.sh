#!/usr/bin/env bash
# MODE: DEV
# test-blast-radius.sh — the integration-safety report tells the truth about a
# change set: a stale generated artifact fails, a new runtime registry that would
# not ship fails, and a consistent tree is quiet.
#
# Dev-only, like test-mermaid-accuracy.sh: blast-radius.sh is maintainer tooling
# and is not in PACKAGE-MANIFEST.tsv, so neither is its test.

set -euo pipefail
# shellcheck source=planning/tests/lib-test.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib-test.sh"
t_begin

export LC_ALL=C

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"
tool=./blast-radius.sh

probe_json=planning/blast-radius-probe.json
probe_test=planning/tests/test-blast-radius-probe.sh
probe_part=/tmp/blast-radius-part.$$
cleanup() {
    rm -f "$probe_json" "$probe_test"
    [ -f "$probe_part" ] && cp "$probe_part" installer/src/00-header.sh
    rm -f "$probe_part"
}
trap cleanup EXIT

note_fail() { printf 'FAIL: %s\n' "$1" >&2; t_record "$1"; }

# For an assertion about blast-radius's output: print what it actually said. A
# bare "the warning was not counted" sent a CI failure through two rounds of
# guessing, because the output it disagreed with was never shown.
note_fail_out() { # <message> <output>
    note_fail "$1"
    printf '  what blast-radius printed:\n' >&2
    printf '%s\n' "$2" | sed 's/^/    /' >&2
}

# Every assertion below needs real history: blast-radius resolves a merge base
# against master, and the drift case asks for the parent of the newest commit
# touching planning/SKILL.md. A depth-1 checkout has neither, which surfaced as
# three unrelated-looking failures in CI while the test passed locally. Say it
# once, plainly, rather than leaving the next reader to work it out.
if [ "$(git -C "$repo_root" rev-parse --is-shallow-repository 2>/dev/null)" = true ]; then
    note_fail 'this test needs full history; the checkout is shallow (use fetch-depth: 0)'
    printf 'test-blast-radius: %d failure(s).\n' "$(t_failures)" >&2
    exit 1
fi

# Sets RUN_OUT rather than printing it: called in a command substitution, every
# note_fail inside would land in a subshell and be silently discarded.
RUN_OUT=""
run() { # <expected-rc> <label> <args...>
    local want="$1" label="$2"; shift 2
    local rc=0
    RUN_OUT="$("$tool" "$@" 2>&1)" || rc=$?
    [ "$rc" -eq "$want" ] || note_fail "$label: exited $rc, want $want"
}

# ---- --help is a contract of its own ----------------------------------------
run 0 '--help' --help
help_out="$RUN_OUT"
case "$help_out" in
    *'integration-safety report'*) ;;
    *) note_fail '--help did not describe what the tool is for' ;;
esac
rc=0; "$tool" --nonsense >/dev/null 2>&1 || rc=$?
[ "$rc" -eq 64 ] || note_fail "an unknown option exited $rc, want 64"

# ---- a consistent tree is quiet ---------------------------------------------
run 0 'consistent tree' planning/MAINTAINER-STYLE-CONTRACT.md installer/src/00-header.sh
out="$RUN_OUT"
case "$out" in
    *'0 failure(s)'*) ;;
    *) note_fail 'a consistent change set reported failures' ;;
esac
case "$out" in
    *'install.sh is generated'*) ;;
    *) note_fail 'the installer coupling was not reported for an installer part' ;;
esac

# ---- a stale generated artifact fails ---------------------------------------
cp installer/src/00-header.sh "$probe_part"
printf '\n# blast-radius probe\n' >> installer/src/00-header.sh
run 1 'stale install.sh' installer/src/00-header.sh
out="$RUN_OUT"
case "$out" in
    *'install.sh is stale'*) ;;
    *) note_fail 'a stale install.sh was not reported with its remedy' ;;
esac
cp "$probe_part" installer/src/00-header.sh
rm -f "$probe_part"

# ---- a new runtime registry that would not ship fails -----------------------
printf '{}\n' > "$probe_json"
run 1 'unshipped registry' "$probe_json"
out="$RUN_OUT"
case "$out" in
    *'will not ship'*) ;;
    *) note_fail 'a new planning/*.json with no manifest row was not a failure' ;;
esac
rm -f "$probe_json"

# ---- a new test file warns but does not fail --------------------------------
printf '#!/usr/bin/env bash\n' > "$probe_test"
run 0 'unregistered test' "$probe_test"
out="$RUN_OUT"
case "$out" in
    *'no PACKAGE-MANIFEST row'*) ;;
    *) note_fail_out 'a new unregistered test produced no warning' "$out" ;;
esac
case "$out" in
    *'1 warning(s)'*) ;;
    *) note_fail_out 'the warning was not counted' "$out" ;;
esac
rm -f "$probe_test"

# ---- drift is reported against the base -------------------------------------
# The parent of the newest commit touching the maintainer contract, so drift is guaranteed
# rather than depending on what the last commit happened to include.
drift_base="$(git log -1 --format=%H -- planning/MAINTAINER-STYLE-CONTRACT.md)^"
run 0 'drift' --base "$drift_base" planning/MAINTAINER-STYLE-CONTRACT.md
out="$RUN_OUT"
case "$out" in
    *'planning/MAINTAINER-STYLE-CONTRACT.md changed in '*) ;;
    *) note_fail_out 'drift against an explicit base was not reported' "$out" ;;
esac
case "$out" in
    *'verify those commits survive'*) ;;
    *) note_fail_out 'the drift note did not say what to do about it' "$out" ;;
esac

if [ "$(t_failures)" -ne 0 ]; then
    printf 'test-blast-radius: %d failure(s).\n' "$(t_failures)" >&2
    exit 1
fi
printf 'test-blast-radius: PASS\n'
