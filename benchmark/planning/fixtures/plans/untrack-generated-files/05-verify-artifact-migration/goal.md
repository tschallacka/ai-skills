# Goal: Prove the whole surface on clean and packaged trees

## Current state and prior-goal handoffs

§ 2.1
Goals 01-04 and 06-07 each stop at their class boundary; no goal yet proves the surfaces that span them (clean-checkout suite, npm package, release tarball, capsule) or the whole-plan ls-files assertion. Prior handoffs: the bootstrap (W08), prepack chain (W11), build seams (W12, W21), capsule builds (W13, W22), gates (W35, W36) and the rjq path (W37, W38).

## Outcome and definition of done

§ 3.1
Cross-cutting proof that no consumer was orphaned: a scratch clean checkout plus the documented bootstrap yields a green full suite under the resource wrapper; npm pack --dry-run contents assert all five libs and REVIEWER.md present, zero lib sources, no planning/bin artifacts; installer/build-release.sh tarball contents assert the same and a scratch install from it delivers a working skill set; a tag-checkout capsule smoke proves the git-archive path builds its artifacts. Demonstrable: the three verification units' commands run end to end with their assertions recorded in the run log.

## Why this goal is needed

§ 4.1
Packaging interactions only appear end to end: a class goal can be green while the pack or tarball silently drops or duplicates an artifact; one goal owns the cross-cutting proof so the others stay independently demonstrable.

## Scope

§ 5.1
In: the archive-checkout suite run with the ls-files assertion, the pack plus tarball plus scratch-install probe, and the capsule tag-path smoke. Out: running a benchmark worker, creating the CI release job (T70), and any code change - this goal is verification only.

## Affected files, systems, data, and interfaces

§ 6.1
No files change. Reads: run-tests.sh, npm pack output, installer/build-release.sh output, benchmark/planning/setup-benchmark.sh output, git ls-files.

## Dependencies and handoffs

§ 7.1
Depends on every prior goal being merged and committed to branch HEAD (W16, W31 and W33 archive or tag that commit - AR-6's precondition), and directly on W35/W36 for suite membership. Handoff: a green run here is the maintainer's green light for T70's release pipeline.

## Implementation approach, risks, and edge cases

§ 8.1
The suite runs under the resource wrapper per the constraints. Risk: a stale scratch tree hides regressions - each probe builds its tree fresh from git archive or a temp tag created at HEAD. Edge: if the suite grows a new generated-file consumer mid-flight, W31 is the tripwire that finds it.

## Owned work units

§ 9.1
`W31` — Fresh git-archive checkout into TMPDIR: run the documented bootstrap once, then ./run-tests.sh under the resource wrapper to green; confirm git ls-files shows none of the four generated classes.

§ 9.2
`W32` — npm pack --dry-run contents assert five libs and REVIEWER.md present, zero lib sources, no planning/bin path; installer/build-release.sh tarball asserts the same and a scratch install from it delivers a working skill set.

§ 9.3
`W33` — Run setup-benchmark.sh against a tag checkout so the git-archive path assembles the capsule; assert built libs and a generated REVIEWER.md inside it.

## Testing requirement

| Test required | Rationale |
|---|---|
| yes | All three units are verification work units (W31, W32, W33) - the goal is their proof. |

## Goal-size exception
