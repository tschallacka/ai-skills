#!/usr/bin/env bash
# MODE: DEV
# test-lib-document.sh — the document functions, each sourced on its own.
#
# The unit layer, deliberately overlapping the integration tests that reach these
# through update-plan-content.sh. Sourcing one function file with its callees
# stubbed means a failure here names the function; the integration failure names
# what it broke.

set -euo pipefail
export LC_ALL=C

tests_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$tests_dir/../.." && pwd)"
lib="$repo_root/planning/scripts/lib"
# shellcheck source=planning/tests/lib-test.sh
source "$tests_dir/lib-test.sh"
t_begin

work="$(mktemp -d "${TMPDIR:-/tmp}/lib-document.XXXXXX")"
trap 'rm -rf "$work"' EXIT

# Files are named group/file so a case can pull a core dependency in explicitly
# rather than getting it by accident.
unit() { # <group/file>... -- <expression>
    local files=() f prelude=''
    while [ "$#" -gt 0 ] && [ "$1" != '--' ]; do files+=("$1"); shift; done
    shift
    for f in "${files[@]}"; do prelude="$prelude source '$lib/$f';"; done
    "$BASH" -c "set -uo pipefail; $prelude $1" 2>&1
}
unit_rc() { # <group/file>... -- <expression>
    local files=() f prelude='' rc=0
    while [ "$#" -gt 0 ] && [ "$1" != '--' ]; do files+=("$1"); shift; done
    shift
    for f in "${files[@]}"; do prelude="$prelude source '$lib/$f';"; done
    "$BASH" -c "set -uo pipefail; $prelude $1" >/dev/null 2>&1 || rc=$?
    printf '%s' "$rc"
}

# ── plan_document_kind: the id vocabulary, including the -testing suffix ────
t_assert_eq 'kind of plan' "$(unit document/plan_document_kind.sh -- 'plan_document_kind plan')" 'plan'
t_assert_eq 'kind of a review' "$(unit document/plan_document_kind.sh -- 'plan_document_kind adversarial-review')" 'review'
t_assert_eq 'kind of a goal' "$(unit document/plan_document_kind.sh -- 'plan_document_kind goal:01-build')" 'goal'
t_assert_eq 'kind of a step' "$(unit document/plan_document_kind.sh -- 'plan_document_kind step:01-build/02-step-x')" 'step'
# The suffix is what separates a step from its companion, and the companion has
# its own writable sections.
t_assert_eq 'kind of a testing companion' \
    "$(unit document/plan_document_kind.sh -- 'plan_document_kind step:01-build/02-step-x-testing')" 'testing'
t_assert_eq 'kind of the inventory' "$(unit document/plan_document_kind.sh -- 'plan_document_kind inventory')" 'reference'
t_assert_eq 'kind of a per-goal progress file' \
    "$(unit document/plan_document_kind.sh -- 'plan_document_kind goal-progress:01-build')" 'reference'

# ── plan_unknown_section: the message names the valid set for that kind ─────
# A review has no narrative section at all, which is why -rs on it must refuse.
review_message="$(unit document/plan_unknown_section.sh core/plan_die.sh core/00-state.sh -- \
    'plan_unknown_section review rationale || true')"
t_assert_contains 'a review reports no rewritable section' 'review' "$review_message"
plan_message="$(unit document/plan_unknown_section.sh core/plan_die.sh core/00-state.sh -- \
    'plan_unknown_section plan nonsense || true')"
t_assert_contains 'a plan lists its own section ids' 'current-state' "$plan_message"
t_assert_contains 'and not a goal id' 'approach' "$plan_message"
goal_message="$(unit document/plan_unknown_section.sh core/plan_die.sh core/00-state.sh -- \
    'plan_unknown_section goal approach || true')"
t_assert_contains 'a goal lists the long approach id' 'implementation-approach-risks-and-edge-cases' "$goal_message"

# ── plan_render_paragraphs: numbering, trimming, and the empty refusal ──────
t_assert_eq 'one paragraph gets one label' \
    "$(unit document/plan_render_paragraphs.sh core/plan_die.sh core/00-state.sh -- \
        'plan_render_paragraphs 4 "The only paragraph."')" \
    "$(printf '§ 4.1\nThe only paragraph.')"
t_assert_eq 'two paragraphs are numbered in order' \
    "$(unit document/plan_render_paragraphs.sh core/plan_die.sh core/00-state.sh -- \
        'plan_render_paragraphs 2 "First.

Second."')" \
    "$(printf '§ 2.1\nFirst.\n\n§ 2.2\nSecond.')"
t_assert_eq 'surrounding whitespace is trimmed' \
    "$(unit document/plan_render_paragraphs.sh core/plan_die.sh core/00-state.sh -- \
        'plan_render_paragraphs 3 "   padded   "')" \
    "$(printf '§ 3.1\npadded')"
t_assert_eq 'empty content is refused' \
    "$(unit_rc document/plan_render_paragraphs.sh core/plan_die.sh core/00-state.sh -- \
        'plan_render_paragraphs 2 ""')" '64'

