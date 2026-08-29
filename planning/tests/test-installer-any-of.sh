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

# ── chat needs no runtime tool: it installs cleanly with a minimal PATH ─────
# The rust chat server mints its own certificate in-crate (rcgen) and the
# client pins it (TOFU), so chat has no runtime requirement at all. Installing
# it under a PATH with every tool hidden must succeed with no warning.
none_bin="$temporary_root/bin-chat-none"
build_hidden_path "$none_bin" openssl python3 node perl socat jq
target_chat="$temporary_root/target-chat"
mkdir -p "$target_chat"
set +e
PATH="$none_bin" AI_SKILLS_NO_SPLASH=1 "$BASH" "$installer" \
    --skill chat --target "$target_chat" --yes >"$temporary_root/out-chat" 2>"$temporary_root/err-chat" </dev/null
chat_rc=$?
set -e
[ "$chat_rc" -eq 0 ] || note_fail "chat with no runtime tool installed exited $chat_rc"
grep -qi 'degraded\|warning.*missing.*tool\|requires' "$temporary_root/err-chat" \
    && note_fail "chat warned about a runtime tool it does not need: $(cat "$temporary_root/err-chat")"
[ -f "$target_chat/chat/SKILL.md" ] || note_fail 'chat install did not place SKILL.md'

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

# ── single-tool strengths are untouched: jq hard blocks, openssl soft warns ──
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

opensslless_bin="$temporary_root/bin-sslg"
build_hidden_path "$opensslless_bin" openssl
target_ssl="$temporary_root/target-ssl"
mkdir -p "$target_ssl"
set +e
PATH="$opensslless_bin" AI_SKILLS_NO_SPLASH=1 "$BASH" "$installer" \
    --skill planning --target "$target_ssl" --yes >"$temporary_root/out-ssl" 2>"$temporary_root/err-ssl" </dev/null
soft_rc=$?
set -e
[ "$soft_rc" -eq 0 ] || note_fail "a soft single requirement changed the exit status: rc=$soft_rc"
grep -Fq 'openssl' "$temporary_root/err-ssl" \
    || note_fail 'the soft single requirement was not warned about'
[ -f "$target_ssl/planning/SKILL.md" ] || note_fail 'a soft single miss blocked the planning install'

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
