#!/usr/bin/env bash
# MODE: DEV
# test-plans-root-parity.sh - the two global-plans-root resolvers agree.
#
# `plan_default_root` (plan-core-lib.sh) is the canonical resolver; the
# `home_plans` function inside plan-root.sh is a second, independent copy,
# because that script sources no library. Two implementations of one rule can
# disagree, so they are pinned to each other rather than trusted - the same
# arrangement test-plan-crypt.sh uses for the compiled and shell SHA-256 rungs.
#
# home_plans is script-local with no subcommand that prints it, so it is
# asserted through behaviour: `plan-root.sh resolve` returns a recognized
# scoped global root when one exists, and the prefix of what it returns is
# home_plans. That is a stronger assertion than reading the function would be.
#
# Usage:
#   test-plans-root-parity.sh

set -uo pipefail
export LC_ALL=C

test_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=planning/tests/lib-test.sh
source "$test_dir/lib-test.sh"
scripts="$test_dir/../scripts"

t_begin

temporary_root="$(mktemp -d "${TMPDIR:-/tmp}/plans-root-parity.XXXXXX")"
trap 'rm -rf "$temporary_root"' EXIT

# The canonical resolver, read out of the compiled library in a clean
# environment. PLANS_ROOT must be unset or it short-circuits both resolvers.
canonical_root() {
    env -u PLANS_ROOT -u XDG_CONFIG_HOME -u HOME -u USERPROFILE \
        ${1+XDG_CONFIG_HOME="$1"} ${2+HOME="$2"} \
        "$BASH" -c '
            set -euo pipefail
            source "$1/plan-document-lib.sh"
            plan_default_root
        ' _ "$scripts" 2>/dev/null
}

# What plan-root.sh believes the global base is, observed through resolve.
# A scoped root is only returned when the directory already exists, so the
# fixture creates the one the documented format names.
observed_root() {
    local xdg="${1:-}" home="${2:-}" project user expected got
    project="$temporary_root/proj-$$-$RANDOM"
    mkdir -p "$project"
    user="${USER:-$(id -un)}"
    local base
    if [ -n "$xdg" ]; then
        base="$xdg/tsch-ai-skills/plans"
    else
        base="$home/.config/tsch-ai-skills/plans"
    fi
    expected="$base/$user/$(basename "$project")"
    mkdir -p "$expected"
    got="$(
        env -u PLANS_ROOT -u XDG_CONFIG_HOME -u USERPROFILE \
            ${xdg:+XDG_CONFIG_HOME="$xdg"} HOME="${home:-$temporary_root/nohome}" \
            "$BASH" "$scripts/plan-root.sh" resolve "$project" 2>/dev/null
    )"
    printf '%s\t%s\n' "$expected" "$got"
}

# ---- case 1: XDG_CONFIG_HOME set -------------------------------------------
xdg_home="$temporary_root/xdg"
mkdir -p "$xdg_home"
canonical="$(canonical_root "$xdg_home")"
t_assert_eq 'plan_default_root honours XDG_CONFIG_HOME' \
    "$canonical" "$xdg_home/tsch-ai-skills/plans"

IFS=$'\t' read -r want got <<<"$(observed_root "$xdg_home" "$temporary_root/h1")"
t_assert_eq 'plan-root.sh recognises a scoped root under XDG_CONFIG_HOME' "$got" "$want"
case "$got" in
    "$canonical"/*) : ;;
    *) t_fail "plan-root.sh root [$got] is not under plan_default_root [$canonical]" ;;
esac

# ---- case 2: XDG_CONFIG_HOME unset, HOME/.config is the base ---------------
fallback_home="$temporary_root/h2"
mkdir -p "$fallback_home"
canonical_fallback="$(canonical_root "" "$fallback_home")"
t_assert_eq 'plan_default_root falls back to HOME/.config' \
    "$canonical_fallback" "$fallback_home/.config/tsch-ai-skills/plans"

IFS=$'\t' read -r want got <<<"$(observed_root "" "$fallback_home")"
t_assert_eq 'plan-root.sh recognises a scoped root under HOME/.config' "$got" "$want"
case "$got" in
    "$canonical_fallback"/*) : ;;
    *) t_fail "plan-root.sh root [$got] is not under plan_default_root [$canonical_fallback]" ;;
esac

# ---- case 3: neither resolver keeps the retired ~/.plans default -----------
case "$canonical_fallback" in
    */.plans) t_fail "plan_default_root still resolves the retired ~/.plans default" ;;
esac
if [ "$canonical_fallback" = "$fallback_home/.plans" ]; then
    t_fail 'plan_default_root did not move off ~/.plans'
fi

# ---- case 4: PLANS_ROOT still overrides both ------------------------------
override="$temporary_root/override"
forced="$(
    env -u XDG_CONFIG_HOME PLANS_ROOT="$override" HOME="$fallback_home" \
        "$BASH" -c '
            set -euo pipefail
            source "$1/plan-document-lib.sh"
            plan_default_root
        ' _ "$scripts" 2>/dev/null
)"
t_assert_eq 'PLANS_ROOT still wins over the XDG default' "$forced" "$override"

t_end 'test-plans-root-parity'
