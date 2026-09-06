# Work-unit inventory: untrack-generated-files

## Definition-of-done coverage

| Required outcome or proof | Work unit IDs | Notes |
|---|---|---|

| git ls-files contains none of the four generated classes and .gitignore covers each | W01,W04,W07,W17,W25 | The four untrack units plus W04's tracked-binary regression guard perform and pin the claim; W31 and W06/W24 verify the resulting index state. |

| A clean checkout plus the documented bootstrap yields a green full suite | W08,W09,W10,W16,W31,W36 | W08 is the bootstrap, W09/W10/W36 the gate rewrites that must pass on unbuilt trees, W16 and W31 the clean-checkout proofs at class and whole level. |

| npm package and release tarball are complete after pack-time generation | W11,W12,W21,W32,W35 | W11/W12/W21 are the generation seams; W32 probes both surfaces end to end; W35 pins tarball content against fresh builds. |

| An install without bundled binaries completes with an actionable notice | W02,W03,W34,W06 | The manifest presence rule, the PATH notice, the skip-copy path and the install probe cover every consumer of the bundled binary. |

| Every freshness gate keeps teeth without committed files | W09,W18,W26,W27,W28 | The four gate rewrites plus the coupling flip re-express each gate as build-and-verify; each carries its own fault-injection criterion. |

| The benchmark capsule is complete on live-tree and git-archive paths | W13,W22,W33 | The two assembly units build libs and generate REVIEWER.md; W33 probes a tag checkout where the archive path cannot carry untracked files. |

| Documentation states the untracked reality everywhere it spoke of committed generated files | W05,W14,W15,W23,W29,W30 | One docs unit per affected file, each with a criterion that the old committed-file wording is gone. |

| Every consumer of a generated file either builds it, generates it, or refuses with an actionable message naming the fix | W03,W19,W20,W24,W34,W37,W38 | Installer notice paths (W03, W34), source-tree readers (W19, W20), the end-to-end probe (W24), and the rjq delivery plus register-test guard (W37, W38) cover each consumer class of the four generated artifacts. |

## Work units

| ID | Type | File | Primary symbol or file scope | Subscope | Intended change | Depends on | Goal | Step |
|---|---|---|---|---|---|---|---|---|

