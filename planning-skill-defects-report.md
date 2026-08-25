# Planning-skill defect report — add-work-unit progress destruction, atomicity checkboxes, adversarial-review consumer

Filed August 2026 against `refactor/portability-and-installer-ox`. Every claim
below was reproduced on a live session; reproduction commands are included per
defect. This file is the long-form record; the machine-readable entries live in
`BUGS.json` (B40–B45) and reference back here.

---

## Trap A — add-work-unit.sh silently destroys a goal's progress record

Severity: high. Confirmed, and broader than originally claimed — it also
un-completes the goal row in the plan-level tracker.

### What happens

`add-work-unit.sh:119-120` calls `plan_rebuild_goal_progress`, which deletes
rather than amends:

```
# plan-reconcile-lib.sh:131-142
progress_file="$goal_dir/progress.md"
if [ -f "$progress_file" ]; then
    rm -f "$progress_file"
    "$script_dir/create-progress.sh" "$goal_dir" "$goal" >/dev/null 2>&1
```

`create-progress.sh` initialises every row 💤 incomplete. No status is read
before the rm. `plan_rebuild_plan_progress` then recomputes the plan-level
goal row from the wiped tracker, so a ✅ completed goal reads 💤 incomplete at
plan level too.

### Reproduction

Goal with 01-step-one completed, 02-step-two in progress:

```
$ add-work-unit.sh $P W03 test test/CTest.php 'CTest::testThree()' N/A "test three" 'W02' 01-alpha 03-step-three
Added W03 and .../probe/01-alpha/steps/03-step-three.md

$ cat probe/01-alpha/progress.md
**Progress:** `0%  #### ----------------  100%` 💤
| 01-alpha | 01-step-one   | change one | 💤 incomplete |
| 01-alpha | 02-step-two   | change two | 💤 incomplete |
```

Both statuses gone. On a fully-complete goal, `probe/progress.md` flipped
✅ completed → 💤 incomplete.

### Conditions

Every add, unconditionally — no flag, no prompt, no dry-run. A tracker must
exist, but that is not a real precondition: `add-goal.sh` creates one, so from
the second unit onward every add resets. Goal completion state is irrelevant.

### What is lost

Statuses only — but that is the entire payload; the schema has no notes or
timestamp column. The progress bar and goal glyph reset with them.

### Output hints: none

The damning comparison is that the sibling helper doing the identical rebuild
does warn — `remove-work-unit.sh:67` prints
`note: goal progress was rebuilt from step files; re-apply completion statuses with update-step.sh`.
`add-work-unit.sh` has no equivalent.

### Git recovery fails in this layout

`plan_git_snapshot` guards on `[ -d "$plan_dir/.git" ]`
(`plan-document-lib.sh:61-68`). Because `.plans/` is gitignored here, the
repo is at `.plans/`, not `.plans/<plan>/` — so every snapshot silently
no-ops. Proven independently: after ~14 helper mutations,
`git -C .plans status --porcelain` listed all 16 touched files as
uncommitted. The skill's "every mutating helper commits first" guarantee is
false in this layout.

### Impact

A completed goal reads as not started, so a resumed session following SKILL.md
§4 re-executes finished work. And §4.1.1 requires converting discovered scope
into inventory rows mid-execution — so the defect is aimed precisely at the
moment a tracker is most populated.

### Fix belongs at

- `plan-reconcile-lib.sh:131-142` — carry statuses across by step name (primary)
- `add-work-unit.sh:119-120` — warn, as a stopgap
- `plan-document-lib.sh:61-68` — the plans-root-repo blind spot
- `SKILL.md:446-448` and `:1181` — do not mention that this helper rewrites trackers at all

---

## Trap B — atomicity checkboxes are mandatory, unwritable, and not decorative

Severity: medium. The "no writable surface" half is confirmed exactly. The
"decorative" half was wrong — and the truth is worse.

### What happens

The three ticks live in the step file under `## Atomicity check`, with no
§ N.N labels and no `plan_section_spec` entry. They are written pre-ticked
once, at creation (`add-work-unit.sh:58`). A step exposes only four mutable
sections (`plan-document-lib.sh:202-205`): objective, instructions,
acceptance-criteria, handoff.

Every helper refuses:

```
$ update-plan-content.sh -ss $P 01-alpha/01-step-one atomicity-check -p 8.1: "x"
Section 'atomicity-check' is not a mutable narrative section for a step document.
Valid step section ids: objective instructions acceptance-criteria handoff

$ update-plan-content.sh -f $P step:01-alpha/01-step-one 'No other file, symbol, test target, or verification flow changes here.' 'no'
Field was not found exactly once: ...
```

`plan-mutate.sh` lists 18 operations, none touching atomicity;
`update-step.sh` writes status only. `grep -rln atomicity` over the script
tree returns nothing — the only Atomicity hits are the writer at
`add-work-unit.sh:58` and a heading check at `validate-plan.sh:755`.

### Not decorative — a hard gate

`validate-plan.sh:765-767` asserts all three with `grep -Fqx`. Untick one →

```
FAIL: ... has not confirmed target isolation
```

And because the match is whole-line fixed-string, a box that stays ticked with
an honest annotation appended also fails:

```
- [x] No other file, symbol, test target, or verification flow changes here. NOTE: the landed commit touched seven files.
→ FAIL: ... has not confirmed target isolation
```

So the validator reads the state, the only passing value is all three ticked
and byte-exact, and no helper can produce any other value. The boxes are a
constant, not evidence — every step in every valid plan asserts atomicity was
respected whether or not it was. `plan-content.sh get step:…` renders them to
a reviewer as if they were findings, which makes the constant actively
misleading.

That is the live case: AR-157 reads "W50's step ticks single-file atomicity
while its commit changed seven files."

### Fix belongs at

- `plan-document-lib.sh:202-205` — add `step/atomicity-check` (primary)
- `validate-plan.sh:765-767` — relax `-Fqx` to allow a recorded violation with its reason and FAIL on it, or delete the three lines and rely on the mechanical checks at :756-764
- `add-work-unit.sh:58` — emit unticked boxes so a tick is an act, not a default
- `SKILL.md:598` — the doc line that made this invisible

---

## Collateral defects in update-adversarial-review.sh

Surfaced while fixing the stale phrases — three real defects plus one cosmetic,
in the only helper that can touch the findings table.

### C1. Incoming findings are consumed even when passed --file

It runs `rm -f adversarial-review-incoming.md` unconditionally, even when
passed `--file`. In the reporting session that file held the ten un-consumed
cycle-18 findings. The worker copied it aside and restored it
byte-identically — all ten rows verified present and the plans repo clean —
but an unaware caller loses them silently.

### C2. Re-minted fix keys under a new session id

The old keys were already stale, so nothing verifiable was lost, but
re-approval needs fresh claims.

### C3. Cycle numbering breaks after gaps

Cycle numbering is `grep -c '^## Cycle ' + 1`, so it archived the prior table
as `## Cycle 11` after Cycle 16. Confirmed in the file.

### C4. CRLF mislabelled as a column error

`--file` rejects Python's default csv.writer output with
`CSV table must have 5 columns.` The real cause is CRLF —
`plan_render_csv_table` exits 4 on `/\r/` but reuses the column/pipe message.

### Stale check now passes properly

14 paragraphs enumerated explicitly, byte-identical replaced with
extracted-text comparisons, no whitelist touched. `validate-plan.sh` and
`--stale default` both exit 0 with only the two pre-existing graph-leaf WARNs.
