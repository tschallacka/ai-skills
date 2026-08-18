#!/usr/bin/env bash
set -euo pipefail

# register-command.sh — maintain a plan's command registry (commands.json).
#
# Every command literal in step instructions or testing companions must be
# registered here with its "when" context, so executors know when the command
# is appropriate and future plans do not copy it out of context. The registry
# is seeded empty by create-plan.sh; validate-plan.sh flags any unregistered
# command literal it finds.
#
# Usage:
#   register-command.sh <plan-directory> <key> <command> <when>
#   register-command.sh <plan-directory> --remove <key>
#   register-command.sh <plan-directory> --list

if [ "$#" -lt 1 ]; then
    printf 'Usage: %s <plan-directory> <key> <command> <when>\n' "$(basename "$0")" >&2
    printf '       %s <plan-directory> --remove <key> | --list\n' "$(basename "$0")" >&2
    exit 64
fi
case "$1" in
    -h|--help)
        printf 'Usage: %s <plan-directory> <key> <command> <when>\n' "$(basename "$0")" >&2
        printf '       %s <plan-directory> --remove <key> | --list\n' "$(basename "$0")" >&2
        exit 0
        ;;
esac

plan_dir="$1"; shift
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$script_dir/plan-document-lib.sh"

plan_require_directory "$plan_dir"
commands_file="$plan_dir/commands.json"
[ -f "$commands_file" ] || plan_die "No $commands_file; create the plan with create-plan.sh to seed an empty registry"

case "${1:-}" in
    --list)
        jq -r 'to_entries[] | "\(.key)\t\(.value.cmd)\t\(.value.when)"' "$commands_file"
        exit 0
        ;;
    --remove)
        [ "$#" -eq 2 ] || plan_die "--remove requires exactly one <key>"
        key="$2"
        jq --arg k "$key" 'del(.[$k])' "$commands_file" > "$commands_file.tmp.$$" && mv "$commands_file.tmp.$$" "$commands_file"
        printf 'Removed command key %s from %s\n' "$key" "$commands_file"
        exit 0
        ;;
esac

[ "$#" -eq 3 ] || plan_die "Usage: register-command.sh <plan-directory> <key> <command> <when>"
key="$1"; command="$2"; when="$3"
[[ "$key" =~ ^[a-z][a-z0-9-]*$ ]] || plan_die "Command key must be kebab-case (e.g. cache-flush)"
[ -n "${command//[[:space:]]/}" ] || plan_die "Command must not be empty"
[ -n "${when//[[:space:]]/}" ] || plan_die "A command must be registered with a 'when' explanation (when is it appropriate to run it?)"

tmp_file="$(mktemp "${TMPDIR:-/tmp}/plan-command-register.XXXXXX")"
trap 'rm -f "$tmp_file"' EXIT
jq --arg k "$key" --arg c "$command" --arg w "$when" \
    '. + { ($k): { cmd: $c, when: $w } }' "$commands_file" > "$tmp_file"
mv "$tmp_file" "$commands_file"
trap - EXIT
printf 'Registered %s = %s (%s)\n' "$key" "$command" "$when"
