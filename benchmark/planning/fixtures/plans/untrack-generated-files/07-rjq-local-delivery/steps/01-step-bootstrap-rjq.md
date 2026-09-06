# Step: 01-step-bootstrap-rjq

## Ownership

- Goal: `07-rjq-local-delivery`
- Work unit: `W37`
- Type: `source`

## Change target

- File: `bootstrap.sh`
- Primary symbol or file scope: `rjq build-if-missing arm`
- Subscope: `N/A`

## Objective

§ 4.1
New repo-root bootstrap.sh: when rjq is neither on PATH nor at planning/bin/<triple>/rjq, build src/rjq (cargo build --release) into the gitignored planning/bin/<triple>/ path the installer's prepend logic already knows, print the PATH line to export, and exit non-zero if cargo is missing with a message naming the T70 release download as the no-toolchain alternative.

## Instructions

§ 5.1
Create repo-root bootstrap.sh (MODE: DEV, bash 3.2, shellcheck-clean): detect the host triple the same way installer rows match uname; when rjq is on PATH, do nothing and exit 0; when planning/bin/<triple>/rjq exists, print the PATH prepend line; otherwise cargo build --release --manifest-path src/rjq/Cargo.toml and copy the binary into the gitignored planning/bin/<triple>/ path (mkdir -p), then print the PATH line. Without cargo, exit 69 with a message naming the T70 release download as the no-toolchain alternative.

## Acceptance criteria

§ 6.1
On a clean checkout without rjq, running bootstrap.sh builds the binary to the gitignored path and prints a working PATH line; with rjq present it is a no-op exiting 0; without cargo it exits 69 with the named alternative; the script never writes outside planning/bin and the repo.

## Handoff

§ 7.1
W08's bootstrap invokes this script; W38's message names it.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
