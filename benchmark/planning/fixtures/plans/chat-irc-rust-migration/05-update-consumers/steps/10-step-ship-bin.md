# Step: 10-step-ship-bin

## Ownership

- Goal: `05-update-consumers`
- Work unit: `W45`
- Type: `config`

## Change target

- File: installer/build-release.sh
- Primary symbol or file scope: release-stage the built chat binaries (not git-ls-files)
- Subscope: `N/A`

## Objective

§ 4.1
Update the release build so it SYNTHESIZES the built rust binaries into the release tarball rather than taking them from git ls-files: build-release.sh build mode (run on a machine with cargo) runs `cargo build --release --workspace`, then copies the resulting chat-server-rs and chat-client-rs binaries into the staged root under chat/bin/ BEFORE the copy loop. DEV-ONLY preview: a developer can build them into chat/bin/ with cargo in the nix dev shell; the shipped install.sh never builds. The skill ships TWO binaries (server + client), so chat/binaries.tsv declares one row per (target, binary); tests/test-shipped-binaries.sh is extended to allow a skill to declare multiple binaries per target (one row each), each accounted for and tested.

## Instructions

§ 5.1
<direct action on this one target>

## Acceptance criteria

§ 6.1
<observable result for this target>

## Handoff

§ 7.1
<what the next named work unit can rely on>

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
