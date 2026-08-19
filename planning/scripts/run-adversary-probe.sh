#!/usr/bin/env bash
# run-adversary-probe.sh — reusable adversarial-review probe.
#
# Materializes the versioned adversary-probe fixture (planning/tests/fixtures/
# adversary-probe) to a working directory, initializes the gated-reader
# snapshot, sanity-checks that every entry id the probe relies on is served by
# the CURRENT reader, and prints the exact spawn prompt for a fresh adversarial
# reviewer.
#
# Usage:
#   run-adversary-probe.sh [<working-dir>]
#   run-adversary-probe.sh --help
#
# The working copy (default under the planning-agent temp dir) is where a
# reviewer writes its verdict; the committed fixture is never mutated.
#
# Exit codes: 1 = the materialized probe is not usable with the current reader;
# 66 = the committed fixture is missing.

set -euo pipefail
export LC_ALL=C

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=planning/scripts/plan-inventory-lib.sh
source "$script_dir/plan-inventory-lib.sh"

usage() {
    local rc="${1:-64}"
    cat <<USAGE
Usage: ${0##*/} [<working-dir>]
       ${0##*/} --help
USAGE
    exit "$rc"
}

case "${1:-}" in
    -h|--help) usage 0 ;;
    -*) printf '%s: unknown option: %s\n' "${0##*/}" "$1" >&2; usage ;;
esac
[ "$#" -le 1 ] || usage

fixture="$script_dir/../tests/fixtures/adversary-probe"
working="${1:-${PLANNING_AGENT_TMPDIR:-${TMPDIR:-/tmp}/planning-agent}/adversary-probe}"

[ -f "$fixture/FIXTURE-VERSION" ] || { printf 'probe fixture missing: %s\n' "$fixture" >&2; exit 66; }

rm -rf "$working"
mkdir -p "$working"
# PORTABILITY(cp-dot-source): tar carries the dotfiles the fixture needs.
(cd "$fixture" && tar cf - .) | (cd "$working" && tar xf -)
rm -f "$working/FIXTURE-VERSION" "$working/README.md"

reader="$script_dir/plan-context.sh"
bash "$reader" init --plan-dir "$working" >/dev/null

fail=0
read_doc() {
    local id="$1"
    if bash "$reader" read --plan-dir "$working" --document "$id" >/dev/null 2>&1; then
        printf '  gate serves --document %s\n' "$id"
    else
        printf '  FAIL: gate does not serve --document %s\n' "$id" >&2
        fail=1
    fi
}

printf '=== gated-reader sanity check on materialized probe ===\n'
read_doc plan
read_doc inventory
read_doc progress
read_doc adversarial-review
for goal_dir in "$working"/*/; do
    id="$(basename "$goal_dir")"
    [ "$id" = context ] && continue
    read_doc "goal:$id"
done
units="$(plan_inventory_rows "$working/work-unit-inventory.md" | cut -f 1)"
for unit in $units; do
    if bash "$reader" read --plan-dir "$working" --unit "$unit" >/dev/null 2>&1; then
        printf '  gate serves --unit %s\n' "$unit"
    else
        printf '  FAIL: gate does not serve --unit %s\n' "$unit" >&2
        fail=1
    fi
done
grep -Fq -- '- Status: pending' "$working/adversarial-review.md" || { printf '  FAIL: fixture is not a reusable pending stub\n' >&2; fail=1; }

[ "$fail" -eq 0 ] || { printf 'probe fixture is not usable with the current reader (update it; no backwards compatibility)\n' >&2; exit 1; }

printf '\n=== spawn a fresh adversarial reviewer with this starting prompt ===\n'
cat <<PROMPT
You are a fresh, independent adversarial reviewer for a benchmark plan. You are NOT the author of the plan.

YOUR PERSONA: ROLE_ID=chris (oriented scout). Adopt this identity: load your scoped role docs and voice via
  ROLE_ID=chris bash $script_dir/role-context.sh chris
(which injects your stance preamble). State your persona id in your self-report. A fresh adversary forms its own findings.

THE REQUEST the plan must satisfy:
"Add a GET /health endpoint to the example service that returns {\"status\":\"ok\"} with HTTP 200."

THE PLAN to review is at exactly:
  $working

MANDATORY READING RULES:
1. SKILL-LOCK: Do not load any skill on your own. Use only what is named in this prompt. Do not infer skills from paths like .plans/.
2. BOUNDED-READ: Read plan files and artifacts ONLY through the gated reader:
     bash $reader read --plan-dir $working --document ID
     bash $reader read --plan-dir $working --unit WNN
   Valid --document IDs: plan, inventory, progress, adversarial-review, goal:<goal id>, step:<goal>/<step>. Use the default summary view; raise --max-records/--max-bytes for a large view if needed. Never load a whole plan file, the whole plan directory, or the .plans/ tree wholesale (no Read/cat/find on plan artifacts). If the gate cannot give you something, report it as a limitation — do not bypass it.

YOUR TASK:
1. Read the plan through the gate: --document plan, inventory, progress, goal:<each goal>, and --unit <each WNN>.
2. Identify unplanned files, symbols, behaviors, tests, dependencies, and verification gaps for the request.
3. Write findings + a verdict (✅ approved / rejected) to $working/adversarial-review-incoming.md.
   That is the ONLY file you may write. Never edit adversarial-review.md — the
   maintainer's update-adversarial-review.sh consumes your incoming file, mints
   the fix keys and archives the result.

BEHAVIOR SELF-REPORT (the most important output — be precise):
- For every read: the exact command used and what it returned (or an error).
- Which entry ids the gate served successfully; any it refused.
- Did you read any plan artifact wholesale (Read/cat)? List exactly which.
- Any friction or missing capability, and what you did instead.
PROMPT

printf '\nWorking probe: %s (reviewer writes its verdict there; committed fixture untouched)\n' "$working"