# ── plan_replace_title: the first heading only ──────────────────────────────
# The heading is "# Kind: title" and only the text after the colon is replaced.
printf '# Plan: Old title\n\n## Current state\n\nProse.\n' > "$work/doc.md"
unit document/plan_replace_title.sh core/plan_die.sh core/plan_atomic_write.sh core/plan_track_tmp.sh \
     core/plan_stat_probe.sh core/plan_require_safe_value.sh core/plan_register_temp_file.sh \
     core/00-state.sh -- \
    "plan_replace_title '$work/doc.md' 'New title'" >/dev/null 2>&1
t_assert_eq 'the title after the colon is replaced' "$(head -1 "$work/doc.md")" '# Plan: New title'
t_assert_eq 'the body is untouched' "$(grep -c '^Prose\.$' "$work/doc.md")" '1'
# A heading with no colon is refused, not silently ignored: a rename that
# changes nothing while reporting success is the shape the drop-notice contract
# exists to prevent. Every document a creator writes has the "# Kind: title"
# form, so this is only reachable on a damaged heading -- and damage must be
# loud.
printf '# NoColonHeading\n\nProse.\n' > "$work/no-colon.md"
t_assert_eq 'a colonless heading is refused' \
    "$(unit_rc document/plan_replace_title.sh core/plan_die.sh core/plan_atomic_write.sh \
        core/plan_track_tmp.sh core/plan_stat_probe.sh core/plan_require_safe_value.sh \
        core/plan_register_temp_file.sh core/00-state.sh -- \
        "plan_replace_title '$work/no-colon.md' 'New'")" '64'
t_assert_eq 'the refused heading is left untouched' "$(head -1 "$work/no-colon.md")" '# NoColonHeading'
t_assert_contains 'the refusal says why' \
    'Document title heading has no '"'"': title'"'"' part to replace' \
    "$(unit document/plan_replace_title.sh core/plan_die.sh core/plan_atomic_write.sh \
        core/plan_track_tmp.sh core/plan_stat_probe.sh core/plan_require_safe_value.sh \
        core/plan_register_temp_file.sh core/00-state.sh -- \
        "plan_replace_title '$work/no-colon.md' 'New'" 2>&1)"
# Exactly one top-level heading is the contract: a second one is ambiguous about
# which is the title, so it refuses rather than guessing.
printf '# One\n\n# Two\n' > "$work/two-titles.md"
t_assert_eq 'two top-level headings are refused' \
    "$(unit_rc document/plan_replace_title.sh core/plan_die.sh core/plan_atomic_write.sh \
        core/plan_track_tmp.sh core/plan_stat_probe.sh core/plan_require_safe_value.sh \
        core/plan_register_temp_file.sh core/00-state.sh -- \
        "plan_replace_title '$work/two-titles.md' 'New'")" '64'

# ── plan_replace_field: one label, siblings untouched ───────────────────────
printf '# Doc\n\n## Verdict\n\n- Status: `pending`\n- Rationale: the old one\n' > "$work/fields.md"
unit document/plan_replace_field.sh core/plan_die.sh core/plan_atomic_write.sh core/plan_track_tmp.sh \
     core/plan_stat_probe.sh core/plan_require_safe_value.sh core/plan_register_temp_file.sh \
     core/00-state.sh -- \
    "plan_replace_field '$work/fields.md' 'Rationale' 'the new one'" >/dev/null 2>&1
t_assert_eq 'the named field is rewritten' \
    "$(grep -c '^- Rationale: the new one$' "$work/fields.md")" '1'
t_assert_eq 'the sibling field survives' \
    "$(grep -c '^- Status: `pending`$' "$work/fields.md")" '1'

# ── plan_refuse_field_section: the guard that saved the approval gate ───────
# A section holding `- Label:` lines must refuse a whole-section rewrite, because
# one of those fields is what the approval gate reads.
t_assert_eq 'a field-shaped section is refused' \
    "$(unit_rc document/plan_refuse_field_section.sh core/plan_die.sh core/00-state.sh -- \
        "plan_refuse_field_section '$work/fields.md' '## Verdict'")" '65'
printf '# Doc\n\n## Approach\n\n%s 3.1\nProse only.\n' '§' > "$work/narrative.md"
t_assert_eq 'a narrative section is allowed' \
    "$(unit_rc document/plan_refuse_field_section.sh core/plan_die.sh core/00-state.sh -- \
        "plan_refuse_field_section '$work/narrative.md' '## Approach'")" '0'
printf '# Doc\n\n## Findings\n\n| ID | Item |\n|---|---|\n| AR-01 | x |\n' > "$work/table.md"
t_assert_eq 'a table section is refused' \
    "$(unit_rc document/plan_refuse_field_section.sh core/plan_die.sh core/00-state.sh -- \
        "plan_refuse_field_section '$work/table.md' '## Findings'")" '65'

# ── plan_missing_section_message: names what the document does have ─────────
absent="$(unit document/plan_missing_section_message.sh -- \
    "plan_missing_section_message '$work/narrative.md' '## Browser verification'")"
t_assert_contains 'the message names the absent heading' 'Browser verification' "$absent"
t_assert_contains 'and the sections the file holds' '## Approach' "$absent"
companion_absent="$(unit document/plan_missing_section_message.sh -- \
    "plan_missing_section_message '$work/x-testing.md' '## Browser verification'")"
t_assert_contains 'a companion names its creator' 'create-step-testing.sh' "$companion_absent"

t_end
