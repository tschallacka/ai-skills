#!/usr/bin/env bash
# Generate the compact reviewer contract from marked SKILL.md sections.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SKILL_DIR="${1:-$(cd "$SCRIPT_DIR/.." && pwd)}"
SOURCE="$SKILL_DIR/SKILL.md"
OUTPUT="${2:-$SKILL_DIR/REVIEWER.md}"

EXPECTED_SECTIONS=(mandatory-review bounded-context)
REVIEWER_PROFILE_VERSION="1.4.2"

if [ ! -f "$SOURCE" ]; then
    printf 'source skill not found: %s\n' "$SOURCE" >&2
    exit 66
fi

sha256_file() {
    local file="$1"
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$file" | awk '{print $1}'
    elif command -v shasum >/dev/null 2>&1; then
        shasum -a 256 "$file" | awk '{print $1}'
    elif command -v openssl >/dev/null 2>&1; then
        openssl dgst -sha256 "$file" | awk '{print $NF}'
    else
        printf 'no SHA-256 implementation available\n' >&2
        exit 69
    fi
}

SOURCE_HASH="$(sha256_file "$SOURCE")"
TEMP_OUTPUT="$(mktemp "${TMPDIR:-/tmp}/reviewer.XXXXXX")"
TEMP_SECTION="$(mktemp "${TMPDIR:-/tmp}/reviewer-section.XXXXXX")"
trap 'rm -f "$TEMP_OUTPUT" "$TEMP_SECTION"' EXIT

{
    printf '# Reviewer contract\n\n'
    printf '> Generated from `%s` by `scripts/generate-reviewer.sh`.\n' \
        "${SOURCE#$SKILL_DIR/}"
    printf '> Reviewer profile contract: `%s`\n' "$REVIEWER_PROFILE_VERSION"
    printf '> Source SHA-256: `%s`\n\n' "$SOURCE_HASH"
    printf 'This file is a review-scoped projection of the tagged `SKILL.md`; '
    printf 'the tagged skill remains authoritative.\n\n'
    printf '## Generated sections\n\n'
    for section in "${EXPECTED_SECTIONS[@]}"; do
        printf -- '- `%s`\n' "$section"
    done
    printf '\n'
} > "$TEMP_OUTPUT"

for section in "${EXPECTED_SECTIONS[@]}"; do
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
    ' "$SOURCE" > "$TEMP_SECTION"; then
        printf 'invalid or missing reviewer section: %s\n' "$section" >&2
        exit 65
    fi
    if ! grep -q '[^[:space:]]' "$TEMP_SECTION"; then
        printf 'empty reviewer section: %s\n' "$section" >&2
        exit 65
    fi
    cat "$TEMP_SECTION" >> "$TEMP_OUTPUT"
    printf '\n' >> "$TEMP_OUTPUT"
done

mkdir -p "$(dirname "$OUTPUT")"
mv "$TEMP_OUTPUT" "$OUTPUT"
printf 'generated %s from %s sha256=%s\n' "$OUTPUT" "$SOURCE" "$SOURCE_HASH"
