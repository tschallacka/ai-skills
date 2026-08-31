# Step: 07-step-binaries-contract-test

## Ownership

- Goal: `16-toolchain-and-packaging`
- Work unit: `W111`
- Type: `test`

## Change target

- File: `tests/test-shipped-binaries.sh`
- Primary symbol or file scope: `shipped binaries contract`
- Subscope: `N/A`

## Objective

§ 4.1
Validate planning/binaries.tsv against the committed artifacts and the regenerated installer: every declared target has an artifact and checksum match, every artifact is declared, and the installer exposes exactly the declared artifact paths.

## Instructions

§ 5.1
Create tests/test-shipped-binaries.sh to validate planning/binaries.tsv in both directions against planning/bin and the regenerated install.sh. Assert every declared target has a present artifact with a matching checksum, every present artifact is declared, and the installer exposes exactly the declared artifact paths without stale removed-renderer paths. Fault-inject a missing artifact, an undeclared artifact and an installer path mismatch; each must fail.

## Acceptance criteria

§ 6.1
The repository test passes only when the registry, committed artifacts and regenerated installer agree in both directions. Each of the three fault injections fails with a diagnostic naming the mismatched registry, artifact or installer path.

## Handoff

§ 7.1
The mechanism CODE-STYLE section 1b mandates is both declared and validated, so a later skill shipping a binary has a working precedent rather than a documented intention.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
