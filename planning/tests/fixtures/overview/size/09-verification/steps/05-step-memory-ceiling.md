# Step: 05-step-memory-ceiling

## Ownership

- Goal: `09-verification`
- Work unit: `W90`
- Type: `verification`

## Change target

- File: `N/A`
- Primary symbol or file scope: memory ceiling across both fixtures
- Subscope: `N/A`

## Objective

§ 4.1
Consume W117 on W91 and W92 with exact rustc 1.86.0 and fixture checksums. Record peak resident memory total bytes total allocation count RenderBuffer allocation count and growth count. Require one RenderBuffer allocation and zero growth and require the per-field mutation to fail the crate test.

## Instructions

§ 5.1
Invoke W117 with the exact normal command cargo test --test memory and the exact mutation command cargo test --features test-per-field-buffer --test memory on W91 and W92. Record peak resident memory, total bytes allocated, total allocation count, RenderBuffer allocation count and RenderBuffer growth count for each, with rustc 1.86.0 and fixture checksums. Make only the direct invariant normative: one RenderBuffer allocation and zero growth because W07 sizes it before rendering. The mutation must fail the crate test because its per-field String allocations exceed one output-buffer allocation.

## Acceptance criteria

§ 6.1
Both fixtures record all four measurements, and the registered harness passes with exactly one output-buffer allocation and zero growth for each. The per-field-concatenation mutation fails the crate test specifically because its output-buffer allocation count exceeds one. Peak memory and total bytes remain recorded diagnostics, not an unstated platform-dependent ceiling.

## Handoff

§ 7.1
The memory requirement that motivated the rewrite has a gate rather than an assertion, so a renderer that allocates a fresh string per field fails a named check instead of passing every declared one. Goal 09 can state that the plan's stated reason for existing is verified and not merely claimed.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
