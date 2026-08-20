#!/usr/bin/env bash
# test-document-sections.sh — a section-form flag may only target a narrative
# section. See CODE-CONTRACTS.md contract 1.
#
# A section form (-ds/-gs/-ss/-rs) rewrites a whole section. That is correct for
# numbered paragraphs and destructive for anything else: it removes the sibling
# `- Label:` fields or the table that lived there. `update-plan-content.sh -rs
# <plan> rationale` dropped `- Status:` from `## Verdict`, which is the field the
# approval gate reads, leaving the plan unapprovable with no helper able to
# restore it.

set -euo pipefail
export LC_ALL=C

tests_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$tests_dir/../.." && pwd)"
scripts_dir="$repo_root/planning/scripts"
registry="$repo_root/planning/document-sections.json"
# shellcheck source=planning/tests/lib-test.sh
source "$tests_dir/lib-test.sh"
t_begin

command -v jq >/dev/null 2>&1 || {
    printf 'test-document-sections: UNCONFIGURED (jq) — the registry is JSON\n' >&2
    exit 0
}
[ -f "$registry" ] || t_fail "the section registry is missing: $registry"

# ---- every allow-listed section is narrative --------------------------------
for kind in plan goal step testing review; do
    list="$(sed -n "s/^        $kind) valid=\"\([^\"]*\)\".*/\1/p" \
        "$scripts_dir/plan-document-lib.sh")"
    for section in $list; do
        shape="$(jq -r --arg k "$kind" --arg s "$section" \
            '.documents[$k][$s] // "UNREGISTERED"' "$registry")"
        case "$shape" in
            narrative|empty) ;;
            UNREGISTERED)
                t_fail "$kind allows '$section' but document-sections.json does not describe it" ;;
            *)
                t_fail "$kind allows '$section', which is $shape: a section rewrite would destroy it — use that shape's writer" ;;
        esac
    done
done

# ---- the registry matches what the documents actually contain ---------------
# plan_section_heading is the authoritative <kind>/<id> -> heading mapping and
# therefore the set of section-form targets. Deriving an id from a heading does
# not work: goal/affected-areas is "## Affected files, systems, data, and
# interfaces", and review/rationale is "## Verdict".
fixture="$repo_root/benchmark/planning/tests/fixtures/self-hosted-plan"
declare_file() { # <kind> -> the document to inspect for that kind
    case "$1" in
        plan) printf '%s\n' "$fixture/plan-description.md" ;;
        review) printf '%s\n' "$fixture/adversarial-review.md" ;;
        goal) printf '%s\n' "$fixture/01-plan-dir-synonym/goal.md" ;;
        step) printf '%s\n' "$fixture/01-plan-dir-synonym/steps/01-step-confirm-hoister.md" ;;
        testing) printf '%s\n' "$fixture/01-plan-dir-synonym/steps/01-step-confirm-hoister-testing.md" ;;
    esac
}

observed_shape() { # <file> <heading>
    awk -v want="$2" '
        $0 == want { inside = 1; next }
        inside && /^## / { exit }
        inside && /^- [A-Z][^:]*:/ { f++ }
        inside && /^§ [0-9]+\.[0-9]+/ { p++ }
        inside && /^\|/ { t++ }
        END {
            if (t > 0 && p == 0) print "table"
            else if (p > 0 && f > 0) print "hybrid"
            else if (p > 0) print "narrative"
            else if (f > 0) print "fields"
            else print "empty"
        }' "$1"
}

mapping="$(grep -oE "^        [a-z]+/[a-z-]+\) printf '%s\\\\t%s\\\\n' '## [^']*'" \
    "$scripts_dir/plan-document-lib.sh" |
    sed "s/^ *//; s/) printf '%s\\\\t%s\\\\n' '/\t/; s/'$//")"
[ -n "$mapping" ] || t_fail 'could not read the section-id to heading mapping'
while IFS="$(printf '\t')" read -r key heading; do
    [ -n "${key:-}" ] || continue
    kind="${key%%/*}"; section="${key#*/}"
    file="$(declare_file "$kind")"
    [ -f "$file" ] || continue
    recorded="$(jq -r --arg k "$kind" --arg s "$section" \
        '.documents[$k][$s] // "UNREGISTERED"' "$registry")"
    if [ "$recorded" = UNREGISTERED ]; then
        t_fail "$key is a section-form target with no entry in document-sections.json"
        continue
    fi
    # A heading the template does not emit yet (the optional verification
    # sections) has no shape to compare against.
    grep -Fqx "$heading" "$file" || continue
    seen="$(observed_shape "$file" "$heading")"
    [ "$seen" = "$recorded" ] \
        || t_fail "$key is $seen in the document but document-sections.json records $recorded"
done <<MAPPING
$mapping
MAPPING

# ---- behaviour: a field section keeps its siblings --------------------------
# The contract is only worth anything if the refusal actually happens.
work="$(mktemp -d "${TMPDIR:-/tmp}/document-sections.XXXXXX")"
trap 'rm -rf "$work"' EXIT
plan="$work/plan"
cp -R "$fixture" "$plan"

before_status="$({ grep -c '^- Status:' "$plan/adversarial-review.md" || true; })"
t_assert_eq 'the review carries exactly one Status field to begin with' "$before_status" 1
rc=0
"$scripts_dir/update-plan-content.sh" -rs "$plan" rationale \
    -p '3.1: - Rationale: a rewritten rationale.' >/dev/null 2>&1 || rc=$?
[ "$rc" -ne 0 ] || t_fail 'a section rewrite of a field-shaped section was accepted'
after_status="$({ grep -c '^- Status:' "$plan/adversarial-review.md" || true; })"
t_assert_eq 'the Status field survives the refused rewrite' "$after_status" 1

# The sanctioned writer for a field still works.
rc=0
"$scripts_dir/update-plan-content.sh" -f "$plan" adversarial-review Rationale \
    'a rationale written through the field form.' >/dev/null 2>&1 || rc=$?
t_assert_eq 'the field form writes a field' "$rc" 0
t_assert_eq 'and leaves Status alone' \
    "$({ grep -c '^- Status:' "$plan/adversarial-review.md" || true; })" 1

t_end
