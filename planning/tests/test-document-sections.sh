#!/usr/bin/env bash
# MODE: DEV
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
            '.documents[$k][$s].shape // "UNREGISTERED"' "$registry")"
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
# The heading comes from the registry, not from scraping the library's case
# statement. The scrape pinned eight spaces of indentation and ids of [a-z-]+
# only, so it resolved 29 of 36 sections. Six of the nine field, table and
# hybrid shapes went unverified: a scrape that matches less verifies less, and
# still reports success.
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

checked=0
while IFS="$(printf '\t')" read -r key shape heading; do
    [ -n "${key:-}" ] || continue
    kind="${key%%/*}"
    file="$(declare_file "$kind")"
    [ -f "$file" ] || continue
    # An optional section the template does not emit has no shape to compare.
    grep -Fqx "$heading" "$file" || continue
    seen="$(observed_shape "$file" "$heading")"
    if [ "$seen" = "$shape" ]; then
        checked=$((checked + 1))
    else
        t_fail "$key is $seen in the document but document-sections.json records $shape"
    fi
done <<REGISTRY
$(jq -r '.documents | to_entries[] | .key as $k | .value | to_entries[]
         | "\($k)/\(.key)\t\(.value.shape)\t\(.value.heading)"' "$registry")
REGISTRY
[ "$checked" -ge 30 ] \
    || t_fail "only $checked section shape(s) were verified against a document; the registry or the fixture stopped covering them"

# ---- the library's section-form targets agree with the registry -------------
# Two independent reads of the same case statement: the id labels, and the
# id-plus-heading pairs. A reformat that breaks the pair regex still shows up in
# the label count, so a mapping that quietly shrinks fails here instead of
# reducing coverage in silence.
lib="$scripts_dir/plan-document-lib.sh"
# The id labels, counted without reference to how the heading is emitted.
labels="$(grep -cE "^[[:space:]]*[a-z]+/[a-z0-9-]+\)" "$lib" || true)"
# The id-and-heading pairs, split on quotes rather than matched against a
# printf format: pinning the format is what let the previous read resolve 29 of
# 36 sections while still reporting success.
pairs="$(awk -F"'" '
    /^[[:space:]]*[a-z]+\/[a-z0-9-]+\)/ {
        split($0, parts, ")"); id = parts[1]
        gsub(/^[[:space:]]+/, "", id)
        heading = ""
        for (i = 2; i <= NF; i++) if (heading == "" && substr($i, 1, 3) == "## ") heading = $i
        if (heading != "") print id "\t" heading
    }' "$lib")"
pair_count="$(printf '%s\n' "$pairs" | grep -c . || true)"
t_assert_eq 'every section-form label yields an id-and-heading pair' "$pair_count" "$labels"

while IFS="$(printf '\t')" read -r key heading; do
    [ -n "${key:-}" ] || continue
    kind="${key%%/*}"; section="${key#*/}"
    recorded="$(jq -r --arg k "$kind" --arg s "$section" \
        '.documents[$k][$s].heading // "UNREGISTERED"' "$registry")"
    case "$recorded" in
        UNREGISTERED) t_fail "$key is a section-form target with no entry in document-sections.json" ;;
        "$heading") ;;
        *) t_fail "$key heading is '$heading' in the library but '$recorded' in the registry" ;;
    esac
done <<MAPPING
$pairs
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


# ---- a section absent from this document names what the document has ---------
# A valid section id that a particular file never received used to report only
# "not found exactly once". A reviewer read that as a broken helper and inferred
# `create-step-testing.sh --overwrite` as the fix, which cannot work: that
# creator emits "## Automated tests" and nothing else.
companion="$plan/01-plan-dir-synonym/steps/01-step-confirm-hoister-testing.md"
if [ -f "$companion" ]; then
    rc=0
    absent_message="$("$scripts_dir/update-plan-content.sh" -ss "$plan" \
        '01-plan-dir-synonym/01-step-confirm-hoister-testing' browser-verification \
        -p '3.1: a browser check.' 2>&1)" || rc=$?
    [ "$rc" -ne 0 ] || t_fail 'a section form added a section the companion never had'
    case "$absent_message" in
        *'## Automated tests'*) ;;
        *) t_fail "the refusal did not name the sections the companion has: $absent_message" ;;
    esac
    case "$absent_message" in
        *create-step-testing.sh*) ;;
        *) t_fail "the refusal did not name the creator that owns the companion's sections: $absent_message" ;;
    esac
else
    t_fail "the fixture companion is missing: $companion"
fi


# ---- every accepted section id is documented in SKILL.md --------------------
# A goal's ids are not the plan description's and are not derivable from the
# headings, so an agent that reads only the plan-description list guesses
# `approach` for a goal and fails. SKILL.md carried the plan list and not the
# goal list, which is precisely the asymmetry that produced the wrong guess. A
# documented list that drifts is worse than none.
skill_doc="$repo_root/planning/SKILL.md"
for kind in plan goal; do
    list="$(sed -n "s/^        $kind) valid=\"\([^\"]*\)\".*/\1/p" \
        "$scripts_dir/plan-document-lib.sh")"
    [ -n "$list" ] || t_fail "could not read the $kind section allow-list"
    for section in $list; do
        grep -Fq "\`$section\`" "$skill_doc" \
            || t_fail "$kind accepts '$section' but SKILL.md never names it; an agent has to fail once to discover it"
    done
done

t_end
