# Step: 01-step-render-shell

## Ownership

- Goal: `02-page-shell`
- Work unit: `W07`
- Type: `source`

## Change target

- File: `src/plan-overview/src/render/shell.rs`
- Primary symbol or file scope: `render_shell()`
- Subscope: `N/A`

## Objective

§ 4.1
Emit the document skeleton through a named production RenderBuffer whose capacity is fixed before rendering. W117 instruments this buffer boundary; the renderer has no runtime dependency on the old interpreter stack.

## Instructions

§ 5.1
Implement the named production RenderBuffer in src/plan-overview/src/render/shell.rs. Fix its capacity from state and template lengths before rendering, expose only its allocation and growth counters to the test feature, write values as slices of already-owned data, and keep the normal path free of per-substitution document copies. The test-per-field-buffer feature replaces this same RenderBuffer through its explicit test seam.

## Acceptance criteria

§ 6.1
One artifact contains every page and the embedded state, and the output is produced in a single pass with one output buffer. Rendering the 337 KB fixture allocates no per-key copy of the document, and US-16 opens the result from disk with the state present.

## Handoff

§ 7.1
Every page unit writes into this shell. W67 adds the transition markup; W51 selects which surface leads.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
