#!/usr/bin/env bash
# MODE: DEV
# ci-test-scope.sh — decide which shell tests and crate tests a CI run has to
# execute, the same idea ci-scope.sh already proves for crate BUILDS, applied
# to the shell suite ci-scope.sh does not touch (T116).
#
# Prints:
#   scope=full|selective
#   reason=<one line saying why>
#   tests=<space-separated repo-relative test paths and crate dirs to run,
#          meaningful only when scope=selective; empty under scope=full,
#          where run-tests.sh's own unfiltered discovery is what runs>
#
# THE DEFAULT IS ALWAYS full, for the same reason ci-scope.sh's is: every
# branch that cannot prove a smaller scope correct returns full, including
# every error path. A selector that narrows when it is confused is worse than
# none, because the green tick then means "we did not look" while reading as
# "we looked".
#
# SELECTION. Each test may declare what it covers with a COVERS marker within
# the first few header lines (right after the shebang and the MODE marker), a
# comment line reading:
#
#   COVERS: <path> <path> ...
#
# A changed path "hits" a COVERS entry when it equals the entry or begins with
# "<entry>/" — a directory entry covers everything under it, a file entry
# covers only itself. A test with NO COVERS marker is UNDECLARED, and an
# undeclared test ALWAYS runs: selection only ever narrows a test that opted
# in, on grounds that test itself stated, never a test nobody has annotated
# yet. That is what keeps marking the rest of the suite a pure optimisation
# rather than a hazard — an unmarked test costs nothing in speed but nothing
# in safety either.
#
# The canonical test/crate list comes from `run-tests.sh --list-only`, not a
# second copy of its suites array here: two lists of "what counts as a test"
# drift, and this selector deciding what NOT to run is exactly the place a
# stale list would fail silently.
#
# Usage:
#   ci-test-scope.sh [--base REF] [--files-from FILE]
#   ci-test-scope.sh --push-to BRANCH
#   ci-test-scope.sh --help
#
#   --base REF        what to diff against (default: origin/master, then master)
#   --files-from FILE  read the change set from FILE instead of git; one path
#                      per line. For tests, so every branch is reachable
#                      without inventing commits.
#   --push-to BRANCH  this run is a push to BRANCH, not a pull request: decide
#                     full and stop, matching ci-scope.sh's own reasoning (a
#                     push to master has an empty diff against itself, and
#                     selection is a pull-request feature).
#
# Exit codes: 0 always, unless usage is wrong (64).

set -uo pipefail
export LC_ALL=C

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
run_tests="$repo_root/run-tests.sh"
base_ref=""
files_from=""
push_to=""

usage() {
    awk 'NR > 1 && /^#/ && !/^# ?(MODE|PACKAGE):/{ sub(/^# ?/, ""); print } /^set -uo/{ exit }' "$0"
    exit "${1:-64}"
}

while [ "$#" -gt 0 ]; do
    case "$1" in
        --base) [ "$#" -ge 2 ] || usage; base_ref="$2"; shift 2 ;;
        --files-from) [ "$#" -ge 2 ] || usage; files_from="$2"; shift 2 ;;
        --push-to) [ "$#" -ge 2 ] || usage; push_to="$2"; shift 2 ;;
        -h|--help) usage 0 ;;
        *) printf '%s: unknown argument: %s\n' "${0##*/}" "$1" >&2; usage ;;
    esac
done

decide() { # <scope> <reason> [tests...]
    local scope="$1" reason="$2"; shift 2
    printf 'scope=%s\n' "$scope"
    printf 'reason=%s\n' "$reason"
    printf 'tests=%s\n' "$*"
    if [ -n "${GITHUB_OUTPUT:-}" ]; then
        {
            printf 'scope=%s\n' "$scope"
            printf 'reason=%s\n' "$reason"
            printf 'tests=%s\n' "$*"
        } >> "$GITHUB_OUTPUT"
    fi
    exit 0
}

# Same reasoning as ci-scope.sh's own --push-to handling: HEAD is
# origin/master on a push to master, so the merge base is HEAD and the diff is
# empty, and selection would answer "nothing changed" on the run that most
# needs to be exhaustive.
[ -z "$push_to" ] \
    || decide full "push to $push_to: an integration branch always runs the full suite"

cd "$repo_root" 2>/dev/null || decide full "cannot enter the repository root; refusing to narrow"

