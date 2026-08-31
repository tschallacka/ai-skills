# Goal: The toolchain and packaging surface

## Current state and prior-goal handoffs

§ 2.1
Depends on goal 03 for artifacts to place and publish, on goal 17 for the compiler that builds them, and on goal 13 for the requires.tsv rows W80 removes, since W113 regenerates the installer from them. Goal 13 in turn depends on this goal for that regeneration, through W80 to W113 to W81; the unit graph stays acyclic and the two goals interleave rather than nesting. Confirmed by the adversarial review, findings AR-06 and AR-19: skill_files() is a flat hand-written list with no platform conditional, package.json includes the planning directory wholesale, and CODE-STYLE section 1b names a binaries.tsv registry and a test-shipped-binaries.sh validator that do not exist. An earlier version of this paragraph also asserted that no row declares the Rust toolchain, which AR-27 refuted and goal 17 now owns (AR-45), and it recorded no dependency on goal 13 at all (AR-61).

## Outcome and definition of done

§ 3.1
What ships is declared once, validated against the tree, and reaches a host as exactly the one artifact that runs there. Demonstrated by installing on a host of each declared platform and executing the placed artifact, by installing on a host matching none and reaching the stated unavailability, and by packing the published tarball to the recorded file list and size. An earlier version of this definition of done read that the compiler is a declared development dependency, which no unit in this goal delivers and which moved to goal 17 in the split; adversarial finding AR-45 recorded it.

## Why this goal is needed

§ 4.1
A binary is a new kind of shipped file for this repository, and the things that decide who gets it are the registry that declares it, the tree that holds it, the installer that places it and the package that publishes it. None of them had an owner. Left alone, the plan would declare five artifacts in a file CODE-STYLE names but nothing creates, ship all five to every machine, and grow the published tarball by a number nobody stated.

## Scope

§ 5.1
In scope: the binaries registry and its validator, the artifacts committed into the tree, per-host artifact selection at install time, what the published package carries, the skill_files arms that account for every tracked file this plan adds, and the regeneration of install.sh that all of those imply. Out of scope: building the artifacts, which goal 03 owns; the compiler that builds them, which goal 17 owns; and the unavailability notice for a host with no matching artifact, which W18 owns.

## Affected files, systems, data, and interfaces

§ 6.1
planning/binaries.tsv is new and declares what ships per target; tests/test-shipped-binaries.sh is new and validates it; planning/bin gains the five committed artifacts; planning/tests/fixtures/overview/npm-package-baseline.tsv is the single W105-owned machine-readable npm tarball file-list and byte-size record consumed by W120; installer/src/50-manifest.sh gains the skill_files entries for the fixture corpus and the registry; installer/src/20-runtime-tools.sh gains the host-to-artifact selection; package.json's files field is decided with the resulting tarball size recorded only in that baseline; and install.sh is regenerated from all of it. An earlier version of this paragraph named flake.nix, which is goal 17's, and named none of the files this goal's own units own; adversarial finding AR-45 recorded it.

## Dependencies and handoffs

§ 7.1
Depends on goal 17 for the Rust 1.86.0 compiler and goal 03 for artifacts. Its packaging branch runs W104 W105 W110 W111 W113 W114 and W115, while goal 13 consumes W113 before W81 and W82 can reconcile the removal declarations. The unit graph is acyclic even though these goal narratives interleave: goal 09 joins both branches only after W48 prerequisites are complete.

## Implementation approach, risks, and edge cases

§ 8.1
Approach: declare first, then commit, then select, then regenerate, and let the manifest and the map catch up afterwards in goal 13 — each step's output is the next one's input, so the order is recorded in the dependencies rather than left to chance. The chain does not end inside this goal: its last consumers are W81 and W82, which goal 13 owns, and adversarial finding AR-61 recorded that this paragraph described a self-contained sequence after AR-54 had made it a shared one. Risk: selection by uname pair is exactly where a host is misidentified, and the failure is silent, a binary placed that cannot run; the acceptance criteria therefore require the placed artifact to be executed once rather than merely present. Risk: excluding artifacts from the tarball silently disables the overview for npm consumers, which is a different decision from shipping them, so whichever is chosen is recorded with its consequence stated. Risk: a tracked file added under a skill and registered in neither arm fails a gate that no unit in this goal runs, so W114 registers them and W48 is downstream; the same class already cost this repository a vacuous pass on the day these units were written. Edge case: a host whose pair matches no artifact must reach W18's notice rather than this goal's selection failing.