| W01 | config | `.gitignore` | `planning/bin section` | `N/A` | Correct the stale tracked-artifact comment and ignore the triple-shaped per-target artifact directories (planning/bin/*/), then git rm --cached planning/bin/x86_64-unknown-linux-musl/rjq so the blob leaves the index while binaries.tsv rows stay declared-but-unbuilt. | — | 01-untrack-rjq-binary | 01-step-ignore-bin-dir |

| W02 | test | `tests/test-skill-files-manifest.sh` | `platform-conditional binary presence rule` | `N/A` | A listed bin/<triple> row resolves as optional-on-disk: when the file exists it must be a regular executable, when absent the row is skipped without failure, because the install-time notice is the enforcement point and CI release builds are the delivery path. | W01 | 01-untrack-rjq-binary | 02-step-manifest-binary-presence |

| W03 | source | `installer/src/20-runtime-tools.sh` | `prepend_bundled_rjq()` | `N/A` | Prepend only when the triple dir and binary exist; when absent print a one-line notice naming the release asset pattern and the T70 dependency instead of prepending a missing path, keeping install.sh behaviour defined by installer/build.sh. | W01 | 01-untrack-rjq-binary | 03-step-bundled-rjq-notice |

| W04 | test | `tests/test-shipped-binaries.sh` | `tracked-binary regression guard` | `N/A` | Add the guard that fails when git ls-files names any file under planning/bin or chat/bin, so a re-committed binary blob fails the suite rather than passing quietly. | W01 | 01-untrack-rjq-binary | 04-step-tracked-binary-guard |

| W05 | docs | `planning/MAINTAINER.md` | `section 1 binaries.tsv row and section 2.8a binary-path wording` | `N/A` | State that per-target artifacts are CI-delivered and untracked, and that a local planning/bin/<triple> path exists only after a local build; align the section 1 artifact-map row with the untracked reality. | W01 | 01-untrack-rjq-binary | 05-step-maintainer-doc |

| W06 | verification | `N/A` | `untrack-rjq end-to-end probe` | `N/A` | On the working tree after W01-W05: git ls-files planning/bin is empty; a scratch install from a tree without the binary completes and prints the notice; npm pack --dry-run file list contains no planning/bin path. | W02,W03,W04,W05,W34 | 01-untrack-rjq-binary | 06-step-verify-untrack |

| W07 | config | `.gitignore` | `compiled plan libraries section` | `N/A` | Add a compiled-libraries section ignoring planning/scripts/plan-core-lib.sh, plan-crypt-lib.sh, plan-document-lib.sh, plan-progress-lib.sh and plan-table-lib.sh, with a comment naming build-plan-libs.sh as the producer. | — | 02-untrack-compiled-libs | 01-step-ignore-compiled-libs |

| W08 | source | `run-tests.sh` | `suite bootstrap` | `N/A` | Before suite discovery, build-if-missing: when any of the five compiled libs is absent run scripts/build-plan-libs.sh; when REVIEWER.md is absent run scripts/generate-reviewer.sh (no-op while it is still tracked; kicks in after goal 03); when rjq is neither on PATH nor built, run bootstrap.sh (W37). Staleness detection stays with the tests, so the bootstrap never masks drift. | W07 | 02-untrack-compiled-libs | 02-step-run-tests-ensure-built |

| W09 | test | `planning/tests/test-plan-libs-build.sh` | `freshness assertions` | `N/A` | Rewrite from compare-against-committed to build-and-verify: byte-compare two fresh prod builds for determinism, keep the 500-line cap, symbol-set, function-file sourceability and dev/prod target assertions, and keep the mid-suite rewrite writing the now-gitignored paths; a source file that disagrees with a fresh build must still fail. | W07 | 02-untrack-compiled-libs | 03-step-libs-test-build-and-verify |

| W10 | test | `tests/test-skill-files-manifest.sh` | `compiled-library presence rule` | `N/A` | The five lib rows keep requiring presence on disk, but the test builds them first when missing, so prepack on a clean checkout passes without a committed copy. | W07 | 02-untrack-compiled-libs | 04-step-manifest-libs-build-first |

| W11 | config | `package.json` | `scripts.prepack` | `N/A` | Prepend the generator bootstrap to prepack: run planning/scripts/build-plan-libs.sh and planning/scripts/generate-reviewer.sh before the existing manifest and register-schema tests, so pack-time artifact generation is one documented seam. | W07 | 02-untrack-compiled-libs | 05-step-prepack-bootstrap |

| W12 | source | `installer/build-release.sh` | `collect() precondition` | `missing-library build step` | Build-if-missing the five compiled libs before the listed-file hard error, so a release build from a clean tree self-heals instead of failing on a generated row. | W07 | 02-untrack-compiled-libs | 06-step-release-build-libs |

| W13 | source | `benchmark/planning/setup-benchmark.sh` | `capsule assembly` | `compiled libraries` | Build the five libs into the capsule after copying planning/scripts/, on both the live-tree and git-archive paths, so tag-based runs are not silently missing generated files. | W07 | 02-untrack-compiled-libs | 07-step-capsule-build-libs |

| W14 | docs | `AGENTS.md` | `Running tests section` | `N/A` | Document the bootstrap: a clean checkout gets built artifacts via the run-tests bootstrap or prepack, and the suite is safe to run on a tree that has never been built. | W08 | 02-untrack-compiled-libs | 08-step-agents-testing-doc |

| W15 | docs | `DEVELOPMENT.md` | `release flow steps` | `N/A` | State that npm pack assembles generated artifacts through prepack, and that no generated file is committed ahead of a release. | W11 | 02-untrack-compiled-libs | 09-step-development-release-doc |

| W16 | verification | `N/A` | `clean-checkout suite proof` | `N/A` | In a scratch clean checkout (git archive of HEAD into TMPDIR): run the documented bootstrap, then ./run-tests.sh under the resource wrapper; the suite passes with the five libs rebuilt from nothing. | W08,W09,W10 | 02-untrack-compiled-libs | 10-step-verify-clean-suite |

| W17 | config | `.gitignore` | `REVIEWER.md entry` | `N/A` | Add planning/REVIEWER.md to .gitignore with a comment naming generate-reviewer.sh and the SKILL.md SHA-256 pin as the producer and its contract. | — | 03-untrack-reviewer-md | 01-step-ignore-reviewer |

| W18 | test | `planning/tests/test-reviewer-projection.sh` | `freshness assertions` | `N/A` | Rewrite from compare-against-committed to build-and-verify: run generate-reviewer.sh to a temp output, assert the pinned SKILL.md SHA-256 and required markers against it, and byte-compare two fresh runs for determinism; a SKILL.md edit that changes the projection must still fail the test. | W17 | 03-untrack-reviewer-md | 02-step-reviewer-test-build-to-temp |

| W19 | source | `planning/scripts/role-context.sh` | `REVIEWER.md read path` | `N/A` | When the skill tree has no REVIEWER.md, refuse with an actionable message naming generate-reviewer.sh and the skill root, instead of reading a missing file. | W17 | 03-untrack-reviewer-md | 03-step-role-context-refusal |

| W20 | source | `planning/scripts/plan-context-lib.sh` | `context source listing` | `N/A` | The listing already omits an absent REVIEWER.md behind an existing -f guard; the change is narrower: print a stated reason line (absent, generate with generate-reviewer.sh) with the omission instead of silence, and keep the present case unchanged. | W17 | 03-untrack-reviewer-md | 04-step-context-listing-guard |

| W21 | source | `installer/build-release.sh` | `collect() precondition` | `reviewer generation` | Run planning/scripts/generate-reviewer.sh before collect when REVIEWER.md is missing from the tree being packaged, mirroring the library build-if-missing step. | W17,W12 | 03-untrack-reviewer-md | 05-step-release-generate-reviewer |

| W22 | source | `benchmark/planning/setup-benchmark.sh` | `capsule assembly` | `reviewer document` | Generate REVIEWER.md into the capsule when absent instead of copying only if present, so every capsule - git-archive tags included - carries it. | W17,W13 | 03-untrack-reviewer-md | 06-step-capsule-generate-reviewer |

| W23 | docs | `planning/MAINTAINER.md` | `section 1 REVIEWER.md row` | `N/A` | Align the artifact-map REVIEWER.md row with the untracked reality: generated on demand by generate-reviewer.sh, pinned to SKILL.md's hash, never committed. | W17 | 03-untrack-reviewer-md | 07-step-maintainer-reviewer-doc |

| W24 | verification | `N/A` | `untrack-reviewer end-to-end probe` | `N/A` | Remove REVIEWER.md from a scratch checkout: role-context.sh fails with the named fix; test-reviewer-projection.sh passes against fresh builds; a build-release.sh tarball contains a generated REVIEWER.md; the capsule contains one on both assembly paths. | W18,W19,W21,W22 | 03-untrack-reviewer-md | 08-step-verify-reviewer |

| W25 | config | `.gitignore` | `PORTABILITY.md entry` | `N/A` | Add PORTABILITY.md to .gitignore with a comment naming generate-portability.sh and portability-rules.json as the producer. | — | 04-untrack-portability-md | 01-step-ignore-portability |

| W26 | test | `planning/tests/test-portability-contract.sh` | `freshness arm` | `N/A` | Rewrite the freshness arm: regenerate to PORTABILITY_OUTPUT temp paths and byte-compare two fresh runs, keeping the marker-id hygiene and banned-construct sweeps running against the regenerated text and the UNCONFIGURED-without-rjq behaviour unchanged. | W25 | 04-untrack-portability-md | 02-step-portability-test-temp |

| W27 | source | `generate-portability.sh` | `--check mode` | `N/A` | --check compares two fresh temp regenerations for determinism instead of diffing against a committed file, so the mode keeps working with nothing tracked. | W25 | 04-untrack-portability-md | 03-step-generator-check-determinism |

| W28 | config | `coupling.tsv` | `rows 6-7 portability check commands` | `N/A` | Flip the two PORTABILITY.md rows from running ./generate-portability.sh --check against the tree to running the portability contract test, whose temp regeneration carries the same drift detection. | W26 | 04-untrack-portability-md | 04-step-coupling-rows-flip |

| W29 | docs | `AGENTS.md` | `PORTABILITY.md pointer` | `N/A` | Reword the loading-skills pointer from read-PORTABILITY.md to generate-then-read, and drop the stale hand-edit warning now that there is nothing committed to hand-edit. | W25 | 04-untrack-portability-md | 05-step-agents-portability-doc |

| W30 | docs | `CODE-STYLE.md` | `PORTABILITY.md references` | `N/A` | Update references so the catalogue is described as generated on demand from portability-rules.json, keeping it the contract for the scripts. | W25 | 04-untrack-portability-md | 06-step-codestyle-portability-doc |

| W31 | verification | `N/A` | `clean-checkout bootstrap and full suite` | `N/A` | Fresh git-archive checkout into TMPDIR: run the documented bootstrap once, then ./run-tests.sh under the resource wrapper to green; confirm git ls-files shows none of the four generated classes. | W06,W16,W24,W26,W27,W28,W29,W30,W35,W36 | 05-verify-artifact-migration | 01-step-verify-bootstrap-suite |

| W32 | verification | `N/A` | `package and release-surface probe` | `N/A` | npm pack --dry-run contents assert five libs and REVIEWER.md present, zero lib sources, no planning/bin path; installer/build-release.sh tarball asserts the same and a scratch install from it delivers a working skill set. | W11,W12,W21,W31 | 05-verify-artifact-migration | 02-step-verify-package-release |

| W33 | verification | `N/A` | `benchmark capsule tag-path smoke` | `N/A` | Run setup-benchmark.sh against a tag checkout so the git-archive path assembles the capsule; assert built libs and a generated REVIEWER.md inside it. | W13,W22,W31 | 05-verify-artifact-migration | 03-step-verify-capsule |

| W34 | source | `installer/src/60-install.sh` | `install_skill() bundled-binary copy` | `N/A` | When a platform-conditional bundled-binary row has no file on disk, skip the copy and print the same one-line notice as prepend_bundled_rjq (release asset pattern, T70 dependency) instead of failing the install on cp of a missing file; regenerate install.sh via installer/build.sh as the step's generated output. | W01,W03 | 01-untrack-rjq-binary | 07-step-install-skip-missing-binary |

| W35 | test | `tests/test-release-package.sh` | `byte-identity assertions` | `N/A` | Compare the tarball's plan-core-lib.sh (and siblings) against fresh build-plan-libs.sh output instead of repo copies, keeping the exactly-once, zero-lib-sources, zero-compiler and installability assertions. | W07,W12 | 06-package-test-gates | 01-step-release-package-fresh-builds |

| W36 | test | `tests/test-mode-markers.sh` | `generated-file scan list` | `N/A` | Build-if-missing the compiled libs and generate-if-missing REVIEWER.md before the marker scans, so the marker contract holds on a tree that has never been built; scans keep failing on a present-but-wrong file. | W07,W17 | 06-package-test-gates | 02-step-mode-markers-build-first |

| W37 | source | `bootstrap.sh` | `rjq build-if-missing arm` | `N/A` | New repo-root bootstrap.sh: when rjq is neither on PATH nor at planning/bin/<triple>/rjq, build src/rjq (cargo build --release) into the gitignored planning/bin/<triple>/ path the installer's prepend logic already knows, print the PATH line to export, and exit non-zero if cargo is missing with a message naming the T70 release download as the no-toolchain alternative. | W01 | 07-rjq-local-delivery | 01-step-bootstrap-rjq |

| W38 | test | `tests/test-register-schemas.sh` | `rjq availability guard` | `N/A` | Add a command -v rjq guard at the top that fails with the bootstrap.sh instruction and the T70 release note instead of a bare command-not-found; confirm tests/test-rjq-active-references.sh's existing unavailability message names the same fix. | W37 | 07-rjq-local-delivery | 02-step-register-schemas-guard |

## Decomposition review

- [x] Every definition-of-done item maps to one or more work units.
- [x] Every known affected file and changing symbol has its own work unit.
- [x] Every work unit has exactly one goal and one step.
- [x] Each goal has 2–10 work units, or records an allowed exception.
- [x] Each step has exactly one work unit and no unnamed incidental edits.
- [x] Dependencies form an executable order with no cycle.
