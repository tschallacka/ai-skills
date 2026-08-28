#!/usr/bin/env bash
# MODE: DEV
# test-installer-any-of — an any-of requirement group warns once when none of
# its members resolves, stays silent when one does, and never changes how the
# single-tool strengths behave (T39).
#
# Usage: test-installer-any-of.sh
#
# Tools are made absent by running the installer under a PATH of symlinks to
# everything except the hidden names, and present by adding throwaway shims, so
# the shipped runtime_requirement_met() is what decides, not a paraphrase.
set -euo pipefail
# shellcheck source=planning/tests/lib-test.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib-test.sh"
t_begin

export LC_ALL=C

repo_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
installer="$repo_dir/install.sh"
temporary_root="$(mktemp -d "${TMPDIR:-/tmp}/installer-anyof.XXXXXX")"
trap 'rm -rf "$temporary_root"' EXIT

note_fail() { printf 'installer-anyof: %s\n' "$1" >&2; t_record "$1"; }

# A PATH whose bin dir symlinks every executable on the real PATH except the
# named tools. $1 is the directory to fill; the rest are the names to hide.
build_hidden_path() {
    local dest="$1" name e want hide
    shift
    mkdir -p "$dest"
    while IFS= read -r d; do
        [ -d "$d" ] || continue
        for e in "$d"/*; do
            [ -x "$e" ] || continue
            name="${e##*/}"
            hide=""
            for want in "$@"; do
                if [ "$name" = "$want" ]; then
                    hide=1
                fi
            done
            [ -z "$hide" ] || continue
            ln -sf "$e" "$dest/$name" 2>/dev/null || true
        done
    done < <(printf '%s\n' "$PATH" | tr ':' '\n')
}

# Throwaway runtimes: presence without a real install, so any one member can be
# added to a hidden PATH without touching this machine.
fake_bin="$temporary_root/fake"
mkdir -p "$fake_bin"
for member in python3 node perl socat; do
    printf '#!/bin/sh\nexit 0\n' > "$fake_bin/$member"
    chmod +x "$fake_bin/$member"
done

run_installer() { # <target> -- <PATH>
    local target="$1" path="$2"
    set +e
    PATH="$path" AI_SKILLS_NO_SPLASH=1 "$BASH" "$installer" \
        --skill chat --target "$target" --yes >"$temporary_root/out" 2>"$temporary_root/err" </dev/null
    RUN_RC=$?
    set -e
    RUN_ERR="$(cat "$temporary_root/err")"
    RUN_OUT="$(cat "$temporary_root/out")"
}

# ── all four members absent: exactly one warning naming set and capability ──
none_bin="$temporary_root/bin-none"
build_hidden_path "$none_bin" python3 node perl socat
if PATH="$none_bin" command -v python3 >/dev/null 2>&1 ||
    PATH="$none_bin" command -v node >/dev/null 2>&1 ||
    PATH="$none_bin" command -v perl >/dev/null 2>&1 ||
    PATH="$none_bin" command -v socat >/dev/null 2>&1; then
    printf 'installer-anyof: UNCONFIGURED (a chat runtime could not be hidden from PATH)\n' >&2
    exit 0
fi
target_none="$temporary_root/target-none"
mkdir -p "$target_none"
run_installer "$target_none" "$none_bin"
[ "$RUN_RC" -eq 0 ] || note_fail "a soft group miss changed the exit status: rc=$RUN_RC"
warnings="$(grep -c 'degraded form' "$temporary_root/err" || true)"
[ "$warnings" -eq 1 ] || note_fail "expected exactly one degraded warning, got $warnings"
case "$RUN_ERR" in
    *'any of python3, node, perl, socat (soft requirement of chat)'*) ;;
    *) note_fail "the warning did not name the member set: $RUN_ERR" ;;
esac
case "$RUN_ERR" in
    *'listening socket'*) ;;
    *) note_fail "the warning did not name the lost capability: $RUN_ERR" ;;
esac
[ -f "$target_none/chat/SKILL.md" ] || note_fail 'a soft group miss blocked the chat install'

# The hint for the missing group must name a usable command and prefer the head
# of the chain (python3), whatever platform prints it.
hint_block="$(sed -n '/install one of them with:/,$p' "$temporary_root/err")"
[ -n "$hint_block" ] || note_fail 'the group miss printed no install hint'
first_hint_word="$(printf '%s\n' "$hint_block" | sed -n '2s/^[[:space:]]*\([A-Za-z0-9._-]*\).*/\1/p')"
if [ "$first_hint_word" != "install" ]; then
    if ! command -v "$first_hint_word" >/dev/null 2>&1; then
        note_fail "the group hint named '$first_hint_word', which does not exist here"
    fi
fi
case "$hint_block" in
    *python3*) ;;
    *) note_fail "the group hint did not recommend the chain head: $hint_block" ;;
esac

# ── one member present: no warning at all, whichever member that is ─────────
one_bin="$temporary_root/bin-one"
build_hidden_path "$one_bin" python3 node perl socat
target_one="$temporary_root/target-one"
mkdir -p "$target_one"
run_installer "$target_one" "$one_bin:$fake_bin"
[ "$RUN_RC" -eq 0 ] || note_fail "installing with python3 present exited $RUN_RC"
warnings="$(grep -c 'degraded form' "$temporary_root/err" || true)"
[ "$warnings" -eq 0 ] || note_fail "a satisfied group still warned ($warnings time(s))"

