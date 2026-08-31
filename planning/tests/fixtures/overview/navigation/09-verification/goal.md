# Goal: Both fixtures, every page, and the portability gates

## Current state and prior-goal handoffs

§ 2.1
Depends on every page goal being complete, because it exercises them. Confirmed: the repository requires that a verification name the mutation that fails without it, and that only one two-shell harness run happen at a time because a second deletes the first worktree.

## Outcome and definition of done

§ 3.1
Every page has passed a browser story with real interaction on the 82-unit fixture, the 337 KB fixture renders and serves, and the repository gates that cover the changed shell and generated artifacts pass on both shells. Demonstrated by the story table showing passes with recorded interaction evidence, and by a green two-shell run.

## Why this goal is needed

§ 4.1
The plan claims a browsable artifact and a size limit removed. Neither claim is worth anything unrecorded, and the stories are the only place the interaction evidence lands.

## Scope

§ 5.1
In scope: executing every story with real interaction and recording the evidence, the size fixture measurement, the registered memory harness and its fault injection, the repository gates on both shells, and confirming the platform matrix green. Out of scope: writing the pages or production tests themselves, which earlier goals own; W117 is the goal's explicitly owned verification harness.

## Affected files, systems, data, and interfaces

§ 6.1
The story table and its per-story run caches, the size fixture record, the registered memory harness in src/plan-overview/tests/memory.rs, and the recorded results of the repository suite and the two-shell harness. No other source file changes here.

## Dependencies and handoffs

§ 7.1
Depends on goals 01 through 08 and 10 through 12 for the product surfaces and on goals 13 through 17 for the removal declarations test rewrites fixtures packaging and Rust toolchain proof consumed by W48 and W49. The unit graph exposes the parallel branches and joins them at W48 after W15 W16 W78 W80 W81 W82 W83 W84 W85 W86 W87 W88 W89 W99 W103 W104 W105 W106 W107 W108 W109 W110 W111 W112 W113 W114 W115 W116 W117 W118 W119 and W120. Hands nothing onward; it is the terminal proof for the plan.

## Implementation approach, risks, and edge cases

§ 8.1
Approach: a story passes only on recorded interaction, never on a screenshot or a state read alone, and a story with no recorded observation counts as failed rather than skipped. Risk: two concurrent harness runs destroy each other, so the gates are run one at a time. Edge case: a story may be excluded only with a recorded user approval, not by an executing agent.

## Owned work units

§ 9.1
`W46` — Every UI story in the story table has passed with real interaction on the 82-unit fixture, each with its recorded control and action. No story passes on a screenshot or a DOM read alone.

§ 9.2
`W47` — The 337 KB fixture renders to an artifact and serves without error, and its pages are navigable. This is the case that exits 126 today.

§ 9.3
`W48` — Join the terminal proof after the removal fixture registration toolchain package and runtime-isolation branches complete. Run both shells with the crate suite and all gates and reject an early or unconfigured run.

§ 9.4
`W49` — The five-artifact CI matrix passes, each job having executed its own artifact once, including the windows msvc leg through PowerShell.

§ 9.5
`W90` — Consume W117 on W91 and W92 with exact rustc 1.86.0 and fixture checksums. Record peak resident memory total bytes total allocation count RenderBuffer allocation count and growth count. Require one RenderBuffer allocation and zero growth and require the per-field mutation to fail the crate test.

§ 9.6
`W108` — Confirm every file under src/plan-overview carries the marker its kind requires, on the first two lines: MODE: DEV and PACKAGE: PROD on the manifest and the sources, MODE: DEV alone on the toolchain file, and an exemption reason for the generated lock and the built artifacts. Each source unit writes its own markers; this unit is where the whole crate is checked once, after the sources exist.

§ 9.7
`W117` — Register memory.rs against RenderBuffer::new and RenderBuffer::write_str in render/shell.rs. Record separate allocation and growth counters for that production buffer. Run cargo test --test memory and cargo test --features test-per-field-buffer --test memory on W91 and W92; the first passes with one allocation and zero growth and the mutation exits non-zero from per-field String allocation.

§ 9.8
`W118` — Register runtime_isolation.rs to install the artifact in a temporary root and run bounded render and serve lifecycles under strace on Linux sandbox-exec on macOS and a Windows Job Object. Require readiness termination and zero prohibited child or overview-script executions.

## Testing requirement

| Test required | Rationale |
|---|---|
| yes | This goal is verification; its units are the story table, the size fixture, the repository gates and the artifact matrix. |

## Goal-size exception
