#!/usr/bin/env bash
# MODE: PROD
# GENERATED FILE — do not edit. Compiled from scripts/lib/core/*.sh by:
#   planning/scripts/build-plan-libs.sh
# Edit the function file in that directory, then re-run the build.
# Target: prod
#
# failure, guards, temp files, atomic writes, plan root, snapshots

set -euo pipefail

[ -z "${PLAN_CORE_LIB_LOADED:-}" ] || return 0
PLAN_CORE_LIB_LOADED=1

# Registered temp files, removed when plan_die exits. Its own file because
# plan_die reads it and a test may source either alone.
PLAN_DIE_TEMP_FILES=()

# Write stdin to <target> atomically. The temp lives in the target's own
# directory so the rename cannot cross a filesystem, inherits the target's mode
# when it exists, and is registered with the cleanup list.
plan_atomic_write() {
    local target="$1" dir base tmp mode
    dir="$(dirname "$target")"
    base="$(basename "$target")"
    [ -d "$dir" ] || plan_die "Target directory not found: $dir" 66
    tmp="$(mktemp "$dir/.$base.XXXXXX")" || plan_die "Cannot create a temp file in: $dir" 73
    plan_track_tmp "$tmp"
    cat > "$tmp"
    if [ -e "$target" ]; then
        mode="$(plan_stat_mode "$target" 2>/dev/null || true)"
        [ -z "$mode" ] || chmod "$mode" "$tmp"
    fi
    mv -f "$tmp" "$target"
}

# The shared awk prelude defining trim(). Used as `awk "$(plan_awk_trim) …"`.
# Both ends are anchored: an unanchored `[[:space:]]+$` alternative strips
# interior whitespace runs and silently mangles table cells.
plan_awk_trim() {
    cat <<'AWK'
function trim(v) { gsub(/^[[:space:]]+|[[:space:]]+$/, "", v); return v }
AWK
}

plan_cleanup() {
    local f body
    for f in ${plan_tmp_files[@]+"${plan_tmp_files[@]}"}; do
        [ -z "$f" ] || rm -f -- "$f"
    done
    plan_tmp_files=()
    # Chain the handler that was installed before we were sourced. `trap -p`
    # prints `trap -- 'body' EXIT`; the inner eval strips the single quotes and
    # runs the body exactly once.
    if [ -n "${plan_prior_exit_trap:-}" ]; then
        body="${plan_prior_exit_trap#trap -- }"
        body="${body% EXIT}"
        plan_prior_exit_trap=""
        eval "eval $body"
    fi
}

plan_decode_escaped_newlines() {
    local value="$1"
    printf '%s' "${value//\\n/$'\n'}"
}

plan_default_root() {
    if [ -n "${PLANS_ROOT:-}" ]; then
        printf '%s\n' "${PLANS_ROOT%/}"
        return 0
    fi
    local home_dir="${HOME:-}"
    if [ -z "$home_dir" ] && [ -n "${USERPROFILE:-}" ]; then
        home_dir="$USERPROFILE"
    fi
    if [ -z "$home_dir" ] && [ -n "${HOMEDRIVE:-}${HOMEPATH:-}" ]; then
        home_dir="${HOMEDRIVE:-}${HOMEPATH:-}"
    fi
    [ -n "$home_dir" ] || plan_die "Unable to resolve the user home directory; set PLANS_ROOT"
    printf '%s/.plans\n' "${home_dir%/}"
}

plan_die() {
    printf '%s: %s\n' "${0##*/}" "$1" >&2
    rm -f ${PLAN_DIE_TEMP_FILES[@]+"${PLAN_DIE_TEMP_FILES[@]}"}
    exit "${2:-64}"
}

