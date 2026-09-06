# Step: 03-step-bundled-rjq-notice

## Ownership

- Goal: `01-untrack-rjq-binary`
- Work unit: `W03`
- Type: `source`

## Change target

- File: `installer/src/20-runtime-tools.sh`
- Primary symbol or file scope: `prepend_bundled_rjq()`
- Subscope: `N/A`

## Objective

§ 4.1
Prepend only when the triple dir and binary exist; when absent print a one-line notice naming the release asset pattern and the T70 dependency instead of prepending a missing path, keeping install.sh behaviour defined by installer/build.sh.

## Instructions

§ 5.1
In installer/src/20-runtime-tools.sh, guard prepend_bundled_rjq(): prepend the triple dir only when both the directory and the rjq binary exist for the matched host; when they do not, print one notice line to stderr naming the release asset pattern (the GitHub releases page for this repo) and that download support is queued as T70, then return 0. Never prepend a nonexistent path. Behaviour with the binary present stays byte-identical. installer/src changes are producers of generated install.sh: run installer/build.sh afterwards and record it as this step's generated output (install.sh is the one tracked generated file, deliberately excluded from untracking).

## Acceptance criteria

§ 6.1
With the binary present, PATH is prepended exactly as before (captured before/after); without it, exit 0, the notice appears on stderr, and PATH gains no nonexistent entry; installer/build.sh --check passes; the changed script passes shellcheck -s bash and bash 3.2.

## Handoff

§ 7.1
W34 reuses this notice wording verbatim; goal 02's docs cite it as the bundled-binary contract.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
