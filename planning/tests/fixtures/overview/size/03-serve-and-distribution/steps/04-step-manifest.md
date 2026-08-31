# Step: 04-step-manifest

## Ownership

- Goal: `03-serve-and-distribution`
- Work unit: `W17`
- Type: `config`

## Change target

- File: `installer/src/50-manifest.sh`
- Primary symbol or file scope: skill_files planning arm artifact entries
- Subscope: `N/A`

## Objective

§ 4.1
Add only the planning-arm artifact entries and remove only the deleted renderer entries from installer/src/50-manifest.sh; W114 owns the dev and prod registration arms in the same file.

## Instructions

§ 5.1
Update only the planning-arm artifact entries in installer/src/50-manifest.sh so the prebuilt artifacts are declared and the removed scripts are not. W114 separately owns the dev and prod registration arms in this file. Do not regenerate the installer here: install.sh is W113's change target.

## Acceptance criteria

§ 6.1
The planning arm names every artifact the matrix builds and none of the removed scripts, while W114's dev and prod registrations remain a separate non-overlapping edit. tests/test-skill-files-manifest.sh passes after W115 commits the artifacts. W104 alone decides which one a host receives.

## Handoff

§ 7.1
W18 adds the message for a platform with no artifact; W20 verifies that path.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
