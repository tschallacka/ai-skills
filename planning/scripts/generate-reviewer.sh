#!/usr/bin/env bash
# MODE: PROD
# generate-reviewer.sh — project the marked SKILL.md sections into REVIEWER.md.
#
# Copies each `<!-- REVIEWER_SECTION:START <name> -->` … `:END` block out of the
# source skill into a compact reviewer contract, prefixed with the reviewer
# profile version and the source's SHA-256. The recorded hash is what
# test-reviewer-projection.sh compares, so neither the projection nor the hash
# format may change without regenerating REVIEWER.md.
#
# Usage:
#   generate-reviewer.sh [<skill-directory>] [<output-file>]
#   generate-reviewer.sh --help
#
# Defaults: the skill directory is this script's parent, the output is
# <skill-directory>/REVIEWER.md.
#
# Exit codes: 65 = a reviewer section is missing, duplicated, or empty;
# 66 = the source skill is absent; 69 = no SHA-256 implementation (the
# plan-crypt binary, sha256sum, or shasum).

set -euo pipefail
export LC_ALL=C

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# Only for plan_sha256_hex: the recorded source digest goes through the one
# SHA-256 chain in the skill rather than a fourth private copy of the probe.
# shellcheck source=planning/scripts/plan-crypt-lib.sh
source "$script_dir/plan-crypt-lib.sh"

usage() {
    local rc="${1:-64}"
    cat <<USAGE
Usage: ${0##*/} [<skill-directory>] [<output-file>]
       ${0##*/} --help
USAGE
    exit "$rc"
}

case "${1:-}" in
    -h|--help) usage 0 ;;
    -*) printf '%s: unknown option: %s\n' "${0##*/}" "$1" >&2; usage ;;
esac
[ "$#" -le 2 ] || usage

skill_dir="${1:-$(cd "$script_dir/.." && pwd)}"
source_file="$skill_dir/SKILL.md"
output="${2:-$skill_dir/REVIEWER.md}"

expected_sections=(mandatory-review bounded-context)
# Parsed by test-adversary-probe-fixture.sh with an anchored ^NAME="..."$ regex;
# the name and the assignment layout are part of that contract.
REVIEWER_PROFILE_VERSION="1.4.2"

if [ ! -f "$source_file" ]; then
    printf 'source skill not found: %s\n' "$source_file" >&2
    exit 66
fi

source_hash="$(plan_sha256_hex < "$source_file")" || {
    printf 'no SHA-256 implementation available (need %s)\n' "$(plan_sha256_chain)" >&2
    exit 69
}
temp_output="$(mktemp "${TMPDIR:-/tmp}/reviewer.XXXXXX")"
temp_section="$(mktemp "${TMPDIR:-/tmp}/reviewer-section.XXXXXX")"
trap 'rm -f "$temp_output" "$temp_section"' EXIT

{
    # REVIEWER.md is a runtime artifact the reviewer reads, and it ships. HTML
    # comments rather than '# MODE:', which would render as a heading and give
    # the file a second top-level one.
    printf '<!-- MODE: PROD -->\n'
    printf '# Reviewer contract\n\n'
    printf '> Generated from `%s` by `scripts/generate-reviewer.sh`.\n' \
        "${source_file#"$skill_dir"/}"
    printf '> Reviewer profile contract: `%s`\n' "$REVIEWER_PROFILE_VERSION"
    printf '> Source SHA-256: `%s`\n\n' "$source_hash"
    printf 'This file is a review-scoped projection of the tagged `SKILL.md`; '
    printf 'the tagged skill remains authoritative.\n\n'
    printf '## Generated sections\n\n'
    for section in "${expected_sections[@]}"; do
        printf -- '- `%s`\n' "$section"
    done
    printf '\n'
} > "$temp_output"

for section in "${expected_sections[@]}"; do
    if ! awk -v wanted="$section" '
        BEGIN { start = "<!-- REVIEWER_SECTION:START " wanted " -->"; end = "<!-- REVIEWER_SECTION:END " wanted " -->" }
        $0 == start {
            if (inside || found) exit 20
            inside = 1
            found = 1
            next
        }
        $0 == end {
            if (!inside) exit 21
            inside = 0
            next
        }
        inside { print }
        END {
            if (!found || inside) exit 22
        }
    ' "$source_file" > "$temp_section"; then
        printf 'invalid or missing reviewer section: %s\n' "$section" >&2
        exit 65
    fi
    if ! grep -q '[^[:space:]]' "$temp_section"; then
        printf 'empty reviewer section: %s\n' "$section" >&2
        exit 65
    fi
    cat "$temp_section" >> "$temp_output"
    printf '\n' >> "$temp_output"
done

mkdir -p "$(dirname "$output")"
mv "$temp_output" "$output"
printf 'generated %s from %s sha256=%s\n' "$output" "$source_file" "$source_hash"
