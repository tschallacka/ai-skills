# Goal: Untrack the committed rjq binary and reconcile its consumers

## Current state and prior-goal handoffs

§ 2.1
The rjq blob sits tracked at planning/bin/x86_64-unknown-linux-musl/rjq (e8bdaef, register T70a). Master b0e31f7 already skips absent planning/bin/* rows in tests/test-skill-files-manifest.sh and lists plan-overview artifact rows with unshipped reasons. installer/src/20-runtime-tools.sh prepend_bundled_rjq() prepends the triple dir unconditionally today; installer/src/60-install.sh copies listed rows and fails on a missing file. No prior goal.

## Outcome and definition of done

§ 3.1
git ls-files planning/bin is empty and the blob from e8bdaef is gone from the index; .gitignore carries the triple-shaped planning/bin section with the stale tracked-artifact comment corrected; tests/test-skill-files-manifest.sh treats a platform-conditional bundled binary as optional-on-disk, because installer/tools.tsv exists precisely because rjq may be missing on a host; installer/src/20-runtime-tools.sh prepend_bundled_rjq prepends only an existing triple dir and prints a one-line notice naming the release asset and the T70 dependency when absent, so the install completes; tests/test-shipped-binaries.sh gains the regression guard that fails when any binary under planning/bin is tracked; planning/MAINTAINER.md section 1 and section 2.8a wording matches the untracked reality. Demonstrable: on the working tree, git ls-files planning/bin is empty, the suite subset test-shipped-binaries + test-skill-files-manifest passes, and a scratch install without the binary completes and states the notice.

## Why this goal is needed

§ 4.1
It is the smallest consumer set of the four classes, so it proves the untrack-and-reconcile pattern (gitignore section, optional-on-disk manifest rule, consumer notice, regression guard, docs) before the goals with forty-plus consumers reuse it; it also closes register task T70a.

## Scope

§ 5.1
In: the .gitignore planning/bin section, the manifest-test verification and invalid-binary guard, the runtime-tools notice, the 60-install skip-copy path, the tracked-binary regression guard, MAINTAINER.md wording, and the end-to-end probe. Out: release download support (T70 owns it), the chat, plan-overview and plan-crypt binaries and registries, and any installer source beyond the two named files.

## Affected files, systems, data, and interfaces

§ 6.1
.gitignore; tests/test-skill-files-manifest.sh; installer/src/20-runtime-tools.sh; installer/src/60-install.sh (with install.sh regenerated through installer/build.sh); tests/test-shipped-binaries.sh; planning/MAINTAINER.md. Probe-only: npm pack listing and a scratch install.

## Dependencies and handoffs

§ 7.1
No prerequisites. Handoffs: goal 02 reuses this goal's untrack-and-reconcile pattern; W06 and W34 must print the identical notice wording W03 defines; W04's guard is what W31's ls-files assertion re-checks at whole-plan level.

## Implementation approach, risks, and edge cases

§ 8.1
Risk: curl-pipe-bash installs lose the bundled binary; the notice names the release asset the existing rjq-binary CI job already uploads, and T70 turns the notice into a download. Edge: hosts matching no installer condition keep today's silent-skip behaviour; the regression guard must tolerate an empty but present bin directory.

## Owned work units

§ 9.1
`W01` — Correct the stale tracked-artifact comment and ignore the triple-shaped per-target artifact directories (planning/bin/*/), then git rm --cached planning/bin/x86_64-unknown-linux-musl/rjq so the blob leaves the index while binaries.tsv rows stay declared-but-unbuilt.

§ 9.2
`W02` — A listed bin/<triple> row resolves as optional-on-disk: when the file exists it must be a regular executable, when absent the row is skipped without failure, because the install-time notice is the enforcement point and CI release builds are the delivery path.

§ 9.3
`W03` — Prepend only when the triple dir and binary exist; when absent print a one-line notice naming the release asset pattern and the T70 dependency instead of prepending a missing path, keeping install.sh behaviour defined by installer/build.sh.

§ 9.4
`W04` — Add the guard that fails when git ls-files names any file under planning/bin or chat/bin, so a re-committed binary blob fails the suite rather than passing quietly.

§ 9.5
`W05` — State that per-target artifacts are CI-delivered and untracked, and that a local planning/bin/<triple> path exists only after a local build; align the section 1 artifact-map row with the untracked reality.

§ 9.6
`W06` — On the working tree after W01-W05: git ls-files planning/bin is empty; a scratch install from a tree without the binary completes and prints the notice; npm pack --dry-run file list contains no planning/bin path.

§ 9.7
`W34` — When a platform-conditional bundled-binary row has no file on disk, skip the copy and print the same one-line notice as prepend_bundled_rjq (release asset pattern, T70 dependency) instead of failing the install on cp of a missing file; regenerate install.sh via installer/build.sh as the step's generated output.

## Testing requirement

| Test required | Rationale |
|---|---|
| yes | W02 and W04 are test units (manifest rule verification with fault-injection; tracked-binary regression guard) and W06 is the end-to-end verification probe. |

## Goal-size exception
