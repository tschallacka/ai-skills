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
build_hidden_path "$none_bin" openssl python3 node perl socat
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

# ── T48a: planning declares no any-of runtime group any more ─────────────────
# Serve mode is served by the shipped plan-overview binary, so python3, node,
# perl and socat are no longer consulted for planning at all. Hiding all four
# must therefore install planning silently: no member set is named and no
# capability is reported lost. Re-adding such a row would fail HERE.
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
    [ "$plan_rc" -eq 0 ] || note_fail "planning install exited $plan_rc with no runtime present"
    plan_err="$(cat "$temporary_root/err-plan")"
    case "$plan_err" in
        *'soft requirement of planning'*)
            note_fail "planning still declares a soft runtime requirement: $plan_err" ;;
    esac
    [ -f "$target_plan/planning/SKILL.md" ] \
        || note_fail 'planning did not install without the retired runtimes'
fi

# The retired group must not come back through requires.tsv either.
if awk -F'\t' '!/^#/ && NF > 4 && $5 != "" && $5 != "group" { found = 1 } END { exit !found }' \
    "$repo_dir/planning/requires.tsv"; then
    note_fail 'planning/requires.tsv declares an any-of group again'
fi

# ── the bundled rjq removes the former hard external dependency ───────────────
jqless_bin="$temporary_root/bin-jqless"
build_hidden_path "$jqless_bin" rjq
target_jq="$temporary_root/target-rjq"
mkdir -p "$target_jq"
set +e
PATH="$jqless_bin" AI_SKILLS_NO_SPLASH=1 "$BASH" "$installer" \
    --skill planning --target "$target_jq" --yes >/dev/null 2>"$temporary_root/err-rjq" </dev/null
hard_rc=$?
set -e
[ "$hard_rc" -eq 0 ] || note_fail 'the bundled rjq did not allow the install'
[ -f "$target_jq/planning/SKILL.md" ] || note_fail 'the planning skill was not installed with bundled rjq'

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
