# Goal: Reconcile packaging-adjacent test gates with build-time artifacts

## Current state and prior-goal handoffs

§ 2.1
tests/test-release-package.sh property 2 (line 113) byte-compares every packed file against the repository copy - an assertion that breaks when the libs and REVIEWER.md are untracked. tests/test-mode-markers.sh scans generated files that will not exist on a clean tree. Master b0e31f7 already exempted fixtures and absent plan-overview artifacts in both files.

## Outcome and definition of done

§ 3.1
tests/test-release-package.sh asserts tarball content against fresh builds instead of repo copies (which no longer exist); tests/test-mode-markers.sh builds-or-generates-if-missing before its marker scans on a clean tree. Demonstrable: both gates pass on a clean checkout with no generated files present beforehand, and both still fail when a generator's output would be wrong.

## Why this goal is needed

§ 4.1
These are the packaging-adjacent gates the suite runs today; left pointing at repo copies they would fail by construction after untracking, and left weakened they would stop catching a stale packaged artifact.

## Scope

§ 5.1
In: the byte-identity arm's fresh-build comparison and the marker scans' build-if-missing precondition. Out: the tarball's expected-set derivation (master's fixture and absent-artifact handling stays), the pack contents design (goal 02's prepack owns it), and the release pipeline (T70).

## Affected files, systems, data, and interfaces

§ 6.1
tests/test-release-package.sh; tests/test-mode-markers.sh.

## Dependencies and handoffs

§ 7.1
Depends on goal 02 (libs untracked, build-if-missing target) and goal 03 (REVIEWER.md generate-if-missing target). Handoff: W31 and W32 depend on both units - the full-suite proof runs only after these gates point at build-time reality.

## Implementation approach, risks, and edge cases

§ 8.1
Fault-injection proves the rewritten arms still bite: a stale lib in the tarball must fail byte-identity, a wrong MODE marker must fail the scan. Edge: the build/generate steps run at most once per invocation, and absent non-generated listed files keep hard-erroring.

## Owned work units

§ 9.1
`W35` — Compare the tarball's plan-core-lib.sh (and siblings) against fresh build-plan-libs.sh output instead of repo copies, keeping the exactly-once, zero-lib-sources, zero-compiler and installability assertions.

§ 9.2
`W36` — Build-if-missing the compiled libs and generate-if-missing REVIEWER.md before the marker scans, so the marker contract holds on a tree that has never been built; scans keep failing on a present-but-wrong file.

## Testing requirement

| Test required | Rationale |
|---|---|
| yes | Both units are test units (W35, W36) with fault-injection criteria. |

## Goal-size exception
