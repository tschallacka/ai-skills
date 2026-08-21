#!/usr/bin/env bash
# MODE: DEV
# test-installer-noninteractive.sh — --yes answers every prompt, and without it a
# prompt refuses cleanly.
#
# Installing planning under --yes used to die on "Create ~/.plans as the global
# plans directory?": confirm honoured YES_ALL, which only the interactive "a"
# answer sets, and never YES. A --yes that stops on a question is broken for the
# one job it has.
#
# HOME is redirected per case, because the prompt only appears when the plans
# directory does not exist yet -- on a developer machine it usually does, which is
# how this survived.

set -euo pipefail
export LC_ALL=C

tests_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$tests_dir/../.." && pwd)"
installer="$repo_root/install.sh"
# shellcheck source=planning/tests/lib-test.sh
source "$tests_dir/lib-test.sh"
t_begin

work="$(mktemp -d "${TMPDIR:-/tmp}/installer-noninteractive.XXXXXX")"
trap 'rm -rf "$work"' EXIT

run_planning() { # <fresh-home> <target> [extra args...]
    local home="$1" target="$2"; shift 2
    mkdir -p "$home"
    HOME="$home" "$BASH" "$installer" --skill planning --target "$target" "$@" \
        >"$work/out" 2>&1 </dev/null
}

# ── with --yes: no prompt stops it ──────────────────────────────────────────
rc=0
run_planning "$work/home-yes" "$work/target-yes" --yes || rc=$?
t_assert_eq 'a planning install under --yes succeeds' "$rc" '0'
t_assert_eq 'and nothing asked for interactive input' \
    "$(grep -c 'Interactive input is required' "$work/out" || true)" '0'
# The permission step is the one that used to die, so assert it did its work
# rather than only that the run survived.
t_assert_eq 'the plans directory was created' \
    "$([ -d "$work/home-yes/.plans" ] && printf yes || printf no)" 'yes'
t_assert_eq 'the skill was installed' \
    "$([ -f "$work/target-yes/planning/SKILL.md" ] && printf yes || printf no)" 'yes'

# ── without --yes and with no tty: refuse, cleanly ─────────────────────────
rc=0
run_planning "$work/home-no" "$work/target-no" || rc=$?
t_assert_eq 'without --yes the prompt refuses' "$rc" '1'
t_assert_eq 'with the actionable message' \
    "$(grep -c 'Interactive input is required' "$work/out" || true)" '1'
# bash prints its own complaint about the closed fd before the message can be
# written, which buried the real one.
t_assert_eq 'and no raw shell error precedes it' \
    "$(grep -c 'invalid file descriptor' "$work/out" || true)" '0'

# ── --yes is not a licence to skip the work ────────────────────────────────
# A skill install under --yes still writes what it promised, so "yes to
# everything" cannot be implemented as "do nothing and return 0".
rc=0
run_planning "$work/home-again" "$work/target-again" --yes || rc=$?
t_assert_eq 'a second install under --yes also succeeds' "$rc" '0'
t_assert_eq 'and the scripts are there' \
    "$([ -x "$work/target-again/planning/scripts/create-plan.sh" ] && printf yes || printf no)" 'yes'

t_end