# Every duplicated step number in a plan, as `<goal> <number> <file> <file>...`,
# one line per collision. Empty output means the plan is clean.
#
# Two steps numbered 04 in one goal have no defined order: steps are read in
# number order and nothing breaks the tie. add-work-unit.sh refuses to create
# one, so this finds plans that already hold the state -- created before that
# guard, or by an editor working outside the helpers.
#
# Deliberately silent about gaps. A goal numbered 01, 04, 07 is unambiguous, so a
# missing number is not a defect and warning about it would train the reader to
# skip these lines.
plan_duplicate_step_numbers() {
    local plan_dir="$1" goal_dir goal steps_dir
    [ -d "$plan_dir" ] || return 0
    for goal_dir in "$plan_dir"/*/; do
        [ -d "$goal_dir" ] || continue
        steps_dir="${goal_dir}steps"
        [ -d "$steps_dir" ] || continue
        goal="$(basename "$goal_dir")"
        # The companion shares its step's number by design, so it is excluded
        # before counting rather than reported as a collision.
        ls "$steps_dir" 2>/dev/null \
            | grep -E '^[0-9][0-9]-step-.*\.md$' \
            | grep -v -- '-testing\.md$' \
            | awk -v goal="$goal" '
                { number = substr($0, 1, 2); files[number] = files[number] " " $0; seen[number]++ }
                END {
                    for (number in seen) {
                        if (seen[number] > 1) print goal " " number files[number]
                    }
                }' \
            | LC_ALL=C sort
    done
}

plan_ensure_root_permissions() {
    local root="${1:-$(plan_default_root)}" helper_dir="${2:-$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)}"
    local probe
    mkdir -p "$root" || plan_die "Cannot create plan root: $root"
    [ -d "$root" ] && [ -r "$root" ] && [ -w "$root" ] && [ -x "$root" ] \
        || plan_die "Plan root is not readable, writable, and searchable: $root"
    probe="$root/.permission-probe.$$"
    ( : > "$probe" && rm -f "$probe" ) || plan_die "Plan root does not permit file editing: $root"
    [ -d "$helper_dir" ] && [ -r "$helper_dir" ] && [ -x "$helper_dir" ] \
        || plan_die "Planning helper directory is not readable/searchable: $helper_dir"
    # find(1) exits 0 regardless of what -exec returns, so `-exec test -r {} \;`
    # cannot fail the check; test in the shell. Libraries are sourced, never
    # executed (CODE-STYLE §3), so only readability is required of them.
    local helper
    while IFS= read -r helper; do
        [ -n "$helper" ] || continue
        [ -r "$helper" ] \
            || plan_die "One or more planning helpers cannot be read and executed: $helper_dir"
        case "$helper" in
            *-lib.sh) ;;
            *) [ -x "$helper" ] \
                || plan_die "One or more planning helpers cannot be read and executed: $helper_dir" ;;
        esac
    done < <(find "$helper_dir" -maxdepth 1 -type f -name '*.sh' -print)
    printf '%s\n' "$root"
}

# Non-fatal accumulators for checkers that report every finding. plan_fail
# bumps plan_error_count; the caller exits 1 at the end when it is non-zero.
# Neither ever exits: a sourced function must leave that decision to its caller.
plan_fail() {
    plan_error_count=$((plan_error_count + 1))
    printf 'FAIL: %s\n' "$*" >&2
}

# Commit the plan before a mutation so every overwrite is recoverable. The
# snapshot lands in the repository PLAN_SNAPSHOT_REPO names, which is usually
# the plans root rather than the plan directory, and the add is scoped to this
# plan so a shared plans-root repo does not sweep up its siblings. No-op without
# git, without a pinned repository, or when that repository is not initialized.
plan_git_snapshot() {
    local plan_dir="$1" repo
    command -v git >/dev/null 2>&1 || return 0
    # Resolve before use: the pathspec below runs with git's cwd set to $repo,
    # so a caller's relative path would be read against the wrong directory and
    # the add would silently match nothing.
    plan_dir="$(cd "$plan_dir" 2>/dev/null && pwd -P)" || return 0
    [ -n "$plan_dir" ] || return 0
    repo="$(plan_snapshot_repo "$plan_dir")" || return 0
    [ -d "$repo/.git" ] || return 0
    git -C "$repo" add -A -- "$plan_dir" >/dev/null 2>&1 || return 0
    git -C "$repo" -c user.name='plan-skill' -c user.email='plan-skill@localhost' \
        commit -q -m "snapshot before ${0##*/}" >/dev/null 2>&1 || true
}

# Accept --plan-dir <path> wherever the plan directory is positional, because
# plan-context.sh and run-adversary-probe.sh take the flag and a reader who
# learned it there should not have a call refused elsewhere. Prints the argument
# list %q-quoted with the flag's value moved to <position>, so a caller does
# `eval "set -- $(plan_hoist_plan_dir 1 "$@")"` and parses as before.
plan_hoist_plan_dir() {
    local position="$1" hoisted="" arg
    shift
    local rest=()
    while [ "$#" -gt 0 ]; do
        case "$1" in
            --plan-dir)
                [ "$#" -ge 2 ] || plan_die "--plan-dir needs a path"
                hoisted="$2"; shift 2 ;;
            --plan-dir=*)
                hoisted="${1#--plan-dir=}"; shift ;;
            *) rest+=("$1"); shift ;;
        esac
    done
    if [ -n "$hoisted" ]; then
        local out=() i=1
        # PORTABILITY(empty-array-setu)
        for arg in ${rest[@]+"${rest[@]}"}; do
            [ "$i" -ne "$position" ] || out+=("$hoisted")
            out+=("$arg"); i=$((i + 1))
        done
        [ "$i" -gt "$position" ] || out+=("$hoisted")
        rest=(${out[@]+"${out[@]}"})
    fi
    # PORTABILITY(empty-array-setu)
    for arg in ${rest[@]+"${rest[@]}"}; do printf '%q ' "$arg"; done
    printf '\n'
}

