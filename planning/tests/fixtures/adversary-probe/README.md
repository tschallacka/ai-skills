# Adversary-review probe fixture

A minimal, versioned dummy plan used to probe how a fresh adversarial reviewer
(and the planning skill's gated-reader + skill-lock safeguards) behaves. It is
deliberately tiny: the point is to observe the reviewer "finding its bearings"
(read discipline, skill loading, gate coverage), not to exercise a realistic
workload.

## Why it is versioned

The probe depends on the **current** planning-skill spec:

- the gated reader (`planning/scripts/plan-context.sh`) must serve every entry
  id the probe relies on (`plan`, `inventory`, `progress`,
  `adversarial-review`, `goal:<id>`, `step:<goal>/<step>`, `unit:WNN`);
- `planning/SKILL.md` section 3 defines the bounded-read/skill-lock starting
  prompt the probe passes to the reviewer;
- the work-unit inventory row schema must match what the reader parses (Goal in
  column 9, Step in column 10 of the pipe-delimited row).

`FIXTURE-VERSION` pins the skill version and the reader schema/generator
versions this fixture complies with.

## Maintaining this fixture (no backwards compatibility)

When the planning skill or the gated-reader spec changes:

1. Update the fixture's plan files to the **new** spec (e.g. reader entry-id
   set, inventory schema, step/goal layout).
2. Bump the matching values in `FIXTURE-VERSION`.
3. Run the compliance test:

   ```bash
   bash planning/tests/test-adversary-probe-fixture.sh
   ```

   It fails (exit non-zero) while the fixture is out of date. Do **not**
   maintain old-format fixtures or add compatibility shims — update in place.

4. Re-run the probe to confirm the reviewer still reads everything through the
   gate:

   ```bash
   bash planning/scripts/run-adversary-probe.sh
   ```

   and follow its printed spawn-prompt to drive a fresh adversarial reviewer.

`adversarial-review.md` is committed as a `pending` stub (the reusable starting
state); the runner materializes a working copy so the committed fixture is never
mutated by a probe run.
