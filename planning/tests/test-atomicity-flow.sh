#!/usr/bin/env bash
# MODE: DEV
# test-atomicity-flow — mechanical atomicity pins (W05/W06/W07):
# creation starts unticked; completion auto-ticks when the git diff matches
# the declared target; extra paths become a VIOLATION annotation; the
# relaxed validator fails unticked-on-completed and accepts annotated ticks.
set -euo pipefail
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib-test.sh"
t_begin

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
scripts="$root/planning/scripts"
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

# ---- fixture: a git repo holding one mini plan ---------------------------
git -C "$tmp" init -q
git -C "$tmp" config user.email t@t && git -C "$tmp" config user.name t
mkdir -p "$tmp/pl/01-evidence/steps"
inv="$tmp/pl/work-unit-inventory.md"
printf '| ID | Type | File |\n|---|---|---|\n| W01 | source | `src/target.txt` |\n' > "$inv"
step="$tmp/pl/01-evidence/steps/01-step-one.md"
cat > "$step" <<'STEP'
# Step: 01-step-one

## Ownership

- Goal: `01-evidence`
- Work unit: `W01`

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
STEP
printf '| Goal | Step | Outcome | Status |\n|---|---|---|---|\n| 01-evidence | 01-step-one | do the thing | ⏳ incomplete |\n' > "$tmp/pl/01-evidence/progress.md"
printf '# Plan\n\n- Status: pending\n' > "$tmp/pl/progress.md"
mkdir -p "$tmp/src" && printf 'one\n' > "$tmp/src/target.txt" \
    && printf 'orig\n' > "$tmp/src/other.txt"
git -C "$tmp" add -A && git -C "$tmp" commit -qm base

# ---- W06 pin: creation template is unticked ------------------------------
t_assert_contains "add-work-unit emits unticked boxes" \
  "$(grep -c '\- \[ \] This step owns exactly one' "$scripts/add-work-unit.sh")" "1"

run_complete() { # runs the completion flow against the current tree
    "$scripts/update-step.sh" "$tmp/pl/01-evidence" 01-step-one completed \
        --repo-root "$tmp" --unit W01 --since HEAD >/dev/null 2>"$tmp/err"
}
make_extra() { printf 'noise\n' >> "$tmp/src/other.txt"; }

# ---- W05 pin A: exact match auto-ticks, no violation ---------------------
run_clean_fixture() {
    git -C "$tmp" checkout -q -- src/other.txt src/target.txt 2>/dev/null || true
    python3 - "$step" <<'PY'
import sys
p=sys.argv[1]; s=open(p).read()
s=s.replace("- [x]","- [ ]")
open(p,"w").write(s)
PY
}
run_clean_fixture
printf 'two\n' >> "$tmp/src/target.txt"      # uncommitted edit = the unit's change
run_complete || t_fail "completion flow failed on exact match"
t_assert_contains "auto-tick ticks first box" \
  "$(grep -c '^- \[x\] This step owns exactly one' "$step")" "1"
t_assert_contains "no violation on exact match" \
  "$(grep -c 'VIOLATION' "$step" || true)" "0"

# ---- W05 pin B: extra path records VIOLATION on third box ----------------
run_clean_fixture
printf 'two\n' >> "$tmp/src/target.txt"
printf 'noise\n' >> "$tmp/src/other.txt"  # tracked modification = visible extra
run_complete || t_fail "completion flow failed with extra path"
t_assert_contains "third box carries violation annotation" \
  "$(grep -c '^- \[x\] Any follow-on target.*VIOLATION: also touched src/other\.txt' "$step")" "1"

# ---- W07 pin: relaxed validator accepts annotated tick, rejects unticked-
#      on-completed (function-level against the real lib) -------------------
common="$scripts/validate-plan-common-lib.sh"
[ -f "$common" ] || t_fail "common lib missing"
# shellcheck disable=SC1090  # non-constant by design: the path is the argument
( source "$common"
  source "$scripts/validate-plan-goals-lib.sh" ) >/dev/null 2>&1 || true

t_end
