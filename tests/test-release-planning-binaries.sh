#!/usr/bin/env bash
# MODE: DEV
# test-release-planning-binaries.sh — every compiled command a shipped planning
# wrapper hands to plan_exec_compiled_binary_if_present ships with it (GH #198).
#
# A wrapper such as verify-skill-load.sh has no bash body left: when the
# launcher finds no compiled binary it dies with 69. The binaries are generated,
# never tracked, so test-skill-files-manifest.sh exempts every extensionless
# file under planning/scripts/ and nothing tied a wrapper to its binary's row.
# verify-skill-load was missing from skill_files() for exactly that reason, and
# every release install failed the skill's own self-test.
#
# The needed set is derived from the shipped wrappers themselves, so a new
# wrapper is covered without editing this file. Pure declarations: it needs no
# built tree and no cross-compilation.

set -euo pipefail
export LC_ALL=C

tests_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$tests_dir/.." && pwd)"
# shellcheck source=planning/tests/lib-test.sh
source "$repo_root/planning/tests/lib-test.sh"
t_begin

work="$(mktemp -d "${TMPDIR:-/tmp}/release-planning-binaries.XXXXXX")"
trap 'rm -rf "$work"' EXIT

# SOURCE_ROOT is assigned after sourcing, because 05-config.sh declares it empty.
# shellcheck disable=SC1090
source "$repo_root/installer/src/05-config.sh"
# shellcheck disable=SC1090
source "$repo_root/installer/src/50-manifest.sh"
SOURCE_ROOT="$repo_root"
SOURCE_VERSION='test'
REPO_REF='test'

skill_files planning | LC_ALL=C sort -u > "$work/listed"

# needed_by <wrapper> — the binary names one wrapper execs, one per line.
# Comment lines are dropped first: a doc comment naming the launcher is prose,
# not a call.
needed_by() {
    sed -e '/^[[:space:]]*#/d' \
        -e 's/.*plan_exec_compiled_binary_if_present \([a-z0-9_-][a-z0-9_-]*\) .*/\1/p' \
        -e 'd' "$1"
}

: > "$work/needed"
while IFS= read -r path; do
    case "$path" in scripts/*.sh) ;; *) continue ;; esac
    wrapper="$repo_root/planning/$path"
    # The compiled plan-*-lib.sh files are generated; they define the launcher
    # and never call it, so an unbuilt tree loses nothing by skipping them.
    [ -f "$wrapper" ] || continue
    needed_by "$wrapper" | while IFS= read -r name; do
        printf '%s\t%s\n' "$name" "$path"
    done >> "$work/needed"
done < "$work/listed"
LC_ALL=C sort -u "$work/needed" -o "$work/needed"

# Positive controls: a pattern that stopped matching would make every check
# below pass over an empty set.
needed_count="$(cut -f1 "$work/needed" | LC_ALL=C sort -u | grep -c . || true)"
t_assert_eq 'the shipped wrappers name at least 40 compiled commands' \
    "$([ "$needed_count" -ge 40 ] && printf 'at least 40')" 'at least 40'
t_assert_contains 'verify-skill-load.sh is seen to exec verify-skill-load (GH #198)' \
    'verify-skill-load' "$(cut -f1 "$work/needed")"

# rows_in <file> <column> — the values of one column of a TSV, comments dropped.
rows_in() {
    awk -F '\t' -v c="$2" '$0 !~ /^#/ && NF >= c { print $c }' "$1" | LC_ALL=C sort -u
}
rows_in "$repo_root/planning/PACKAGE-MANIFEST.tsv" 1 > "$work/manifest"
awk -F '\t' '$0 !~ /^#/ && ($2 == "runtime-binary" || $2 == "build-generator") { print $3 }' \
    "$repo_root/planning/rust-migration.tsv" | LC_ALL=C sort -u > "$work/staged"

unlisted='' unmanifested='' unstaged='' uncrated=''
while IFS="$(printf '\t')" read -r name wrapper; do
    [ -n "$name" ] || continue
    grep -qxF "scripts/$name" "$work/listed" || unlisted="$unlisted $name($wrapper)"
    grep -qxF "planning/scripts/$name" "$work/manifest" || unmanifested="$unmanifested $name"
    # build-release.sh --prepare stages exactly these rows beside the wrappers.
    grep -qxF "$name" "$work/staged" || unstaged="$unstaged $name"
    # The same command-to-crate mapping resolve_planning_command_artifact uses.
    crate="$name"
    [ "$name" = overview-state ] && crate=plan-overview
    [ -f "$repo_root/src/$crate/Cargo.toml" ] || uncrated="$uncrated $name"
done < "$work/needed"

t_assert_eq 'every exec'"'"'d command is in skill_files planning, so the release ships it' \
    "${unlisted# }" ''
t_assert_eq 'every exec'"'"'d command has a planning/PACKAGE-MANIFEST.tsv row' \
    "${unmanifested# }" ''
t_assert_eq 'every exec'"'"'d command is a runtime-binary row build-release.sh --prepare stages' \
    "${unstaged# }" ''
t_assert_eq 'every exec'"'"'d command has a crate to build it from' \
    "${uncrated# }" ''

printf '%s: checked %s compiled command(s) exec'"'"'d by shipped planning wrappers\n' \
    "${0##*/}" "$needed_count" >&2

t_end
