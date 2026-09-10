#!/usr/bin/env bash
# MODE: DEV
# generate-skill-docs.sh — build SKILL.md and its parts from skill-source.txt.
#
# A maintainer-only tool, like skill-source.txt itself: an installed skill
# already carries the generated SKILL.md/parts it needs to be USED, and
# regenerating them is only ever done by someone editing this repository.
#
# planning/skill-source.txt is the authored content; SKILL.md and
# planning/parts/part-N.md are GENERATED from it, the way install.sh is
# generated from installer/src/ — same reason: a file over roughly 25,000
# tokens is silently truncated by at least one harness this repo runs under
# (see .agents/knowledge/agent-read-limits.md), and planning/SKILL.md alone
# was 1502 lines / 89,860 bytes, well past that (T87).
#
# skill-source.txt marks each part with a comment pair:
#   <!-- SKILL_SECTION:START <name> targets=<target,...> -->
#   ...
#   <!-- SKILL_SECTION:END <name> -->
# Every byte between them is copied verbatim into every listed target. A
# target may be named by more than one section (they concatenate, in source
# order), and a section may list more than one target — neither is used by
# planning today, but the mechanism does not assume a single 1:1 split.
#
# Generated files stay COMMITTED and SHIPPED: unlike install.sh, a skill has
# no build step at the point it is consumed, so there is nothing to run this
# at. Use --check to catch a source edit that was not followed by a rebuild.
#
# T86: every part also gets a load-sanity line — see plant_load_proof below —
# at a random position, planted in the same pass so the split and the load
# check are one generator rather than two retrofitted onto each other.
#
# generate-reviewer.sh is a separate, narrower generator over the same
# source (REVIEWER.md is not committed, unlike these targets, and its own
# REVIEWER_SECTION markers serve a different purpose: an excerpt for review,
# not a readable part). This script does not produce REVIEWER.md.
#
# Usage:
#   generate-skill-docs.sh [--check] [<skill-directory>]
#   generate-skill-docs.sh --help
#
# Exit codes: 64 = bad usage; 65 = a listed part had no matching section, or a
# section is empty; 66 = skill-source.txt is missing; 69 = no SHA-256
# implementation (the plan-crypt binary, sha256sum or shasum) for the T86
# token, which is derived from each part's own content — see plant_load_proof.
set -euo pipefail
export LC_ALL=C

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=planning/scripts/plan-crypt-lib.sh
source "$script_dir/plan-crypt-lib.sh"

usage() {
    local rc="${1:-64}"
    cat <<USAGE
Usage: ${0##*/} [--check] [<skill-directory>]
       ${0##*/} --help
USAGE
    exit "$rc"
}

check_only=false
case "${1:-}" in
    -h|--help) usage 0 ;;
    --check) check_only=true; shift ;;
esac
[ "$#" -le 1 ] || usage
case "${1:-}" in -*) usage ;; esac

skill_dir="${1:-$(cd "$script_dir/.." && pwd)}"
source_file="$skill_dir/skill-source.txt"
[ -f "$source_file" ] || { printf 'source skill not found: %s\n' "$source_file" >&2; exit 66; }

# The four parts this source is split into today. Adding a fifth is one more
# entry here plus a matching SKILL_SECTION in the source — nothing else in
# this script names a part count.
PARTS=(part-1 part-2 part-3 part-4)
PART_TITLES=(
    "1 of 4 — setup, operating rules, gates, and establishing the plan boundary"
    "2 of 4 — create the plan directory"
    "3 of 4 — mandatory classification and independent review"
    "4 of 4 — resume and update the plan"
)

# skill_section_body <target> — every SKILL_SECTION whose targets= list
# contains <target>, concatenated in source order, verbatim.
skill_section_body() {
    local wanted="$1"
    awk -v wanted="$wanted" '
        /^<!-- SKILL_SECTION:START / {
            line = $0
            sub(/^<!-- SKILL_SECTION:START /, "", line)
            sub(/ -->[[:space:]]*$/, "", line)
            targets = line
            sub(/^[^[:space:]]+[[:space:]]+targets=/, "", targets)
            matched = 0
            n = split(targets, arr, ",")
            for (i = 1; i <= n; i++) if (arr[i] == wanted) matched = 1
            if (matched) inside = 1
            next
        }
        /^<!-- SKILL_SECTION:END / { inside = 0; next }
        inside { print }
    ' "$source_file"
}

# front_matter — the YAML block between the first two '---' lines, verbatim,
# for SKILL.md alone: the parts are prose, not skill entry points, and never
# carry frontmatter of their own.
front_matter() {
    awk '
        /^---$/ { n++; print; if (n == 2) exit; next }
        n == 1 { print }
    ' "$source_file"
}

