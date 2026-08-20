<!-- MODE: PROD -->
<!-- PACKAGE: PROD -->
# UI user-story validation

Use this reference whenever a planning initiative creates, changes, repairs,
or validates a UI component, page, interaction, visual state, or user-facing
flow. Read it before researching the plan boundary or creating goals.

Treat user stories as executable acceptance contracts, not prose written after
implementation. The plan is not complete until every applicable story has
passed in the browser or has an explicit user-approved exclusion.

## Required UI artifacts

Use `create-ui-validation.sh` to add the required UI-validation section,
`ui-user-stories.md`, and `bugs.md`. Use `add-ui-story.sh` for each bounded
story and `configure-ui-story-cache.sh` for its cache. Do not recreate their
tables by hand; the helpers enforce the canonical shape.

Story statuses are:

- `💤 untested`
- `⏳ in progress`
- `✅ passed`
- `🐛 bug found`
- `⏭️ excluded` — requires the user’s explicit approval and its reason in
  `Evidence`

Never mark a story passed because an automated test passes. Browser evidence
is required for a UI story unless the user explicitly agrees that the story
cannot be exercised in the available environment.

An open bug row must never be removed or marked resolved merely by changing a
story to passed. Keep its evidence and link the retest that resolved it.

## Browser-first discovery and story design

Use the available Browser MCP tools before writing UI implementation steps.
Test as many stories as the current environment permits before changing code;
this is the default priority, not an optional polish pass.

### Direct-interaction rule

Every UI story must exercise the application through real user-facing browser
input. Its `Browser actions` and `Interaction evidence` must name at least one
of: a mouse click or selection, typed text, a keyboard action (for example
Tab, Enter, Escape, or an arrow key), or a mobile tap, swipe, pinch, or drag.
Perform that input through the rendered UI; record the control and action that
occurred. Opening a URL, taking a screenshot, inspecting the DOM, or reading
the console alone is not a user interaction and cannot pass a story.

Never use a console command, JavaScript evaluation, DevTools state mutation,
direct HTTP/API request, `localStorage`/`sessionStorage` edit, injected event,
or application-internal function to put a story into a passing state or to
trigger the behavior under test. These are invalid test evidence, even when
they are quicker than using the UI. Use the Browser MCP's normal navigation,
mouse, keyboard, and mobile-gesture tools instead.

Console and network panels may be inspected only to collect supplementary
diagnostic evidence after the UI interaction. They do not replace the
interaction or its observable result. If a required state cannot be reached
through the UI, record the limitation and ask the user whether a documented
test-data setup is acceptable; do not silently bypass the UI.

### Browser run cache and buffered input

Before running a story, create its cache at
`<planname>/ui-story-runs/US-<NN>.md`. Use the helper when available:

```bash
PLANNING_SKILL_DIR="<installed-planning-skill-directory>"
"$PLANNING_SKILL_DIR/scripts/create-ui-story-run-cache.sh" <plan-directory> US-01
```

For a new plan, prefer the command workflow: `create-ui-validation.sh` creates
the UI-validation section plus empty story and bug artifacts;
`add-ui-story.sh` adds a canonical story row and cache; and
`configure-ui-story-cache.sh` replaces that cache's template values before
normal plan validation. These commands avoid patches and keep the table and
cache shape validator-compatible.

The cache helper supplies the required headings and tables. Fill it only through
the helper or the approved content-update commands; do not patch its structure.

Cache every action in execution order, its target/value, and every wait. Prefer
a readiness condition over a blind delay; when a delay is unavoidable, record
both the maximum and actual elapsed wait. Do not cache secrets or credentials.

When the Browser MCP supports buffered, batched, or scripted input, submit the
whole `Buffered interaction sequence` as one browser operation, with the
recorded waits/readiness checks between actions. When it does not, execute the
same cached sequence exactly in order rather than re-planning individual
actions. The cache contains only normal mouse, keyboard, and mobile-gesture
input; it must never contain console, JavaScript, storage, direct-API, or
injected-event shortcuts.

Update the cache with actual timing and evidence after each run. Invalidate and
rebuild it when a UI change alters a target, starting state, route, timing, or
expected result. Do not reuse a cache after it fails until its failure and the
required correction are recorded.

### Revise after review

Use flagged plan-content targets for review corrections. For example, update a
step's acceptance criterion with
`--step-paragraph <plan> <goal>/<step> 6.1 "<revised criterion>"`, or replace
several paragraphs with `--step-section ... -p N.N: "<text>"`. Recheck the
matching testing companion after any step change. Use
`configure-ui-story-cache.sh` for structured story-cache fields; do not rewrite
its table by hand.

For a tabular acceptance record in a plan paragraph, use
`--table-paragraph <plan> <document-id> <N.N> <columns> "<CSV>"`; quote CSV
fields and represent literal quotes as doubled quotes (`""`) or shell-friendly
backslash escapes (`\"`). To add a
review-driven clarification without renumbering later sections, use
`--insert-after` or `--insert-before`; numbering shifts only within the target
section.

### Design fast, high-signal test steps

Make user-story validation fast by reducing repeated setup and waiting, never
by bypassing the rendered UI:

1. **Start from a stable route and persona.** Record one deterministic route,
   viewport, user account, and feature-flag state per story family. Reuse an
   authenticated browser context only while its state is known and the next
   story declares the same preconditions. When a story changes persistent UI
   state, restore it through the UI or start a fresh context before the next
   independent story.
2. **Factor shared UI setup.** Put common navigation and login/form actions in
   a named cache prefix, then append each story's one distinct interaction and
   assertion. The prefix still uses mouse, keyboard, or mobile input; it is
   reused as a buffered sequence, not replaced with a direct API or state
   setup.
