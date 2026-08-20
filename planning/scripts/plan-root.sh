#!/usr/bin/env bash
# MODE: PROD
# Resolve (and, on first use in a project, choose) the plans root.
# This is the single decision point behind PLANS_ROOT so that the first plan
# created in a project asks the human where plans should live, then remembers
# the answer for every later plan in that project.
#
# Usage:
#   plan-root.sh resolve [directory]      # print the resolved plans root
#   plan-root.sh project-root [directory] # print the project (git) root if any
#
# Rules (in priority order):
#   1. If PLANS_ROOT is already exported, it wins (no prompt; automation).
#   2. If <project>/.plans exists and is "consistent" (its .env records that
#      .plans as the plans root), return it as the default (no prompt).
#   3. If a directory matching a global format for this project already exists
#      (~/.plans/<owner>/<repo> or ~/.plans/<user>/<projectdir>), return it as
#      the recognized root (no prompt). No marker file is written; recognition
#      is purely by matching the directory format.
#   4. Otherwise this is the first plan in the project: when run on an
#      interactive terminal ask the human whether to store globally under
#      ~/.plans or in the project's ./.plans; when non-interactive, default to
#      project storage and print a note.

set -euo pipefail
export LC_ALL=C

usage() {
    local rc="${1:-64}"
    cat <<USAGE
Usage: ${0##*/} resolve [directory]
       ${0##*/} project-root [directory]
       ${0##*/} --help
USAGE
    exit "$rc"
}

die() {
    printf 'plan-root: %s\n' "$*" >&2
    exit 1
}

ask() {
    local prompt="$1"
    printf '%s' "$prompt" >&2
    IFS= read -r REPLY || REPLY=""
}

confirm() {
    local prompt="$1"
    ask "$prompt [y/N] "
    case "$REPLY" in
        y|Y|yes|YES) return 0 ;;
        *) return 1 ;;
    esac
}

interactive() {
    [ -t 0 ]
}

project_root_for() {
    local dir="${1:-$PWD}"
    [ -d "$dir" ] || die "directory does not exist: $dir"
    local top
    if command -v git >/dev/null 2>&1 \
        && top="$(git -C "$dir" rev-parse --show-toplevel 2>/dev/null)"; then
        [ -n "$top" ] && { (cd "$top" && pwd -P); return 0; }
    fi
    (cd "$dir" && pwd -P)
}

home_plans() {
    local home_dir="${HOME:-}"
    [ -n "$home_dir" ] || home_dir="${USERPROFILE:-}"
    [ -n "$home_dir" ] || die "unable to resolve home directory; set PLANS_ROOT"
    printf '%s/.plans\n' "${home_dir%/}"
}

project_has_consistent_plans() {
    # Separate `local` statements: in `local a="$1" b="$a"` bash creates every
    # name first, so $a is still empty when b is assigned and candidate became
    # "/.plans" — rule 2 (an existing consistent project .plans) never fired.
    local project="$1" env_file
    local candidate="$project/.plans"
    [ -d "$candidate" ] || return 1
    env_file="$candidate/.env"
    [ -f "$env_file" ] || return 1
    # grep -F: fixed string, so no escaping of the candidate path.
    grep -qF -- "PLANS_ROOT=$candidate" "$env_file"
}

# Candidate global roots for this project, in format-match order:
#   ~/.plans/<owner>/<repo>  (from the git remote, when it exists)
#   ~/.plans/<user>/<projectdir>
global_scoped_root() {
    local project="$1" home_plans owner="" repo="" base user
    home_plans="$(home_plans)"
    base="$(basename "$project")"
    if command -v git >/dev/null 2>&1; then
        local remote
        remote="$(git -C "$project" remote get-url origin 2>/dev/null || true)"
        # `owner` must capture the WHOLE namespace path: collapsing a GitLab
        # group/subgroup would collide two repos of the same name.
        # ---- quoted: remote URL forms ----
        # https://host/owner/name.git
        # git@host:owner/name.git
        # ssh://host/owner/name.git
        # ---- end quoted ----
        if [[ "$remote" =~ (ssh://[^/]+/|git@[^:]+:|https?://[^/]+/)(.*)/([^/.]+)(\.git)?$ ]]; then
            owner="${BASH_REMATCH[2]}"
            repo="${BASH_REMATCH[3]}"
        fi
    fi
    if [ -n "$owner" ] && [ -n "$repo" ] && [ -d "$home_plans/$owner/$repo" ]; then
        printf '%s/%s/%s\n' "$home_plans" "$owner" "$repo"
        return 0
    fi
    user="${USER:-$(id -un)}"
    printf '%s/%s/%s\n' "$home_plans" "$user" "$base"
}

add_to_gitignore() {
    local gitignore="$1" entry="$2"
    if [ ! -f "$gitignore" ]; then
        printf '%s\n' "$entry" > "$gitignore"
        return 0
    fi
    grep -Fqx -- "$entry" "$gitignore" || printf '%s\n' "$entry" >> "$gitignore"
}

choose_root() {
    local project="$1" piped=""
    if interactive; then
        ask "Store plans globally under ~/.plans, or in this project's ./.plans? [g/p] (default: p) "
        case "$REPLY" in
            g|G|global) printf '%s\n' "$(global_scoped_root "$project")"; return 0 ;;
            p|P|project|"") printf '%s\n' "$project/.plans"; return 0 ;;
            *) printf 'plan-root: invalid choice (%s); using project storage\n' "$REPLY" >&2
               printf '%s\n' "$project/.plans"; return 0 ;;
        esac
    fi
    # Non-interactive: accept one piped answer token (g|p for storage, y/n for
    # the gitignore follow-up). A piped "no" means project storage, no
    # gitignore — matching what an agent can actually answer.
    if IFS= read -r piped; then :; fi
    case "$piped" in
        g|G|global) printf '%s\n' "$(global_scoped_root "$project")"; return 0 ;;
    esac
    case "$piped" in
        y|Y|yes) PIPED_GITIGNORE=yes ;;
        n|N|no) PIPED_GITIGNORE=no ;;
    esac
    printf 'plan-root: automated run detected; defaulting to project storage (%s/.plans)%s\n' "$project" "$([ -n "$piped" ] && printf ' (piped answer: %s)' "$piped")" >&2
    printf '%s\n' "$project/.plans"
}

