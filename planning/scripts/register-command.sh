#!/usr/bin/env bash
# MODE: PROD
# register-command.sh — maintain a plan's command registry (commands.json).
#
# Every command literal in step instructions or testing companions must be
# registered here with its "when" context, so executors know when the command
# is appropriate and future plans do not copy it out of context. The registry
# is seeded empty by create-plan.sh; validate-plan.sh flags any unregistered
# command literal it finds.
#
# --list emits TSV (key, command, when) so a caller can parse it (§10).
#
# Usage:
#   register-command.sh [--plan-dir] <plan-directory> <key> <command> <when>
#   register-command.sh [--plan-dir] <plan-directory> --remove <key>
#   register-command.sh [--plan-dir] <plan-directory> --list
#   register-command.sh --help
#
# Requires jq (declared in install.sh's runtime_requirements()).
#
# Exit codes: 64 bad invocation, 66 no plan directory, 69 jq unavailable.

set -euo pipefail
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=planning/scripts/plan-document-lib.sh
source "$script_dir/plan-document-lib.sh"
# Accept --plan-dir as a synonym for the positional plan directory: the
# bounded reader takes the flag, so a reader who learned it there is not
# refused here.
eval "set -- $(plan_hoist_plan_dir 1 "$@")"

export LC_ALL=C


usage() {
    local rc="${1:-64}"
    cat <<USAGE
Usage: ${0##*/} [--plan-dir] <plan-directory> <key> <command> <when>
       ${0##*/} [--plan-dir] <plan-directory> --remove <key>
       ${0##*/} [--plan-dir] <plan-directory> --list
       ${0##*/} --help
USAGE
    exit "$rc"
}

mode=add
plan_dir=""
key=""
command_literal=""
when=""
positional=0
while [ "$#" -gt 0 ]; do
    case "$1" in
        -h|--help) usage 0 ;;
        --list) [ "$mode" = add ] || usage; mode=list; shift ;;
        --remove) [ "$mode" = add ] || usage; mode=remove; shift ;;
        --) shift; break ;;
        -*) printf '%s: unknown option: %s\n' "${0##*/}" "$1" >&2; usage ;;
        *)
            positional=$((positional + 1))
            case "$positional" in
                1) plan_dir="$1" ;;
                2) key="$1" ;;
                3) command_literal="$1" ;;
                4) when="$1" ;;
                *) usage ;;
            esac
            shift
            ;;
    esac
done
[ -n "$plan_dir" ] || usage
case "$mode" in
    list) [ "$positional" -eq 1 ] || usage ;;
    remove) [ "$positional" -eq 2 ] || plan_die "--remove requires exactly one <key>" ;;
    add) [ "$positional" -eq 4 ] || usage ;;
esac

# jq is a declared runtime requirement, but a hand-copied skill directory can
# still miss it; say so with the "tool unavailable" code rather than letting the
# shell's own "command not found" surface.
command -v jq >/dev/null 2>&1 || plan_die 'jq is required by register-command.sh; install jq (macOS: brew install jq, Debian: apt-get install jq)' 69

plan_require_directory "$plan_dir"
commands_file="$plan_dir/commands.json"
[ -f "$commands_file" ] || plan_die "No $commands_file; create the plan with create-plan.sh to seed an empty registry"

# The trap is installed before the first write so a jq failure cannot leak the
# temp, and is never released with `trap - EXIT`: that would discard the
# library's cleanup handler too (CODE-STYLE §8).
case "$mode" in
    list)
        jq -r 'to_entries[] | "\(.key)\t\(.value.cmd)\t\(.value.when)"' "$commands_file"
        exit 0
        ;;
    remove)
        tmp_file="$(mktemp "${TMPDIR:-/tmp}/plan-command-register.XXXXXX")"
        trap 'rm -f "$tmp_file"' EXIT
        jq --arg k "$key" 'del(.[$k])' "$commands_file" > "$tmp_file"
        mv "$tmp_file" "$commands_file"
        printf 'Removed command key %s from %s\n' "$key" "$commands_file"
        exit 0
        ;;
esac

[[ "$key" =~ ^[a-z][a-z0-9-]*$ ]] || plan_die "Command key must be kebab-case (e.g. cache-flush)"
[ -n "${command_literal//[[:space:]]/}" ] || plan_die "Command must not be empty"
[ -n "${when//[[:space:]]/}" ] || plan_die "A command must be registered with a 'when' explanation (when is it appropriate to run it?)"

tmp_file="$(mktemp "${TMPDIR:-/tmp}/plan-command-register.XXXXXX")"
trap 'rm -f "$tmp_file"' EXIT
jq --arg k "$key" --arg c "$command_literal" --arg w "$when" \
    '. + { ($k): { cmd: $c, when: $w } }' "$commands_file" > "$tmp_file"
mv "$tmp_file" "$commands_file"
printf 'Registered %s = %s (%s)\n' "$key" "$command_literal" "$when"
