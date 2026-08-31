# Step: 07-step-memory-harness

## Ownership

- Goal: `09-verification`
- Work unit: `W117`
- Type: `test`

## Change target

- File: `src/plan-overview/tests/memory.rs`
- Primary symbol or file scope: `memory_harness()`
- Subscope: `N/A`

## Objective

§ 4.1
Register memory.rs against RenderBuffer::new and RenderBuffer::write_str in render/shell.rs. Record separate allocation and growth counters for that production buffer. Run cargo test --test memory and cargo test --features test-per-field-buffer --test memory on W91 and W92; the first passes with one allocation and zero growth and the mutation exits non-zero from per-field String allocation.

## Instructions

§ 5.1
Create src/plan-overview/tests/memory.rs against RenderBuffer::new and RenderBuffer::write_str in render/shell.rs. Reset the seam's allocation and growth counters immediately before each W91 and W92 render. Run cargo test --test memory for the normal path and cargo test --features test-per-field-buffer --test memory for the mutation path. The normal command must report one RenderBuffer allocation and zero growth for each fixture. The feature command must return non-zero because the feature replaces RenderBuffer::write_str output with a fresh String allocation per emitted field. Record rustc 1.86.0 and both fixture checksums.

## Acceptance criteria

§ 6.1
The registered cargo test target measures RenderBuffer::new and RenderBuffer::write_str in the production render path on both fixtures and records rustc 1.86.0 and fixture checksums. It passes only when that buffer allocates once with zero growth for each fixture. Running cargo test --features test-per-field-buffer --test memory fails because the mutation produces more than one output-buffer allocation. A test that measures an unconnected helper does not satisfy this criterion.

## Handoff

§ 7.1
W90 consumes this registered harness for the recorded two-fixture memory proof; W48 consumes its crate-test result through the repository gate.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
