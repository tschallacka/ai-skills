#!/usr/bin/env bash
# MODE: DEV
# ci-scope.sh — decide how much of the workspace a CI run has to build.
#
# Prints three lines, and writes the same to $GITHUB_OUTPUT when it is set:
#
#   scope=full|selective|none
#   reason=<one line saying why>
#   crates=<space-separated crate names, empty unless scope=selective>
#
# full       build and test every workspace member on every target
# selective  build and test only the named crates
# none       no crate needs building; the shell and packaging gates still run
#
# THE DEFAULT IS ALWAYS full. Every branch that cannot prove a smaller scope is
# correct returns full, including every error path: no merge base, a shallow
# clone, cargo metadata failing, an unreadable change set. A selector that
# narrows the run when it is confused is worse than no selector, because the
# green tick then means "we did not look" while reading as "we looked".
#
# Selection is crate changes UNIONED WITH THEIR DEPENDENTS. Building only the
# changed crate is wrong: chat-proto has two dependents, planning-core has
# twenty-four, and a library change that compiles in isolation can still break
# every consumer. The reverse edges come from cargo metadata, so the graph is
# the real one rather than a list someone maintains by hand.
#
# Usage:
#   ci-scope.sh [--base REF] [--files-from FILE] [--threshold N]
#   ci-scope.sh --help
#
#   --base REF        what to diff against (default: origin/master, then master)
#   --files-from FILE  read the change set from FILE instead of git; one path
#                      per line. For tests, so every branch is reachable
#                      without inventing commits.
#   --threshold N     override the derived threshold (see below)
#
# THE THRESHOLD IS DERIVED, NOT A CONSTANT. A closure bigger than a quarter of
# the workspace goes full: ceil(members / 4), with a floor of 5 so a small
# workspace does not end up with a threshold of 1.
#
# A fixed number would rot. When this was written the workspace had 78 members
# and the closure sizes were 25, 13, 10, 10, 7, 6, 5, 3, then 2 for eleven more
# libraries and 1 for the 69 leaf crates. Any hand-picked value between 10 and
# 13 behaved identically, and so did anything from 13 to 24 — so the number
# looked meaningful while being arbitrary inside a gap, and would have silently
# changed meaning as crates were added.
#
# The obvious alternative is to find the widest gap in that distribution and
# split there, which is what a person does by eye. It is rejected deliberately:
# the widest gap MOVES. One new crate with a mid-sized closure relocates it, and
# the policy flips with no edit and no announcement. A ratio is monotonic — it
# only ever moves when the workspace size moves, and it moves predictably.
#
# The ratio is a cap on RISK as much as on cost. Pure economics would put the
# crossover much higher, since the fixed overhead of a run is paid either way;
# but a large closure means a broad change, and a broad change is exactly where
# selection is most likely to miss a path cargo cannot see — a shell script, a
# generated library, a packaged file. Capping well below the cost crossover
# buys back that uncertainty.
#
# Exit codes: 0 always, unless usage is wrong (64). A decision is not a failure.

set -uo pipefail
export LC_ALL=C

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
base_ref=""
files_from=""
# Empty means derive it from the workspace size once cargo metadata is read.
threshold="${CI_SCOPE_THRESHOLD:-}"
threshold_source="derived"
# ceil(members / CI_SCOPE_DIVISOR), never below CI_SCOPE_FLOOR.
divisor="${CI_SCOPE_DIVISOR:-4}"
floor="${CI_SCOPE_FLOOR:-5}"

usage() {
    awk 'NR > 1 && /^#/{ sub(/^# ?/, ""); print } /^set -uo/{ exit }' "$0"
    exit "${1:-64}"
}

while [ "$#" -gt 0 ]; do
    case "$1" in
        --base) [ "$#" -ge 2 ] || usage; base_ref="$2"; shift 2 ;;
        --files-from) [ "$#" -ge 2 ] || usage; files_from="$2"; shift 2 ;;
        --threshold) [ "$#" -ge 2 ] || usage; threshold="$2"; threshold_source="given"; shift 2 ;;
        -h|--help) usage 0 ;;
        *) printf '%s: unknown argument: %s\n' "${0##*/}" "$1" >&2; usage ;;
    esac
done

# An unusable override is discarded rather than honoured: a threshold of
# "abc" must not become 0 and silently make every run full, nor become huge and
# silently make every run selective. Empty means derive.
case "$threshold" in
    '')       threshold_source="derived" ;;
    *[!0-9]*) threshold=""; threshold_source="derived (ignored an unusable override)" ;;
    *)        [ "$threshold_source" = "given" ] || threshold_source="CI_SCOPE_THRESHOLD" ;;
esac
case "$divisor" in ''|*[!0-9]*|0) divisor=4 ;; esac
case "$floor" in ''|*[!0-9]*) floor=5 ;; esac

decide() { # <scope> <reason> [crates...]
    local scope="$1" reason="$2"; shift 2
    printf 'scope=%s\n' "$scope"
    printf 'reason=%s\n' "$reason"
    printf 'crates=%s\n' "$*"
    if [ -n "${GITHUB_OUTPUT:-}" ]; then
        {
            printf 'scope=%s\n' "$scope"
            printf 'reason=%s\n' "$reason"
            printf 'crates=%s\n' "$*"
        } >> "$GITHUB_OUTPUT"
    fi
    exit 0
}

