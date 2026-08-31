#!/usr/bin/env bash
# MODE: DEV
# test-shipped-binaries — check the binary registry, artifacts, and installer.
set -euo pipefail
export LC_ALL=C

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
registry="$repo_root/planning/binaries.tsv"
[ -f "$registry" ] || { printf 'missing %s\n' "$registry" >&2; exit 1; }
tab="$(printf '\t')"
declared="$(mktemp)"
missing="$(mktemp)"
trap 'rm -f "$declared" "$missing"' EXIT

awk -F "$tab" '!/^[[:space:]]*#/ && NF && $1 != "target" { print "bin/" $1 "/" $3 }' "$registry" | sort -u >"$declared"
[ -s "$declared" ] || { printf 'empty binary registry\n' >&2; exit 1; }

while IFS="$tab" read -r target condition binary why; do
    [ -n "$target" ] || continue
    case "$target" in \#*) continue ;; esac
    [ "$target" = target ] && continue
    path="planning/bin/$target/$binary"
    if [ ! -f "$repo_root/$path" ]; then
        printf '%s\n' "$path" >> "$missing"
    fi
done < "$registry"

if [ -s "$missing" ]; then
    printf 'test-shipped-binaries: UNAVAILABLE (prebuilt artifacts not present)\n' >&2
    while IFS= read -r path; do
        printf '  unavailable: %s\n' "$path" >&2
    done < "$missing"
else
    printf '%s\n' 'test-shipped-binaries: artifacts present'
fi

installer_paths="$(awk '/^SKILL.md$/{in_plan=1} in_plan && /^bin\//{print} in_plan && /^EOF$/{exit}' \
    "$repo_root/install.sh" | sort -u)"
cmp -s "$declared" <(printf '%s\n' "$installer_paths") || {
    printf 'installer binary paths differ from planning/binaries.tsv\n' >&2
    exit 1
}

printf '%s\n' 'test-shipped-binaries: PASS'
