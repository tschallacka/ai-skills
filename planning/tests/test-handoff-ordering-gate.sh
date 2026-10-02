#!/usr/bin/env bash
# MODE: DEV
# B113 — a step's `## Handoff` paragraph naming a work unit the dependency
# graph does not order (in either direction) must be flagged. Before this,
# --propagation swept six surfaces and never read Handoff prose at all, so a
# step could promise a later unit something the graph never guaranteed and
# nothing objected.
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/../scripts" && pwd)"
# shellcheck source=planning/tests/lib-test.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib-test.sh"
t_begin
fail() { t_fail "$*"; }
temporary_root="$(mktemp -d "${TMPDIR:-/tmp}/planning-handoff-gate-test.XXXXXX")"
trap 'rm -rf "$temporary_root"' EXIT

# set_handoff <step-file> <paragraph> -- replaces add-work-unit.sh's
# placeholder line, the only portable way to author Handoff prose: this repo
# treats `sed -i` as non-portable (BSD vs GNU differ), so a rewrite-and-move
# through awk is the house style.
set_handoff() {
    local file="$1" paragraph="$2" tmp="$1.tmp"
    awk -v repl="$paragraph" '
        $0 == "<what the next named work unit can rely on>" { print repl; next }
        { print }
    ' "$file" > "$tmp" && mv "$tmp" "$file"
}

build_fixture() { # <plan-dir>
    local plan="$1"
    "$script_dir/create-plan.sh" "$plan" handoff-gate >/dev/null
    "$script_dir/add-goal.sh" "$plan" 01-g 'G' 'an outcome' >/dev/null
    # W01: unordered relative to W02 in both directions -- the defect shape.
    "$script_dir/add-work-unit.sh" "$plan" --id W01 --type source --file a.php \
        --scope 'A::x' --subscope N/A --change 'change A' \
        --depends-on '—' --goal 01-g --step 01-step-a >/dev/null
    "$script_dir/add-work-unit.sh" "$plan" --id W02 --type source --file b.php \
        --scope 'B::x' --subscope N/A --change 'change B' \
        --depends-on '—' --goal 01-g --step 02-step-b >/dev/null
    # W04 depends on W03, so W03's handoff naming W04 IS ordered: W03 runs
    # first, and W04 (which depends on it) is exactly who can rely on that.
    "$script_dir/add-work-unit.sh" "$plan" --id W03 --type source --file c.php \
        --scope 'C::x' --subscope N/A --change 'change C' \
        --depends-on '—' --goal 01-g --step 03-step-c >/dev/null
    "$script_dir/add-work-unit.sh" "$plan" --id W04 --type source --file d.php \
        --scope 'D::x' --subscope N/A --change 'change D' \
        --depends-on 'W03' --goal 01-g --step 04-step-d >/dev/null
    set_handoff "$plan/01-g/steps/01-step-a.md" 'W02 can rely on A being changed.'
    set_handoff "$plan/01-g/steps/03-step-c.md" 'W04 can rely on C being changed.'
    # A corrective paragraph restating an old, disproven claim about W02 --
    # the history marker must exempt it even though it names an unordered
    # unit, or every self-correcting plan gets a permanent false positive.
    set_handoff "$plan/01-g/steps/04-step-d.md" \
        'An earlier version of this paragraph said W02 could rely on this; W02 has no dependency relation to this step at all.'
}

plan="$temporary_root/plan"
build_fixture "$plan"
"$script_dir/validate-plan.sh" "$plan" > "$temporary_root/out.log" 2>&1 || true

t_assert_eq \
    "an unordered handoff pair (W01 -> W02) is reported" \
    "$(grep -c 'W01 handoff names W02, but neither has a dependency path' "$temporary_root/out.log" || true)" \
    1

t_assert_eq \
    "an ordered handoff pair (W03 -> W04, joined by depends-on) is silent" \
    "$(grep -c 'W03 handoff names W04' "$temporary_root/out.log" || true)" \
    0

t_assert_eq \
    "a history-marked paragraph naming an unordered unit (W04 -> W02) is exempt" \
    "$(grep -c 'W04 handoff names W02' "$temporary_root/out.log" || true)" \
    0

t_end