# ---- the change set --------------------------------------------------------
changed=""
if [ -n "$files_from" ]; then
    [ -r "$files_from" ] || decide full "cannot read the change set from $files_from"
    changed="$(cat "$files_from")"
else
    git rev-parse --git-dir >/dev/null 2>&1 || decide full "not a git repository"
    if [ -z "$base_ref" ]; then
        for candidate in origin/master master; do
            git rev-parse --verify "$candidate" >/dev/null 2>&1 && { base_ref="$candidate"; break; }
        done
    fi
    [ -n "$base_ref" ] || decide full "no base ref resolves; refusing to narrow"
    git rev-parse --verify "$base_ref" >/dev/null 2>&1 || decide full "base ref $base_ref does not resolve"
    merge_base="$(git merge-base "$base_ref" HEAD 2>/dev/null)" \
        || decide full "no merge base with $base_ref (shallow clone?); refusing to narrow"
    [ -n "$merge_base" ] || decide full "empty merge base with $base_ref; refusing to narrow"
    changed="$(git diff --name-only "$merge_base..HEAD" 2>/dev/null)" \
        || decide full "cannot diff $merge_base..HEAD; refusing to narrow"
fi

file_count="$(printf '%s\n' "$changed" | awk 'NF' | wc -l | tr -d ' ')"
[ "$file_count" -gt 0 ] || decide full "nothing differs from ${base_ref:-the base}; nothing to narrow against"

# ---- global inputs: anything that can change what ANY test exercises ------
# The selector must not exempt itself (`.github/*`), and run-tests.sh and
# lib-test.sh are the execution machinery every test implicitly depends on
# whether or not it names them in a COVERS line.
global_hit=""
while IFS= read -r path; do
    [ -n "$path" ] || continue
    case "$path" in
        Cargo.toml|Cargo.lock|rust-toolchain.toml|flake.nix|flake.lock) global_hit="$path"; break ;;
        .github/*) global_hit="$path"; break ;;
        run-tests.sh) global_hit="$path"; break ;;
        planning/tests/lib-test.sh) global_hit="$path"; break ;;
    esac
done <<CHANGED
$changed
CHANGED
[ -z "$global_hit" ] || decide full "$global_hit changed, which the whole suite execution depends on"

if [ "$file_count" -gt 100 ]; then
    decide full "$file_count files changed, past the point where selecting pays"
fi

# ---- which declared tests are hit ------------------------------------------
# A COVERS entry hits a changed path when the path equals it or starts with
# "<entry>/" -- a directory entry covers everything under it, a file entry
# covers only itself.
covers_hits() { # <covers-line> -> 0 if any entry intersects $changed
    local entries="$1" entry
    for entry in $entries; do
        while IFS= read -r path; do
            [ -n "$path" ] || continue
            case "$path" in
                "$entry"|"$entry"/*) return 0 ;;
            esac
        done <<CHANGED2
$changed
CHANGED2
    done
    return 1
}

all_items="$("$run_tests" --list-only)" \
    || decide full "run-tests.sh --list-only failed; cannot read the canonical test list"
[ -n "$all_items" ] || decide full "run-tests.sh --list-only listed nothing; refusing to narrow"

selected=""
excluded_count=0
while IFS= read -r item; do
    [ -n "$item" ] || continue
    marker=""
    if [ -f "$repo_root/$item" ]; then
        # Within the first few lines, not a fixed line number: the shebang and
        # the MODE marker both precede it, and a file missing one of those (or
        # carrying a PACKAGE marker too) must not silently read as undeclared.
        marker="$(sed -n '1,5{/^# COVERS: /p;}' "$repo_root/$item" | head -1)"
    fi
    case "$marker" in
        '# COVERS: '*)
            if covers_hits "${marker#\# COVERS: }"; then
                selected="$selected$item
"
            else
                excluded_count=$((excluded_count + 1))
            fi
            ;;
        *)
            # Undeclared (no marker, or a crate directory with no COVERS line
            # of its own) always runs.
            selected="$selected$item
"
            ;;
    esac
done <<ITEMS
$all_items
ITEMS

selected="$(printf '%s' "$selected" | awk 'NF')"
[ -n "$selected" ] || decide full "selection excluded every test; refusing to trust an empty run"

# shellcheck disable=SC2046  # deliberate: one path per word, no globs in it
decide selective \
    "$file_count changed file(s); $excluded_count declared test(s) excluded on their own stated grounds" \
    $(printf '%s\n' "$selected" | tr '\n' ' ')
