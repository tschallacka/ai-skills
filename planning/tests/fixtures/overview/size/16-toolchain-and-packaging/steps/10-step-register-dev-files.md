# Step: 10-step-register-dev-files

## Ownership

- Goal: `16-toolchain-and-packaging`
- Work unit: `W114`
- Type: `config`

## Change target

- File: `installer/src/50-manifest.sh`
- Primary symbol or file scope: skill_files dev and prod registration arms
- Subscope: `N/A`

## Objective

§ 4.1
Register the checked-in fixture corpus in the dev arm and planning/binaries.tsv in the prod arm of installer/src/50-manifest.sh; W17 owns the planning-arm artifact entries, so these scopes do not overlap.

## Instructions

§ 5.1
Update only the dev and prod registration arms in installer/src/50-manifest.sh: register the checked-in fixture corpus in the dev arm and planning/binaries.tsv in the prod arm. W17 separately owns the planning-arm artifact entries. Do not regenerate install.sh here; W113 owns that generated output.

## Acceptance criteria

§ 6.1
The fixture corpus is registered in the dev arm, planning/binaries.tsv is registered in the prod arm, W17's planning-arm entries remain intact, and each promised path exists. The marker and skill-file gates pass after W115 and W110 create the files.

## Handoff

§ 7.1
W113 regenerates install.sh downstream of this unit, so the generated installer carries these entries rather than being regenerated before they exist. W48 can assert that the skill-files manifest gate passes on a tree that contains the fixture corpus and the binaries registry. Adversarial findings AR-29 and AR-30 recorded that the corpus, the registry and their tests were tracked but in neither arm, so the gate W48 depends on could not have passed.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