node_only_bin="$temporary_root/bin-node"
build_hidden_path "$node_only_bin" python3 node perl socat
rm -f "$fake_bin/python3"
target_node="$temporary_root/target-node"
mkdir -p "$target_node"
run_installer "$target_node" "$node_only_bin:$fake_bin"
[ "$RUN_RC" -eq 0 ] || note_fail "installing with only node present exited $RUN_RC"
warnings="$(grep -c 'degraded form' "$temporary_root/err" || true)"
[ "$warnings" -eq 0 ] || note_fail "a satisfied group (second member) still warned"

restore_python3() {
    printf '#!/bin/sh\nexit 0\n' > "$fake_bin/python3"
    chmod +x "$fake_bin/python3"
}
restore_python3

# ── T48a: planning's own group is asserted, not only chat's ──────────────────
# Deleting one of planning's four runtime rows (and regenerating) must fail
# HERE, on the reported members and capability, not only via group-id
# uniqueness elsewhere. The group is soft: serve mode degrades, the file
# overview still ships.
plan_none_bin="$temporary_root/bin-plan-none"
build_hidden_path "$plan_none_bin" python3 node perl socat
if PATH="$plan_none_bin" command -v python3 >/dev/null 2>&1 ||
    PATH="$plan_none_bin" command -v perl >/dev/null 2>&1; then
    printf 'installer-anyof: UNCONFIGURED (a planning runtime could not be hidden)\n' >&2
else
    target_plan="$temporary_root/target-plan"
    mkdir -p "$target_plan"
    set +e
    PATH="$plan_none_bin" AI_SKILLS_NO_SPLASH=1 "$BASH" "$installer" \
        --skill planning --target "$target_plan" --yes \
        >"$temporary_root/out-plan" 2>"$temporary_root/err-plan" </dev/null
    plan_rc=$?
    set -e
    [ "$plan_rc" -eq 0 ] || note_fail "planning soft group miss changed the exit status: rc=$plan_rc"
    plan_err="$(cat "$temporary_root/err-plan")"
    case "$plan_err" in
        *'any of python3, node, perl, socat (soft requirement of planning)'*) ;;
        *) note_fail "planning's warning did not name its member set: $plan_err" ;;
    esac
    case "$plan_err" in
        *'serve mode'*|*'listening socket'*) ;;
        *) note_fail "planning's warning did not name the lost serve capability: $plan_err" ;;
    esac
    [ -f "$target_plan/planning/SKILL.md" ] \
        || note_fail 'a soft group miss blocked the planning install'
fi

# ── single-tool strengths are untouched: jq hard blocks, a soft row warns ────
jqless_bin="$temporary_root/bin-jqless"
build_hidden_path "$jqless_bin" jq
target_jq="$temporary_root/target-jq"
mkdir -p "$target_jq"
set +e
PATH="$jqless_bin" AI_SKILLS_NO_SPLASH=1 "$BASH" "$installer" \
    --skill planning --target "$target_jq" --yes >/dev/null 2>"$temporary_root/err-jq" </dev/null
hard_rc=$?
set -e
[ "$hard_rc" -ne 0 ] || note_fail 'a hard single requirement no longer blocks the install'
grep -Fq 'jq' "$temporary_root/err-jq" || note_fail 'the hard block did not name jq'

# The soft single requirement was planning's openssl until plan-crypt replaced
# it and the row left planning/requires.tsv. merge-request-etiquette's `git` is
# the remaining ungrouped soft row that applies on every platform, so the arm
# moved there rather than being dropped: an ungrouped soft miss must still warn
# by name and still install, and nothing else covers that shape.
gitless_bin="$temporary_root/bin-gitless"
build_hidden_path "$gitless_bin" git
target_soft="$temporary_root/target-soft"
mkdir -p "$target_soft"
set +e
PATH="$gitless_bin" AI_SKILLS_NO_SPLASH=1 "$BASH" "$installer" \
    --skill merge-request-etiquette --target "$target_soft" --yes \
    >"$temporary_root/out-soft" 2>"$temporary_root/err-soft" </dev/null
soft_rc=$?
set -e
[ "$soft_rc" -eq 0 ] || note_fail "a soft single requirement changed the exit status: rc=$soft_rc"
grep -Fq 'git' "$temporary_root/err-soft" \
    || note_fail 'the soft single requirement was not warned about'
[ -f "$target_soft/merge-request-etiquette/SKILL.md" ] \
    || note_fail 'a soft single miss blocked the install'

# And planning now declares no ungrouped soft requirement at all, which is the
# point of shipping plan-crypt: a static binary asks the target machine for
# nothing, so the openssl row went away rather than moving.
if awk -F'\t' '$1 == "openssl" { found = 1 } END { exit !found }' "$repo_dir/planning/requires.tsv"; then
    note_fail 'planning/requires.tsv declares openssl again'
fi

# ── the generated artifact matches its sources ───────────────────────────────
# One retry: the check is a pure regenerate-and-diff, so only genuine drift
# fails twice; a resource-cap kill of one awk pass must not read as drift.
check_ok=0
for _attempt in 1 2; do
    if "$BASH" "$repo_dir/installer/build.sh" --check >/dev/null 2>&1; then
        check_ok=1
        break
    fi
done
[ "$check_ok" -eq 1 ] || note_fail 'installer/build.sh --check failed twice; generated tables drifted from sources'

[ "$(t_failures)" -eq 0 ] || exit 1
printf '%s\n' 'test-installer-any-of: PASS'