cd "$repo_root" 2>/dev/null || decide full "cannot enter the repository root; refusing to narrow"

# ---- the change set --------------------------------------------------------
# A PR gives us its base; otherwise master. The MERGE BASE, not the ref itself,
# so a master that moved ahead does not present its own commits as this
# branch's work — the same mistake pre-push-check carried until today.
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
[ "$file_count" -gt 0 ] || decide none "nothing differs from ${base_ref:-the base}"

# ---- global inputs: anything that can change every crate's meaning --------
# A hit here is not a heuristic. The root manifest defines the members and the
# release profile; the lock fixes every dependency version; the toolchain file
# and the flake fix the compiler; the workflows are the thing being trusted, so
# a change to them must be exercised in full; the installer and package.json
# decide what ships, which the packaging gates read.
global_hit=""
while IFS= read -r path; do
    [ -n "$path" ] || continue
    case "$path" in
        Cargo.toml|Cargo.lock|rust-toolchain.toml|flake.nix|flake.lock) global_hit="$path"; break ;;
        .github/workflows/*) global_hit="$path"; break ;;
        install.sh|package.json) global_hit="$path"; break ;;
        installer/*) global_hit="$path"; break ;;
    esac
done <<CHANGED
$changed
CHANGED
[ -z "$global_hit" ] || decide full "$global_hit changed, which can alter every crate"

if [ "$file_count" -gt 100 ]; then
    decide full "$file_count files changed, past the point where selecting pays"
fi

# ---- which crates changed -------------------------------------------------
changed_crates="$(printf '%s\n' "$changed" \
    | awk -F/ '$1 == "src" && NF >= 2 && $2 != "" { print $2 }' \
    | sort -u)"
[ -n "$changed_crates" ] || decide none "no crate under src/ differs from ${base_ref:-the base}"

# ---- the reverse-dependency closure ---------------------------------------
# cargo metadata is the authority on the graph. If it cannot be read the scope
# cannot be trusted, so that is a full run rather than a guess.
command -v cargo >/dev/null 2>&1 || decide full "cargo is not on PATH; cannot compute the dependency graph"
metadata="$(cargo metadata --format-version 1 --no-deps 2>/dev/null)" \
    || decide full "cargo metadata failed; cannot compute the dependency graph"
[ -n "$metadata" ] || decide full "cargo metadata produced nothing; cannot compute the dependency graph"

# member -> the members that depend on it, one "dependent dependency" pair per line
edges="$(printf '%s' "$metadata" | rjq -r '
    [.packages[].name] as $members
    | .packages[]
    | .name as $from
    | .dependencies[]
    | select(.name as $d | $members | index($d))
    | "\($from) \(.name)"' 2>/dev/null)" \
    || decide full "cannot read the workspace graph from cargo metadata"

members_total="$(printf '%s' "$metadata" | rjq -r '.packages|length' 2>/dev/null || echo 0)"

# Transitive closure upward: start with the changed crates, repeatedly add any
# crate that depends on something already selected, until nothing new appears.
# shellcheck disable=SC2086  # deliberate: one crate name per word, no globs
selected="$(printf '%s\n' $changed_crates | sort -u)"
while : ; do
    added="$(
        printf '%s\n' "$edges" | awk -v sel="$selected" '
            BEGIN { n = split(sel, s, "\n"); for (i = 1; i <= n; i++) if (s[i] != "") have[s[i]] = 1 }
            NF == 2 && ($2 in have) && !($1 in have) { print $1 }
        ' | sort -u
    )"
    [ -n "$added" ] || break
    selected="$(printf '%s\n%s\n' "$selected" "$added" | awk 'NF' | sort -u)"
done

selected_count="$(printf '%s\n' "$selected" | awk 'NF' | wc -l | tr -d ' ')"
changed_count="$(printf '%s\n' "$changed_crates" | awk 'NF' | wc -l | tr -d ' ')"

# Derive the threshold from the workspace this run is actually deciding about.
# Integer ceil, so it never rounds down into a stricter policy than the ratio
# asks for.
if [ -z "$threshold" ]; then
    case "$members_total" in
        ''|*[!0-9]*|0) decide full "cannot read the workspace member count; refusing to narrow" ;;
    esac
    threshold=$(( (members_total + divisor - 1) / divisor ))
    [ "$threshold" -ge "$floor" ] || threshold="$floor"
    threshold_label="$threshold = ceil($members_total/$divisor), floor $floor"
else
    threshold_label="$threshold from $threshold_source"
fi

if [ "$selected_count" -gt "$threshold" ]; then
    decide full "$changed_count changed crate(s) fan out to $selected_count of $members_total, past the threshold ($threshold_label)"
fi

# shellcheck disable=SC2046  # deliberate: the closure becomes one argument per crate
decide selective \
    "$changed_count changed crate(s) plus dependents = $selected_count of $members_total, within the threshold ($threshold_label)" \
    $(printf '%s\n' "$selected" | awk 'NF' | tr '\n' ' ')
