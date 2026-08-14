#!/usr/bin/env bash
# Reusable adversarial-review probe.
#
# Materializes the versioned adversary-probe fixture (planning/tests/fixtures/
# adversary-probe) to a working directory, initializes the gated-reader
# snapshot, sanity-checks that every entry id the probe relies on is served by
# the CURRENT reader, and prints the exact spawn prompt for a fresh adversarial
# reviewer.
#
# Usage:
#   planning/scripts/run-adversary-probe.sh [working-dir]
#
# The working copy (default under the planning-agent temp dir) is where a
# reviewer writes its verdict; the committed fixture is never mutated.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FIXTURE="$SCRIPT_DIR/../tests/fixtures/adversary-probe"
WORKING="${1:-${PLANNING_AGENT_TMPDIR:-${TMPDIR:-/tmp}/planning-agent}/adversary-probe}"

[ -f "$FIXTURE/FIXTURE-VERSION" ] || { echo "probe fixture missing: $FIXTURE" >&2; exit 66; }

rm -rf "$WORKING"
mkdir -p "$WORKING"
cp -R "$FIXTURE"/. "$WORKING"/
rm -f "$WORKING/FIXTURE-VERSION" "$WORKING/README.md"

READER="$SCRIPT_DIR/plan-context.sh"
bash "$READER" init --plan-dir "$WORKING" >/dev/null

fail=0
read_doc() {
    local id="$1"
    if bash "$READER" read --plan-dir "$WORKING" --document "$id" >/dev/null 2>&1; then
        echo "  gate serves --document $id"
    else
        echo "  FAIL: gate does not serve --document $id" >&2
        fail=1
    fi
}

echo "=== gated-reader sanity check on materialized probe ==="
read_doc plan
read_doc inventory
read_doc progress
read_doc adversarial-review
for goal_dir in "$WORKING"/*/; do
    id="$(basename "$goal_dir")"
    [ "$id" = context ] && continue
    read_doc "goal:$id"
done
units="$(awk -F'|' 'function trim(v){gsub(/^[[:space:]]+|[[:space:]]+/,"",v); return v} /^\|[[:space:]]*W[0-9][0-9]+[[:space:]]*\|/ {print trim($2)}' "$WORKING/work-unit-inventory.md")"
for unit in $units; do
    if bash "$READER" read --plan-dir "$WORKING" --unit "$unit" >/dev/null 2>&1; then
        echo "  gate serves --unit $unit"
    else
        echo "  FAIL: gate does not serve --unit $unit" >&2
        fail=1
    fi
done
grep -Fq -- '- Status: pending' "$WORKING/adversarial-review.md" || { echo "  FAIL: fixture is not a reusable pending stub" >&2; fail=1; }

[ "$fail" -eq 0 ] || { echo "probe fixture is not usable with the current reader (update it; no backwards compatibility)" >&2; exit 1; }

echo
echo "=== spawn a fresh adversarial reviewer with this starting prompt ==="
cat <<PROMPT
You are a fresh, independent adversarial reviewer for a benchmark plan. You are NOT the author of the plan.

YOUR PERSONA: ROLE_ID=chris (oriented scout). Adopt this identity: load your scoped role docs and voice via
  ROLE_ID=chris bash $SCRIPT_DIR/role-context.sh chris
(which injects your stance preamble). State your persona id in your self-report. A fresh adversary forms its own findings.

THE REQUEST the plan must satisfy:
"Add a GET /health endpoint to the example service that returns {\"status\":\"ok\"} with HTTP 200."

THE PLAN to review is at exactly:
  $WORKING

MANDATORY READING RULES:
1. SKILL-LOCK: Do not load any skill on your own. Use only what is named in this prompt. Do not infer skills from paths like .plans/.
2. BOUNDED-READ: Read plan files and artifacts ONLY through the gated reader:
     bash $READER read --plan-dir $WORKING --document ID
     bash $READER read --plan-dir $WORKING --unit WNN
   Valid --document IDs: plan, inventory, progress, adversarial-review, goal:<goal id>, step:<goal>/<step>. Use the default summary view; raise --max-records/--max-bytes for a large view if needed. Never load a whole plan file, the whole plan directory, or the .plans/ tree wholesale (no Read/cat/find on plan artifacts). If the gate cannot give you something, report it as a limitation — do not bypass it.

YOUR TASK:
1. Read the plan through the gate: --document plan, inventory, progress, goal:<each goal>, and --unit <each WNN>.
2. Identify unplanned files, symbols, behaviors, tests, dependencies, and verification gaps for the request.
3. Write findings + a verdict (✅ approved / rejected) to $WORKING/adversarial-review.md.

BEHAVIOR SELF-REPORT (the most important output — be precise):
- For every read: the exact command used and what it returned (or an error).
- Which entry ids the gate served successfully; any it refused.
- Did you read any plan artifact wholesale (Read/cat)? List exactly which.
- Any friction or missing capability, and what you did instead.
PROMPT

echo
echo "Working probe: $WORKING (reviewer writes its verdict there; committed fixture untouched)"