3. **Use stable, user-visible targets.** Prefer accessible roles and names,
   visible labels, button text, form labels, and intentional test IDs over
   CSS position or volatile DOM structure. If a reliable target is absent,
   plan an atomic accessibility/semantics work unit instead of adding fragile
   coordinate-based actions to every story.
4. **Wait for readiness, not elapsed time.** Attach each interaction to the
   earliest observable condition that makes the next interaction safe: a
   control becomes enabled, a loading indicator disappears, navigation
   completes, focus moves, or a result appears. Set a short maximum timeout,
   record actual elapsed time, and remove excess wait after confirming the
   condition is sufficient.
5. **Batch independent stories deliberately.** Run caches sharing a stable
   starting state in a single buffered Browser MCP session, but preserve a
   separate run result, evidence, and pass/fail status for each story. Put
   destructive, permission-changing, checkout, logout, and responsive-device
   stories in their own resettable batches.
6. **Capture only useful evidence.** Record the route, interaction outcome,
   and one decisive screenshot or observable assertion per story. Capture
   additional screenshots, console logs, and network details only on failures
   or when they are needed to prove a requirement. This keeps successful runs
   quick without weakening their evidence.
7. **Run the cheapest discriminating stories first.** Start with the route
   load, primary action, validation/error path, and persistence/result path.
   Execute broader responsive, permission, empty-state, and edge-case stories
   next. Stop the affected batch as soon as a bug is found and use the bug
   feedback loop; rerunning known-failing downstream stories wastes time and
   obscures the root cause.
8. **Continuously tune the cache.** After a passing run, retain the exact
   targets and measured waits. After repeated successful runs, shorten only a
   recorded maximum wait that has evidence of being unnecessary. Do not remove
   a direct interaction or change a user-visible assertion merely to make the
   sequence faster.

#### Story execution and recording sequence

1. Navigate to the real route or use the project’s approved route-discovery
   method. Establish the relevant persona, data, viewport, and feature flags.
2. Capture the current state with browser evidence: the exact URL, visible
   UI, screenshots when useful, and console or network errors when the MCP
   exposes them.
3. Exercise existing flows rather than inferring them from source code. Include
   successful, empty, invalid, loading, error, permission, and responsive
   states whenever they are applicable to the requested change.
4. Write only stories that are observable and bounded: a named starting state,
   explicit direct browser actions, interaction evidence, and a concrete
   expected result. Split a flow when a failure can have a different cause or
   fix.
5. Record the evidence and current result for every exercised story in
   `ui-user-stories.md`. Add a separate verification work unit and step for
   every story that must be rerun after a change.

If the browser target, persona, intended behavior, design, or expected result
is ambiguous, stop and ask the user for clarification or input. Do not invent
an acceptance criterion or silently use production-like data to fill the gap.

## Incorporate stories into goals and steps

Map every UI story to the work-unit inventory. A story can depend on several
implementation work units, but its browser validation is a separate
`verification` work unit and atomic step. Do not bundle “test all stories”
into one step: one browser story or bounded flow is one verification step.

Add an ordered final UI-validation goal containing the remaining story
verification work units. Its definition of done is: every required story is
`✅ passed`, evidence is recorded, and no unresolved `🐛 bug found` rows
remain. Repeat relevant story checks after each working goal; run the complete
story suite after all implementation and fix goals are complete.

Before marking that final goal and the initiative complete, run the completion
gate as well as the normal structural validation:

```bash
PLANNING_SKILL_DIR="<installed-planning-skill-directory>"
"$PLANNING_SKILL_DIR/scripts/validate-plan.sh" --complete <plan-directory>
```

## Bug feedback loop

When a story fails unexpectedly, do not continue coding around it and do not
weaken the story merely to obtain a pass.

1. Change the story status to `🐛 bug found` and record exact reproduction,
   actual result, browser evidence, severity, and the story ID.
2. Add new numbered goals to the same plan using the normal planning skill:
   `NN-investigate-<bug-slug>` first, then `NN-fix-<bug-slug>` dependent on
   that investigation. The investigation goal establishes reproduction,
   affected work units, root cause or bounded uncertainty, and the required
   fix scope. The fix goal receives its own atomic work units, tests, and
   browser verification; do not append unplanned work to the current goal.
3. Update `plan-description.md`, `work-unit-inventory.md`, both progress
   trackers, and `ui-user-stories.md` so the new goals, dependencies, and
   verification ownership are explicit. Validate the revised plan before
   executing the new goals.
4. Complete the investigation and fix goals, then return to the failed story.
   Rerun it and any stories whose recorded dependencies include changed work
   units. Update the story only to reflect a confirmed requirement or
   user-approved behavior change; retain the bug history and evidence.

The `bugs.md` row must name the investigation goal, fix goal, and retest story
before either new goal starts. A normal plan validation fails if a `🐛 bug
found` story lacks this traceability; completion fails if any bug is open.

Classify a bug as severe when it blocks the primary user journey, causes data
loss or corruption, violates authorization or privacy expectations, prevents
the UI from loading, or makes further story results unreliable. When severe:

1. Mark the current goal `⏳ in progress` but pause its execution.
2. Insert the investigation and fix goals as the next execution priority.
   Their numeric names must be new and unique; priority is recorded through
   dependencies and the plan tracker, not by renumbering completed goals.
3. Finish and verify the severe fix before any non-blocking implementation or
   story testing resumes.
4. Restart user-story testing from `US-01`, because earlier results may no
   longer be trustworthy. Repeat this loop for each newly discovered severe
   blocker.

Do not declare the initiative complete while a required user story is
untested, in progress, bug found, or excluded without explicit user approval.