plan_refuse_existing() {
    [ ! -e "$1" ] || plan_die "Refusing to overwrite an existing artifact: $1" 73
}

plan_register_temp_file() {
    PLAN_DIE_TEMP_FILES+=("$1")
}

# Refuse to run under a bash older than <major>. NOT called at load time — this
# library itself is 3.2-clean; a script that genuinely needs bash 4 calls it.
plan_require_bash() {
    local want="$1"
    [ "${BASH_VERSINFO[0]}" -ge "$want" ] || plan_die \
        "needs bash $want or newer (running ${BASH_VERSION:-unknown}); on macOS: brew install bash" 78
}

plan_require_directory() {
    [ -d "$1" ] || plan_die "Plan directory not found: $1" 66
}

# Require an input file; refuse to clobber an existing artifact. Same exit-code
# vocabulary as plan_require_directory: 66 for "missing input", 73 for
# "already there, not overwriting".
plan_require_file() {
    [ -f "$1" ] || plan_die "File not found: $1" 66
}

plan_require_safe_value() {
    local label="$1" value="$2"
    [ -n "$value" ] || plan_die "$label must not be empty"
    [[ "$value" != *'|'* ]] || plan_die "$label must not contain a Markdown table separator (|)"
    [[ "$value" != *$'\n'* && "$value" != *$'\r'* ]] || plan_die "$label must be one line"
}

# Resolve a symlink chain without `readlink -f` (GNU; macOS only since 12.3).
# Relative targets resolve against the link's own directory; a non-symlink is
# echoed back. The 32-hop cap turns a cycle into a diagnosed failure.
plan_resolve_symlink() {
    local path="$1" hops=0 target
    while [ -L "$path" ]; do
        hops=$((hops + 1))
        [ "$hops" -le 32 ] || plan_die "symlink chain exceeds 32 hops (cycle?): $1" 66
        target="$(readlink "$path")"
        case "$target" in
            /*) path="$target" ;;
            *) path="$(dirname "$path")/$target" ;;
        esac
    done
    printf '%s\n' "$path"
}

# The repository that owns this plan's snapshots, as create-plan.sh pinned it in
# PLAN_SNAPSHOT_REPO. Returns 1 when there is none, which is the honest answer
# for a plan versioned in a repository the user owns: plan snapshots do not
# belong in that history. The value is read, not re-derived, so a .gitignore
# change after creation cannot move the target silently.
plan_snapshot_repo() {
    local env_file="$1/.env" assignment repo
    [ -f "$env_file" ] || return 1
    assignment="$(grep '^PLAN_SNAPSHOT_REPO=' "$env_file" 2>/dev/null)" || return 1
    # Same character rule plan-env.sh manifest_check enforces on values: a
    # manifest carrying any of these is already invalid, so refuse it here too
    # rather than eval the line.
    case "$assignment" in
        *'$'*|*'`'*|*';'*|*'|'*|*'&'*|*'<'*|*'>'*) return 1 ;;
    esac
    # eval is the inverse of the printf %q that wrote the value, and is what
    # keeps a plans root containing a space working.
    eval "repo=${assignment#PLAN_SNAPSHOT_REPO=}" 2>/dev/null || return 1
    [ -n "$repo" ] || return 1
    printf '%s\n' "$repo"
}

# ── Temp-file bookkeeping (CODE-STYLE §8) ────────────────────────────────────
# bash keeps exactly one EXIT handler: a script installing its own after
# sourcing this library replaces plan_cleanup, and `trap - EXIT` clears the slot.
plan_track_tmp() {
    plan_tmp_files=(${plan_tmp_files[@]+"${plan_tmp_files[@]}"} "$1")
}

plan_warn() {
    printf 'WARN: %s\n' "$*" >&2
}

# Ensure the planning scratch directory exists for this boot. Failure is
# ignored: the helpers still work when a nonstandard TMPDIR is unwritable.
planning_ensure_tmpdir() {
    local d
    d="$(planning_tmpdir)"
    mkdir -p "$d" 2>/dev/null && chmod 700 "$d" 2>/dev/null || true
}

# Scratch directory the planning skill may write temporary capsules and run
# artifacts into. It lives under the system temp dir so it is fresh per boot;
# the agent's existing write access to the temp dir suffices to create it.
planning_tmpdir() {
    printf '%s\n' "${TMPDIR:-/tmp}/planning-agent"
}