# plant_load_proof <part-name> — T86. Inserts a load-sanity line in the LAST
# FIFTH of the body (stdin), so its presence is evidence of having read past
# that point rather than of having reached a landmark a truncated read could
# still be told to name. Both the token and its offset within that range are
# derived from the body's own SHA-256 (two disjoint slices of the same
# digest) rather than drawn from an entropy source: neither is guessable
# without the actual content, and both stay stable across an unchanged
# source, which is what lets --check compare against a committed file at
# all — a fresh random draw on every run would make every check a false
# diff regardless of whether the source had changed.
plant_load_proof() {
    local part="$1" body total floor span hash token offset_dec at
    body="$(cat)"
    total="$(printf '%s\n' "$body" | wc -l | tr -d ' ')"
    floor=$((total * 4 / 5))
    [ "$floor" -lt "$total" ] || floor=$((total - 1))
    [ "$floor" -ge 0 ] || floor=0
    hash="$(printf '%s' "$body" | plan_sha256_hex)" || {
        printf 'no SHA-256 implementation available (need %s)\n' "$(plan_sha256_chain)" >&2
        exit 69
    }
    token="${hash:0:16}"
    span=$((total - floor + 1))
    [ "$span" -gt 0 ] || span=1
    offset_dec=$((0x${hash:16:8}))
    at=$((floor + (offset_dec % span)))
    printf '%s\n' "$body" | awk -v at="$at" -v part="$part" -v token="$token" '
        { print }
        NR == at { printf "\n<!-- SKILL-LOAD-PROOF part=%s token=%s -->\n\n", part, token }
    '
}

# emit_part <index 0-based> — MODE banner, the load-sanity instructions (T86),
# then the section body with its planted proof line.
emit_part() {
    local i="$1"
    local name="${PARTS[$i]}" title="${PART_TITLES[$i]}" body
    body="$(skill_section_body "$name")"
    if [ -z "$(printf '%s' "$body" | tr -d '[:space:]')" ]; then
        printf 'no content for part %s (targets=%s matched nothing in %s)\n' \
            "$name" "$name" "$source_file" >&2
        exit 65
    fi
    printf '<!-- MODE: PROD -->\n'
    printf '> Generated from `skill-source.txt` by `scripts/generate-skill-docs.sh` — do not edit.\n'
    printf '> Part %s.\n' "$title"
    printf '>\n'
    printf '> Before treating this part as read: find the line below matching\n'
    printf '> `<!-- SKILL-LOAD-PROOF part=%s token=... -->` — its position moves on every\n' "$name"
    printf '> regeneration — and run `planning/scripts/verify-skill-load.sh --part %s\n' "$name"
    printf '> --token <the-token-you-found>` before continuing. Naming a token is not\n'
    printf '> enough; the command must succeed. If it refuses, you have not finished\n'
    printf '> reading this part.\n\n'
    printf '%s\n' "$body" | plant_load_proof "$name"
}

# emit_index — SKILL.md: front matter, a short pointer table, and the same
# load-sanity contract stated once so a reader of the index knows what every
# part below expects before it opens one.
emit_index() {
    front_matter
    printf '<!-- MODE: PROD -->\n\n'
    printf '# Planning\n\n'
    printf 'Use this skill to turn an initiative into a directory of Markdown files\n'
    printf 'that another agent can resume and execute without reconstructing missing\n'
    printf 'context. Do not use it for a small, self-contained change or a temporary\n'
    printf 'in-chat plan.\n\n'
    printf 'The full skill is generated from a single authored source\n'
    printf '(`skill-source.txt`) into this short index plus the parts below, because\n'
    printf 'the whole thing is 89,860 bytes and at least one harness this repo runs\n'
    printf 'under silently truncates a file read past roughly 25,000 tokens with no\n'
    printf 'notice anywhere (`.agents/knowledge/agent-read-limits.md`). Read the part\n'
    printf 'that applies to what you are doing now; each is well under that budget on\n'
    printf 'its own.\n\n'
    printf '| Part | Covers |\n|---|---|\n'
    printf '| [parts/part-1.md](parts/part-1.md) | Setup, operating rules, tool/context-limit discipline, hard planning gates, establishing the plan boundary |\n'
    printf '| [parts/part-2.md](parts/part-2.md) | Creating the plan directory |\n'
    printf '| [parts/part-3.md](parts/part-3.md) | Mandatory classification and independent review |\n'
    printf '| [parts/part-4.md](parts/part-4.md) | Resuming and updating a plan |\n\n'
    printf 'Every part carries a load-sanity check (T86): a hidden line at a random\n'
    printf 'position near its end, and a command (`planning/scripts/verify-skill-load.sh\n'
    printf 'with --part <N> --token <token>`) that must succeed before the part counts\n'
    printf 'as read. Stating a token from memory or from this index is not that command\n'
    printf 'succeeding; the check exists because reporting a token is claimable and\n'
    printf 'running the command against the real file is not.\n'
}

write_or_check() {
    local target="$1" content="$2"
    if [ "$check_only" = true ]; then
        if ! diff -u "$target" - <<<"$content" >/dev/null 2>&1; then
            printf '%s is stale; run %s\n' "${target#"$skill_dir"/}" "${0##*/}" >&2
            stale=1
        fi
        return 0
    fi
    printf '%s\n' "$content" > "$target"
}

stale=0
mkdir -p "$skill_dir/parts"
write_or_check "$skill_dir/SKILL.md" "$(emit_index)"
for i in "${!PARTS[@]}"; do
    write_or_check "$skill_dir/parts/${PARTS[$i]}.md" "$(emit_part "$i")"
done

if [ "$check_only" = true ]; then
    [ "$stale" -eq 0 ] || exit 1
    printf 'SKILL.md and parts/ are up to date with skill-source.txt\n'
else
    printf 'Wrote %s and %s/parts/{%s}.md\n' \
        "$skill_dir/SKILL.md" "$skill_dir" "$(IFS=,; echo "${PARTS[*]}")"
fi
