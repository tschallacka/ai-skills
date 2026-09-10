#!/usr/bin/env bash
# MODE: PROD
# verify-skill-load.sh — T86: the command that decides whether a part of the
# planning skill was actually read, not merely reported as read.
#
# Every generated planning/parts/part-N.md carries a load-sanity line at a
# position generate-skill-docs.sh chose from the part's own content, in its
# last fifth: `<!-- SKILL-LOAD-PROOF part=<N> token=<hex> -->`. This command
# reads that SAME file itself, from disk, and compares its OWN reading of the
# current token against the one the caller supplies. An instruction to
# report a token is claimable without having read that far; an argument
# this command independently verifies against the real file is not — that
# distinction is the whole reason the check is a command and not a line of
# prose asking the agent to say a number.
#
# A missing or mismatched token means one of: the part was not read that
# far (a harness silently truncated it — see
# .agents/knowledge/agent-read-limits.md), a stale token was recalled from a
# previous, since-regenerated version of the part, or the part name is wrong.
# Re-read the current part and try again with what it actually says.
#
# Usage:
#   verify-skill-load.sh --part <name> --token <hex> [<skill-directory>]
#   verify-skill-load.sh --help
#
# Exit codes: 64 = bad usage; 65 = the part carries no load-proof line at all
# (a stale generation, or generate-skill-docs.sh has not been run since the
# part was added); 66 = no such part file; 1 = the token does not match.
set -euo pipefail
export LC_ALL=C

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

usage() {
    local rc="${1:-64}"
    cat <<USAGE
Usage: ${0##*/} --part <name> --token <hex> [<skill-directory>]
       ${0##*/} --help
USAGE
    exit "$rc"
}

part='' token='' skill_dir=''
while [ "$#" -gt 0 ]; do
    case "$1" in
        -h|--help) usage 0 ;;
        --part) [ "$#" -ge 2 ] || usage; part="$2"; shift 2 ;;
        --token) [ "$#" -ge 2 ] || usage; token="$2"; shift 2 ;;
        -*) usage ;;
        *)
            [ -z "$skill_dir" ] || usage
            skill_dir="$1"
            shift
            ;;
    esac
done
[ -n "$part" ] && [ -n "$token" ] || usage
skill_dir="${skill_dir:-$(cd "$script_dir/.." && pwd)}"

part_file="$skill_dir/parts/$part.md"
[ -f "$part_file" ] || {
    printf 'no such part: %s (looked for %s)\n' "$part" "$part_file" >&2
    exit 66
}

actual="$(grep -oE "SKILL-LOAD-PROOF part=$part token=[0-9a-f]+" "$part_file" \
    | sed 's/.*token=//' | head -1)"
[ -n "$actual" ] || {
    printf '%s carries no load-proof line; run generate-skill-docs.sh\n' "$part_file" >&2
    exit 65
}

if [ "$token" = "$actual" ]; then
    printf 'verified: %s was read at least as far as its load-sanity line\n' "$part"
    exit 0
fi
printf 'refused: the token for %s does not match — you have not finished\n' "$part" >&2
printf 'reading it, or you are recalling a token from a version that has since\n' >&2
printf 'regenerated. Re-read %s and find the current line.\n' "${part_file#"$skill_dir"/}" >&2
exit 1
