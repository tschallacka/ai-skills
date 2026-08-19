#!/usr/bin/env bash
# test-document-id-parity.sh — plan-content.sh and plan-context.sh accept the
# same document ids, and the id names the document.
#
# The bugs this pins: the two readers had different vocabularies, so a reviewer
# who learned an id from one had it refused by the other. One document was
# served under two spellings (`review` here, `adversarial-review` there), and
# five documents a reviewer must audit -- coverage, stories, fixes, fix-keys,
# approval -- were reachable only through the ungated reader, which SKILL.md
# prohibits using for plan content.

set -euo pipefail
export LC_ALL=C

tests_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
scripts_dir="$(cd "$tests_dir/../scripts" && pwd)"
fixture="$(cd "$tests_dir/../../benchmark/planning/tests/fixtures/review-lifecycle-plan" && pwd)"
work="$(mktemp -d "${TMPDIR:-/tmp}/doc-id-parity.XXXXXX")"
trap 'rm -rf "$work"' EXIT

fail=0
note_fail() { printf 'FAIL: %s\n' "$1" >&2; fail=$((fail + 1)); }

plan="$work/plan"
cp -R "$fixture" "$plan"
# Every optional document present, so an absent file cannot be mistaken for a
# rejected id -- the distinction that made this gap hard to see.
printf '# UI user stories\n' > "$plan/ui-user-stories.md"
printf '# Fixes\n' > "$plan/fixes.md"
printf '# Bugs\n' > "$plan/bugs.md"
printf '{"minted_by":"probe"}\n' > "$plan/fix-keys.json"
printf '{"overall_plan_approval":true}\n' > "$plan/approval.json"

goal=01-lossless-finding-contract
canonical="plan inventory coverage progress adversarial-review stories bugs fixes fix-keys approval
goal:$goal goal-progress:$goal step:$goal/01-step-preserve-finding-envelope"

rejected() { # <output> -> 0 when the id was refused rather than the file missing
    case "$1" in
        *'Unknown document ID'*|*'unsupported entry id'*|*'usage: invalid '*) return 0 ;;
    esac
    return 1
}

for id in $canonical; do
    out="$("$scripts_dir/plan-content.sh" get "$plan" "$id" 2>&1)" || true
    ! rejected "$out" || note_fail "plan-content.sh rejects the canonical id '$id'"
    out="$(PLANNING_CONTEXT_CACHE=1 "$scripts_dir/plan-context.sh" read \
        --plan-dir "$plan" --document "$id" --read-only 2>&1)" || true
    ! rejected "$out" || note_fail "plan-context.sh rejects the canonical id '$id'"
done

# The old spelling is gone from both, not aliased: two names for one document is
# the defect, so an alias would preserve it.
for reader in content context; do
    if [ "$reader" = content ]; then
        out="$("$scripts_dir/plan-content.sh" get "$plan" review 2>&1)" || true
    else
        out="$(PLANNING_CONTEXT_CACHE=1 "$scripts_dir/plan-context.sh" read \
            --plan-dir "$plan" --document review --read-only 2>&1)" || true
    fi
    rejected "$out" || note_fail "plan-$reader still accepts the ambiguous id 'review'"
done

# The id must name the right document, or parity is cosmetic.
check_serves() { # <id> <needle>
    local out
    out="$(PLANNING_CONTEXT_CACHE=1 "$scripts_dir/plan-context.sh" read \
        --plan-dir "$plan" --document "$1" --read-only 2>&1)" || true
    case "$out" in
        *"$2"*) ;;
        *) note_fail "the id '$1' did not serve its document (wanted '$2')" ;;
    esac
}
check_serves stories '# UI user stories'
check_serves bugs '# Bugs'
check_serves fixes '# Fixes'
check_serves fix-keys 'minted_by'
check_serves approval 'overall_plan_approval'
check_serves adversarial-review 'Adversarial review'

# JSON documents must arrive whole: a truncated head is not parseable, so a
# summary default would hand a reviewer something they cannot use.
for id in fix-keys approval; do
    out="$(PLANNING_CONTEXT_CACHE=1 "$scripts_dir/plan-context.sh" read \
        --plan-dir "$plan" --document "$id" --format json --read-only 2>&1)" || true
    case "$out" in
        *'"view":"full"'*) ;;
        *) note_fail "$id did not default to the full view" ;;
    esac
done

# check must see them, or an edit to one goes unreported.
PLANNING_CONTEXT_CACHE=1 "$scripts_dir/plan-context.sh" init --plan-dir "$plan" >/dev/null 2>&1 || true
index="$(find "$plan/context" -name index.tsv 2>/dev/null | head -1)"
if [ -n "$index" ]; then
    for id in coverage stories fixes fix-keys approval; do
        entries="$(grep -c "^$id	" "$index" || true)"
        [ "${entries:-0}" -ge 1 ] || note_fail "the context index carries no '$id' entry"
    done
else
    note_fail 'context init wrote no index'
fi

# A missing document and an unknown id are different faults, and a caller must
# be able to tell them apart (CODE-STYLE §5): 66 is "the input is not there",
# 64 is "you asked for something that does not exist".
rm -f "$plan/bugs.md"
rc=0
"$scripts_dir/plan-content.sh" get "$plan" bugs >/dev/null 2>&1 || rc=$?
[ "$rc" -eq 66 ] || note_fail "a missing document exited $rc, want 66"
rc=0
"$scripts_dir/plan-content.sh" get "$plan" no-such-id >/dev/null 2>&1 || rc=$?
[ "$rc" -eq 64 ] || note_fail "an unknown document id exited $rc, want 64"

if [ "$fail" -ne 0 ]; then
    printf 'test-document-id-parity: %d failure(s).\n' "$fail" >&2
    exit 1
fi
printf 'test-document-id-parity: PASS\n'
