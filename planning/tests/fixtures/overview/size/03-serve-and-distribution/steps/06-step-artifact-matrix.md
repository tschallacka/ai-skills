# Step: 06-step-artifact-matrix

## Ownership

- Goal: `03-serve-and-distribution`
- Work unit: `W19`
- Type: `config`

## Change target

- File: `.github/workflows/render-artifacts.yml`
- Primary symbol or file scope: `artifact matrix`
- Subscope: `N/A`

## Objective

§ 4.1
Produce the five declared artifacts, each proven to execute on its own platform.

## Instructions

§ 5.1
Add one job per triple: x86_64 and aarch64 linux musl, x86_64 and aarch64 apple darwin, and x86_64 pc windows msvc. Each job builds the binary on a runner of that platform and then runs it once there, so an artifact that cannot execute fails its own job. The windows leg invokes it through PowerShell. The darwin legs must not be cross-compiled from a Linux runner: the linker ad-hoc signs only on macOS, and Apple Silicon kills an unsigned arm64 binary, so a cross-compiled artifact would pass this job and die on a user machine.

## Acceptance criteria

§ 6.1
All five jobs pass and each has executed its own artifact at least once on its own platform. A job that builds without running is not acceptable evidence, and neither is a darwin job that built on a Linux runner. Feasibility was measured in a separate repository with this exact matrix shape.

## Handoff

§ 7.1
W49 confirms the matrix is green as release evidence; W17 ships what these jobs produce.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
