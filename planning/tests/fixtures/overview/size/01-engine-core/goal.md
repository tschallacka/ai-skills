# Goal: Plan tree read and state parsing without limits

## Current state and prior-goal handoffs

§ 2.1
Confirmed: the shell renderer render-plan-overview.sh exits 126 on the 337 KB codegraph-bash-indexing-v2 state because it passes the state through jq --arg and hits the argument-length limit, and its serve mode does not work. Confirmed: overview-state.sh emits the state document this goal must parse, and it is the only input; the plan tree on disk is the other. No prior goal precedes this one, so there are no incoming handoffs.

## Outcome and definition of done

§ 3.1
A Rust binary reads a plan tree of any size and produces the full derived state in memory: identity, goals, steps, edges, testing marks, coverage, findings, cycles and review target, plus the counts, percentages and ring geometry the pages need. Demonstrated by producing correct derived values for the 337 KB fixture that the current renderer cannot render at all, with no argument-length limit anywhere in the path.

## Why this goal is needed

§ 4.1
Every other goal reads the derived state this goal produces. Without it there is no renderer at all: the existing one cannot render the largest real plan, so the pages the rest of the plan designs would have nothing to draw from on exactly the inputs that matter most.

## Scope

§ 5.1
In scope: reading the plan tree from files, parsing the state document into typed values, deriving counts, percentages and ring geometry, and surfacing unrecognised state fields rather than dropping them. Out of scope: emitting any markup, serving anything over a socket, and any presentation decision; those belong to goals 02, 03 and 04 onward.

## Affected files, systems, data, and interfaces

§ 6.1
New Rust crate at src/plan-overview, with src/plan/tree.rs, src/plan/state.rs and the derive modules inside the crate's own src tree. Every path this plan records for a crate file reads from the repository root, so src/plan-overview/tests/router.rs is not confused with the repository's own tests directory, which is where run-tests.sh looks for shell tests. The crate reads the plan directory and the state document that overview-state.sh emits. It writes nothing into the plan directory, which is user data.

## Dependencies and handoffs

§ 7.1
Depends on nothing. Hands to goal 02 the in-memory derived state as one owned structure, and hands to goal 09 the size fixture measurement that proves the argument-length limit is gone.

## Implementation approach, risks, and edge cases

§ 8.1
Approach: read every input as a file, never as a process argument, which is the defect being removed. Risk: a state field added later is silently ignored, so unrecognised fields are surfaced rather than dropped, and goal 07 tests that every emitted field reaches a page. Edge cases: a zero-total plan must not divide by zero when deriving percentages or ring geometry, and a truncated state must report where parsing stopped rather than yielding partial values.

## Owned work units

§ 9.1
`W01` — Read a plan directory into memory: plan description, goals, steps, testing companions, inventory, review, coverage and history, returning one owned structure. Every path is opened as a file; nothing is passed through a process argument.

§ 9.2
`W02` — Parse the canonical serialized state produced by W102 into typed values, reporting unknown fields and malformed input instead of dropping or partially presenting them.

§ 9.3
`W03` — Compute the counts and percentages the pages present: goals, steps, units, steps complete, findings total and open and resolved, resolved percentage, review depth against target, and per-goal completion.

§ 9.4
`W04` — Compute the donut offset and the three ring values from the derived counts, keeping the circumference constant in one place instead of repeated in markup.

§ 9.5
`W05` — Pin the production state contract: serialize W102 extraction output, parse it through W02, preserve every emitted field and value, and retain unknown-field and malformed-input failures.

§ 9.6
`W06` — Run the binary against the 337 KB codegraph-bash-indexing-v2 state and confirm it produces complete derived values with no argument-length error, where the current renderer exits 126. Record the measured state size and the wall time.

§ 9.7
`W77` — Create the crate manifest for the renderer with no runtime dependencies and a named test-per-field-buffer feature used only by W117's deterministic mutation test. The crate sits at src/plan-overview, one directory per binary under src, as CODE-STYLE section 1b requires.

§ 9.8
`W100` — The crate root: the module declarations that make plan, render, pages, serve and watch reachable, and nothing else. Without it every other source unit names a file the compiler never sees.

§ 9.9
`W101` — The command-line surface the binary exposes: --plan-dir, --out, --refresh, --watch, --serve and --port, with the same meanings the removed wrapper and its runtime servers gave them. This is the contract the skill contract and the documentation both name, so it is decided here rather than discovered at execution.

§ 9.10
`W102` — Extract the canonical typed state from the plan tree and serialize that exact state for the parser contract; never invoke the old shell extractor at runtime.

## Testing requirement

| Test required | Rationale |
|---|---|
| yes | Parsing and derivation produce the numbers every page presents; a silent change there is invisible on the page, so each unit is pinned by a fixture test and the size fixture is verified end to end. |

## Goal-size exception