resolve() {
    local project root scoped
    project="$(project_root_for "${1:-$PWD}")"

    if [ -n "${PLANS_ROOT:-}" ]; then
        printf '%s\n' "${PLANS_ROOT%/}"
        return 0
    fi
    if project_has_consistent_plans "$project"; then
        printf '%s\n' "$project/.plans"
        return 0
    fi
    # Recognize an already-chosen global root purely by directory format.
    scoped="$(global_scoped_root "$project")"
    if [ -d "$scoped" ]; then
        printf '%s\n' "$scoped"
        return 0
    fi

    root="$(choose_root "$project")"
    [ -n "$root" ] || die "no plans root chosen"

    if [ "$root" = "$project/.plans" ]; then
        if [ -n "${PIPED_GITIGNORE:-}" ]; then
            case "$PIPED_GITIGNORE" in
                yes) add_to_gitignore "$project/.gitignore" "/.plans" ;;
            esac
        elif interactive; then
            if confirm "Add /.plans to this project's .gitignore?"; then
                add_to_gitignore "$project/.gitignore" "/.plans"
            fi
        fi
    fi

    printf '%s\n' "$root"
}

case "${1:-}" in
    -h|--help) usage 0 ;;
    resolve) resolve "${2:-$PWD}" ;;
    project-root) project_root_for "${2:-$PWD}" ;;
    *) usage ;;
esac
