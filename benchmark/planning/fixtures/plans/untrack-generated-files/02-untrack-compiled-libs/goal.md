# Goal: Untrack the five compiled plan libraries; build on demand everywhere

## Current state and prior-goal handoffs

§ 2.1
The five compiled libs are tracked; 42 scripts source plan-document-lib.sh and the facade chains the rest. planning/tests/test-plan-libs-build.sh byte-compares committed libs against a fresh build and rewrites them mid-suite. prepack runs only the manifest and register-schema tests (master b0e31f7 added >/dev/null and .ci-bin). Prior goal handoff: goal 01's pattern and its gitignored planning/bin path, which the W37 bootstrap fills.

## Outcome and definition of done

§ 3.1
planning/scripts/plan-{core,crypt,document,progress,table}-lib.sh are gitignored and untracked; run-tests.sh carries an ensure-built step so a clean checkout self-heals; planning/tests/test-plan-libs-build.sh verifies build determinism and its 500-line cap, symbol-set and dev/prod assertions against fresh builds instead of committed files, with unchanged teeth; tests/test-skill-files-manifest.sh resolves the five lib rows after the ensure-built step; package.json prepack runs build-plan-libs.sh before the manifest and register-schema tests; installer/build-release.sh builds missing libs before collect; benchmark/planning/setup-benchmark.sh builds libs into the capsule for both the live-tree and git-archive paths; AGENTS.md documents the bootstrap and DEVELOPMENT.md the publish-time build. Demonstrable: rm the five libs, run ./run-tests.sh, and the suite passes with the libs rebuilt; npm pack --dry-run contains all five; a tag-checkout capsule contains built libs.

## Why this goal is needed

§ 4.1
The compiled libs are the widest-blast-radius class: every helper, the suite itself and the packaged skill read them, so this goal establishes the build-on-demand model (run-tests bootstrap, prepack generators, build-release build-if-missing, capsule build) that makes untracking safe.

## Scope

§ 5.1
In: the .gitignore libs section, the run-tests bootstrap (libs, REVIEWER.md generate-if-missing, rjq via W37), the lib test's build-and-verify rewrite, the manifest test building first, the prepack generator chain, the build-release libs arm, the capsule libs build, AGENTS.md and DEVELOPMENT.md wording, and the clean-checkout suite probe. Out: the five lib rows in skill_files() (they stay listed and resolve post-build), REVIEWER.md untracking (goal 03), and any change to lib/ sources.

## Affected files, systems, data, and interfaces

§ 6.1
.gitignore; run-tests.sh; planning/tests/test-plan-libs-build.sh; tests/test-skill-files-manifest.sh; package.json; installer/build-release.sh; benchmark/planning/setup-benchmark.sh; AGENTS.md; DEVELOPMENT.md. Invoked, not edited: bootstrap.sh (W37).

## Dependencies and handoffs

§ 7.1
Depends on goal 01's pattern and W37's bootstrap.sh for the rjq arm. Handoffs: W16 and W31 inherit the bootstrap; the libs-before-reviewer prepack order is the constraint W21 repeats in build-release.sh; W15's handoff records the RELEASE.md sweep outcome.

## Implementation approach, risks, and edge cases

§ 8.1
The lib test rewrites the real libs mid-suite - safe for git once ignored, and its determinism check keeps teeth. Register tests need rjq on PATH, covered by W37/W38. The npm files list names planning/bin explicitly, so the W32 probe asserts pack contents rather than assuming how gitignore interacts with the whitelist.

## Owned work units

§ 9.1
`W07` — Add a compiled-libraries section ignoring planning/scripts/plan-core-lib.sh, plan-crypt-lib.sh, plan-document-lib.sh, plan-progress-lib.sh and plan-table-lib.sh, with a comment naming build-plan-libs.sh as the producer.

§ 9.2
`W08` — Before suite discovery, build-if-missing: when any of the five compiled libs is absent run scripts/build-plan-libs.sh; when REVIEWER.md is absent run scripts/generate-reviewer.sh (no-op while it is still tracked; kicks in after goal 03); when rjq is neither on PATH nor built, run bootstrap.sh (W37). Staleness detection stays with the tests, so the bootstrap never masks drift.

§ 9.3
`W09` — Rewrite from compare-against-committed to build-and-verify: byte-compare two fresh prod builds for determinism, keep the 500-line cap, symbol-set, function-file sourceability and dev/prod target assertions, and keep the mid-suite rewrite writing the now-gitignored paths; a source file that disagrees with a fresh build must still fail.

§ 9.4
`W10` — The five lib rows keep requiring presence on disk, but the test builds them first when missing, so prepack on a clean checkout passes without a committed copy.

§ 9.5
`W11` — Prepend the generator bootstrap to prepack: run planning/scripts/build-plan-libs.sh and planning/scripts/generate-reviewer.sh before the existing manifest and register-schema tests, so pack-time artifact generation is one documented seam.

§ 9.6
`W12` — Build-if-missing the five compiled libs before the listed-file hard error, so a release build from a clean tree self-heals instead of failing on a generated row.

§ 9.7
`W13` — Build the five libs into the capsule after copying planning/scripts/, on both the live-tree and git-archive paths, so tag-based runs are not silently missing generated files.

§ 9.8
`W14` — Document the bootstrap: a clean checkout gets built artifacts via the run-tests bootstrap or prepack, and the suite is safe to run on a tree that has never been built.

§ 9.9
`W15` — State that npm pack assembles generated artifacts through prepack, and that no generated file is committed ahead of a release.

§ 9.10
`W16` — In a scratch clean checkout (git archive of HEAD into TMPDIR): run the documented bootstrap, then ./run-tests.sh under the resource wrapper; the suite passes with the five libs rebuilt from nothing.

## Testing requirement

| Test required | Rationale |
|---|---|
| yes | W09 and W10 are test units (lib build-and-verify rewrite; manifest build-first) and W16 is the clean-checkout suite verification. |

## Goal-size exception
