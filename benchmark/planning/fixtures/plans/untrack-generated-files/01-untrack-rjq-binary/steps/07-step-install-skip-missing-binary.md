# Step: 07-step-install-skip-missing-binary

## Ownership

- Goal: `01-untrack-rjq-binary`
- Work unit: `W34`
- Type: `source`

## Change target

- File: `installer/src/60-install.sh`
- Primary symbol or file scope: `install_skill() bundled-binary copy`
- Subscope: `N/A`

## Objective

§ 4.1
When a platform-conditional bundled-binary row has no file on disk, skip the copy and print the same one-line notice as prepend_bundled_rjq (release asset pattern, T70 dependency) instead of failing the install on cp of a missing file; regenerate install.sh via installer/build.sh as the step's generated output.

## Instructions

§ 5.1
In installer/src/60-install.sh, when a platform-conditional bundled-binary row has no file on disk, skip the copy and print the same one-line notice prepend_bundled_rjq prints (release asset pattern, T70 dependency) instead of failing install_skill() on cp of a missing file; rows with the file present copy exactly as before. Run installer/build.sh to regenerate install.sh and record it as this step's generated output.

## Acceptance criteria

§ 6.1
A scratch install without the binary completes, prints the notice once, and installs everything else; with the binary present the install is byte-behaviour-identical to before; installer/build.sh --check passes.

## Handoff

§ 7.1
Goal 05's W32 scratch-install assertion relies on this skip-with-notice behaviour existing.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
