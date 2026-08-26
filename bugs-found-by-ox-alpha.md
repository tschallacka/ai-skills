# Bugs found by ox-alpha

Findings while exercising the planning skill (`~/.config/opencode/skills/planning`)
and its helpers on the health-app VPS, August 2026. Each entry: where, what
happened, why it bites, and the workaround that proved safe.

## 1. `dep_reaches` memo poisoning in `validate-plan-propagation-lib.sh`

- **Where:** `scripts/validate-plan-propagation-lib.sh`, `dep_reaches()`
  (used by `plan_validate_propagation_reach`).
- **What happened:** A verification unit (W17) with a DIRECT dependency edge
  to a same-goal unit (W15) was reported as
  `W17 is a verification unit that grades W15 but has no dependency path to it`
  even though `work-unit-inventory.md` listed `W15` in W17's Depends-on cell.
  A standalone probe of the same pair returned success.
- **Root cause:** `dep_reaches` memoizes every visited `(from,to)` pair as a
  failure: it does `plan_map_has dep_seen "$key" && return 1` **before**
  exploring, then `plan_map_set dep_seen "$key" 1` immediately. The memo does
  not distinguish "explored and failed" from "explored and succeeded". When a
  pair is first traversed inside an *elif reverse-edge* branch of an earlier
  check (e.g. scanning unit A whose step mentions B, probing
  `dep_reaches(B,A)` which succeeds), the `(B,A)` key is left in the memo;
  the later direct check for (B,A) hits the memo, gets `return 1`, and its own
  fallback direction was already memoized as failed during the forward attempt.
  Result: a false FAIL for a pair that has both edges satisfiable.
- **Repro shape:** two verification steps in one goal each name the other's
  unit ID; validation fails the second pair despite valid edges; extracting
  the function into a fresh shell passes.
- **Workaround:** never mention another work unit's ID inside a verification
  step's narrative prose (the scanner treats any `WNN` word in the step file
  as a grading claim). Reword to "the dual build gate", "the story-two
  record", etc., so no cross-pair probes are generated.
- **Suggested fix:** only treat the memo as a failure cache when exploration
  actually completed without success (set the memo on the failure return
  paths instead of at entry), or clear the key before returning success.

## 2. `update-adversarial-review.sh` CSV parser chokes on reviewer file headers

- **Where:** `scripts/update-adversarial-review.sh` consuming
  `adversarial-review-incoming.md`.
- **What happened:** The incoming file written per protocol (title line
  `# Adversarial review incoming findings — <persona>`, then header row, then
  CSV rows) failed with `CSV row 2 has 1 columns, expected 5`. The exact same
  rows parsed cleanly once the comment/title lines were stripped and the file
  passed via `--file`.
- **Why it bites:** reviewers following the documented write format get their
  findings rejected; the error message points at "row 2" (the real header)
  rather than the offending preamble.
- **Workaround:** strip everything above the `ID,...` header line into a temp
  CSV and run `update-adversarial-review.sh <plan> --file clean.csv`.
- **Suggested fix:** skip blank lines and lines starting with `#` before CSV
  parsing.

## 3. `add-work-unit.sh` partial application on goal-roster failure

- **Where:** `scripts/add-work-unit.sh`.
- **What happened:** When the owning goal's `Owned work units` section had
  been emptied earlier (all units removed), `add-work-unit.sh` wrote the new
  inventory row and step file first, then aborted with `Goal has no numbered
  Owned work units section`. The retry then failed with `Work-unit ID already
  exists`, leaving the unit half-created (row + step exist, roster missing).
- **Why it bites:** recovery is not obvious; you must re-create the §9.1
  roster paragraph via `update-plan-content.sh -gs ... owned-work-units` and
  then use `update-work-unit.sh` (not add) for the stranded ID.
- **Workaround:** after any add/remove failure, check the inventory for the
  stranded ID before retrying; recreate the roster section first.
- **Suggested fix:** validate goal structure before mutating inventory/step,
  or roll back the partial writes on abort.

## 4. `remove-work-unit.sh` deletes the whole Owned-units section when last unit goes

- **Where:** `scripts/remove-work-unit.sh`.
- **What happened:** Removing the final remaining unit from a goal removed
  the entire numbered `Owned work units` section rather than leaving an empty
  scaffold, which then breaks subsequent `add-work-unit.sh` calls (see #3).
- **Workaround:** re-create the section with
  `update-plan-content.sh -gs <plan> <goal> owned-work-units -p 9.1: "..."`
  before re-adding units.
- **Suggested fix:** leave the section header plus an empty §9.1 in place.

## 5. `mint-fix-keys.sh` gated-row failure leaves review table updated without keys

- **Where:** `update-adversarial-review.sh` → `mint-fix-keys.sh` chain.
- **What happened:** A findings row with non-conforming Work unit cell
  (`W01-W18`) produced `WARN skipping gated row ... could not be minted`,
  exited nonzero, and did NOT write `fix-keys.json` — but the Findings table
  rewrite (and history archive) had already been applied. State ended up
  half-migrated.
- **Why it bites:** the approval gate later demands claims for keys that were
  never minted, and the operator must notice that the table update stuck.
- **Workaround:** normalize Work unit cells to a single `WNN` or `N/A`
  before running; re-run after fixing the CSV; verify with
  `verify-fix-keys.sh <plan> --claimed-by <session>`.

## Non-bugs worth remembering (documented behavior that surprised)

- `create-ui-story-run-cache.sh` reports `already exists` if
  `add-ui-story.sh` pre-created the cache — harmless, not an error.
- Once a goal's testing requirement flips to `yes`, EVERY unit in that goal
  needs a `-testing.md` companion, including pure source steps; budget for
  `create-step-testing.sh` calls per unit.
- Verification-step prose is scanned for `WNN` tokens anywhere in the file
  (including handoff sections); see #1 for the safest authoring rule.

## 6. Self-certification gate cannot distinguish sessions in opencode harness

- **Where:** `verify-fix-keys.sh` self-certification check (minted_by vs
  claiming session).
- **What happened:** Subagent reviewers spawned via opencode's Task tool share
  the coordinator's derived session id (`a7fb…`), so even when the reviewer
  runs the mint itself the gate sees minted_by == claimer and refuses.
- **Workaround:** documented `MINTED_BY` override to attribute the mint to the
  reviewer role identity (`MINTED_BY=chris-reviewer-cycles-1-4`).
- **Suggested fix:** accept explicit MINTED_BY as legitimate attribution and
  document it at the gate, or derive session identity from something subagents
  do not share.

## 7. No helper to mark a step completed in goal progress.md

- **Where:** planning skill scripts (gap, not a defect).
- **What happened:** After executing a step there is no sanctioned command to
  flip its row from `💤 incomplete` to `✅ completed` in `<goal>/progress.md`.
  I hand-edited with sed ~40 times across two plans; `update-progress.sh`
  only recomputes percentages from whatever the table says.
- **Suggested fix:** `complete-step.sh <goal-dir> <step-name>` (or a
  `--set-status` flag on update-progress.sh) that edits the one row and then
  recomputes.

## Minor notes (non-bugs)

- `update-adversarial-review.sh` rejects pipe characters inside CSV cells —
  reasonable for table safety, but reviewer prose that quotes table syntax
  needs `\|` escaping guidance in the reviewer brief template.
- It rejects CRLF CSVs with a clear actionable error (good); note for
  coordinators: python `csv.writer` defaults to `\r\n`, so strip CR before
  handing files to the helper.
