# Internal planning-style contract

This is an internal maintainer document. It is not part of the agent-facing
planning instructions. Read it when changing a planning helper, validator, or
generated artifact; keep the scripts, validator, tests, and this contract in
sync.

## Source of truth

- Creation scripts define canonical document order, headings, tables, labels,
  and initial values.
- `plan-document-lib.sh` defines document IDs, mutable section mappings,
  paragraph replacement, safe values, and testing reminders.
- `update-plan-content.sh` accepts flagged targets only. Paragraph flags target
  one `N.N` paragraph; section flags accept repeated `-p N.N: content` values.
- `--table-paragraph` replaces one paragraph with a Markdown table rendered
  from quoted CSV. Its explicit column count must match every CSV row, and the
  first row is emitted as the header. Literal quotes use CSV-standard doubled
  quotes (`""`); shell-friendly backslash-escaped quotes (`\"`) are also
  accepted.
- `--insert-after` and `--insert-before` add one paragraph around a target and
  renumber only later paragraphs in that target's section. Other sections keep
  their numbering.
- `validate-plan.sh` is the readiness and completion gate. Do not weaken it to
  accommodate a malformed fixture; update the fixture through the helpers.
- Every hard rule needs a regression assertion in `planning/tests/`.
- `plan-context.sh` and `plan-context-lib.sh` own bounded snapshot reads,
  SHA-256 freshness checks, processed-entry state, and context writes.
  Context IDs are tagged (`plan`, `goal:<id>`, `step:<goal>/<step>`, and
  `unit:WNN`); do not replace them with ambiguous positional arguments.
- Hash drift is a suspect external edit. Helpers report it for human
  intervention and must not silently overwrite or repair the document.

## Historical evidence and protocol boundaries

- Benchmark reports and archives are immutable evidence of the code and
  protocol that produced them.
- Do not rerun an older version or retrofit an older report to satisfy a newer
  protocol. This project does not add backward compatibility for that purpose.
- Compare archived data as-is only when its task, revision, metadata, and
  evidence boundaries are compatible. Otherwise record the comparison as
  unavailable or contextual, and run only the current protocol when new
  evidence is genuinely required.

## Names, IDs, and paths

- Plan directories: lowercase kebab case.
- Goal directories: `NN-lowercase-kebab-case`.
- Work units: `WNN` or a larger zero-padded numeric suffix.
- Step files: `NN-step-lowercase-kebab-case.md`.
- Testing companions: the same step filename with `-testing.md` appended.
- User stories: `US-NN`; UI bugs: `BUG-NN`; adversarial findings: `AR-NN`.
- Inventory file paths name one concrete file; `N/A` is valid only for
  verification work units. Globs, directories, comma-separated symbols, and
  `and`-joined scopes are invalid.

## Paragraph contract

- A paragraph label is exactly `§ N.N`.
- The label is preceded by a blank line and followed by the paragraph on the
  next line.
- A section replacement numbers paragraphs from `.1` sequentially and uses
  the section's fixed prefix. A paragraph replacement changes exactly one
  existing label.
- Paragraph text is one line, non-empty, and contains no paragraph marker `§`.
  Quote text containing spaces or argument-like strings so the shell passes it
  as one argument. Table values cannot contain `|` or newlines.
- A table paragraph is the controlled multiline exception: its CSV rows become
  a header row, separator row, and data rows under one paragraph label.
- Paragraphs are created or replaced through the flagged update helper; do not
  add a second numbering scheme or hand-reflow labels.

## Canonical documents

`create-plan.sh` must emit, in this order:

1. Plan narrative sections: Current state, Desired outcome, Approach, Scope,
   Affected areas, Constraints and decisions, Risks and open questions.
2. UI classification.
3. Adversarial review status.

`add-goal.sh` must emit the goal narrative sections, Owned work units,
Testing requirement, and Goal-size exception. Its narrative labels are
`§ 2.1` through `§ 9.1`, then `§ 11.1` after the structured testing table.

`add-work-unit.sh` must emit Ownership, Change target, Objective, Instructions,
Acceptance criteria, Handoff, and the three checked Atomicity check lines.
Ownership fields must mirror the inventory row exactly.

`create-adversarial-review.sh` must emit Review scope, Findings, and Verdict.
The review status and plan-description status are synchronized atomically.

## Table schemas

Every generated Markdown table has a fixed header, separator row, column
count, and cell order. Preserve these exactly:

- Definition-of-done coverage: outcome, work-unit IDs, notes.
- Work units: ID, type, file, primary scope, subscope, intended change,
  dependencies, goal, step.
- Goal testing: Test required, Rationale; value is `yes` or `no`.
- Plan progress: Goalname, Description, Completion status.
- Goal progress: Goalname, Stepname, Description, Completion status.
- Review findings: ID, missing/over-broad item, required plan change, status.
- UI stories: ID, persona/precondition, browser actions, interaction evidence,
  expected result, status, evidence, related work units, run cache.
- UI bugs: ID, story, severity, reproduction/evidence, investigation goal,
  fix goal, retest story, status.
- Browser cache interaction rows: order, direct UI input, target/value,
  readiness signal.
- Browser cache wait rows: after order, wait/condition, maximum wait, observed
  result.

Do not add decorative columns, omit empty cells, or put an unescaped `|` in a
cell. Use the dedicated helper for structured table mutations.

## Status and progress values

- Plan/goal/step progress: `💤 incomplete`, `⏳ in progress`, `✅ completed`.
- Adversarial review: `💤 pending` or `✅ approved`.
- UI stories: `💤 untested`, `⏳ in progress`, `✅ passed`, `🐛 bug found`, or
  user-approved `⏭️ excluded`.
- UI run caches: `💤 untested`, `✅ passed`, or an explicitly recorded
  exclusion.
- Progress bars use the generated 20-character `#`/`-` bar and matching icon;
  never hand-edit percentages.

## Ownership and testing gates

- Each inventory work unit has exactly one goal and one step.
- Each step owns exactly one work unit and one concrete target.
- Goals normally contain 2–10 work units; single-unit exceptions are limited
  to the validator's allowed types and require the Goal-size exception section.
- A goal marked `Test required: yes` owns a `test` or `verification` unit,
  every implementation unit has downstream proof, and every non-documentation
  step has a testing companion.
- A goal marked `no` must state why testing is not meaningful or possible. A
  goal containing test/verification work must say `yes`.
- Updating a step through the CLI must remind the agent to review an existing
  testing companion for accuracy and completeness.

## UI testing limits

- UI stories require direct rendered input: click, tap, typing, keyboard input,
  swipe, pinch, or drag.
- Console/JavaScript evaluation, storage edits, direct API calls, injected
  events, and internal functions are not valid story evidence.
- UI caches record actions in order, readiness conditions, maximum/actual wait,
  evidence, and cache validity. Dedicated UI helpers own their table shape.
- A failed story remains a bug with linked investigation, fix, and retest
  goals; passing a story does not erase its bug history.

## Maintenance checklist

When changing a generated format:

1. Update the creating helper and any parser/validator that consumes it.
2. Update the flagged mutation helper if the document is mutable.
3. Add or update a regression fixture covering the positive and malformed
   forms.
4. Run shell syntax checks, `git diff --check`, and all bounded planning tests.
5. Keep this contract precise and keep agent-facing prose limited to workflow
   decisions rather than repeating these implementation details.