## Owned work units

§ 9.1
`W104` — Implement one normalize_platform function for production uname and PROCESSOR_ARCHITECTURE inputs plus test-only PLAN_OVERVIEW_TEST_MODE inputs PLAN_OVERVIEW_TEST_OS and PLAN_OVERVIEW_TEST_ARCH. Use it for artifact selection and placement and route unmatched keys to W18.

§ 9.2
`W105` — Include all five target artifacts and install.sh in the npm package and generate or refresh the single repository-owned baseline at planning/tests/fixtures/overview/npm-package-baseline.tsv. The baseline is tab-separated with a fixed header, one sorted tarball path and byte-size row per packaged file, and one final tarball-bytes row; W105 owns its generation and refresh after npm pack. W120 reads this exact file and creates no alternate expectations.

§ 9.3
`W106` — Install from the repository and from the packed npm tarball on each declared platform and on one unsupported pair, execute the selected artifact, and run the installed artifact in render and serve modes with the prohibited interpreter stack unavailable or trapped.

§ 9.4
`W110` — Declare per target which artifact the skill ships: the uname pair, the artifact path and its checksum. CODE-STYLE section 1b names this file as how a skill declares what ships; it does not exist in the repository, and this plan is the first thing to ship a binary, so this unit is where the declared mechanism becomes real.

§ 9.5
`W111` — Validate planning/binaries.tsv against the committed artifacts and the regenerated installer: every declared target has an artifact and checksum match, every artifact is declared, and the installer exposes exactly the declared artifact paths.

§ 9.6
`W113` — Regenerate and commit install.sh after the declarations it is generated from have changed. It is a tracked generated file whose freshness gate diffs it against its source, so leaving it uncommitted reddens two gates while no unit owns the file. This is the inseparable generated-file exception: build.sh writes the whole file in one pass, so individual review is impossible.

§ 9.7
`W114` — Register the checked-in fixture corpus in the dev arm and planning/binaries.tsv in the prod arm of installer/src/50-manifest.sh; W17 owns the planning-arm artifact entries, so these scopes do not overlap.

§ 9.8
`W115` — Commit the five prebuilt artifacts the matrix builds, one per target, named by uname pair. The manifest gate requires every file skill_files() promises to exist on disk, so the artifacts must be in the tree rather than fetched at install time, and this is where the repository takes on their committed size as a stated cost rather than an accident.

§ 9.9
`W119` — Test W104 normalize_platform through the installer path with PLAN_OVERVIEW_TEST_MODE=1 and explicit supported OS and architecture values including Windows_NT AMD64 and one unsupported Windows architecture. Assert normalized keys selected paths and no artifact plus the unavailable notice.

§ 9.10
`W120` — Pack and extract the npm tarball in a clean temporary directory and compare it with the single W105-owned planning/tests/fixtures/overview/npm-package-baseline.tsv record. Invoke the generated installer through W104 test-only platform inputs and assert the supported and unsupported results without defining a second package file list or byte-size baseline.

## Testing requirement

| Test required | Rationale |
|---|---|
| yes | Every claim here is about what a machine ends up holding, and each fails silently: an unregistered tracked file reddens a gate nobody ran, a mis-selected artifact is present but unrunnable, a stale generated installer diffs against its own source, and a packaging change is invisible until a consumer installs. The proofs are an install on a host of each declared platform with the placed artifact executed once, a packed tarball listed and sized, and the registry validated against the tree in both directions. An earlier version of this rationale argued from an undeclared toolchain, which belongs to goal 17. |
## Goal-size exception
