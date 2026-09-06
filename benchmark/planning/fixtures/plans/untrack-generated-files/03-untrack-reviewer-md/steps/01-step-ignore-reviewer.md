# Step: 01-step-ignore-reviewer

## Ownership

- Goal: `03-untrack-reviewer-md`
- Work unit: `W17`
- Type: `config`

## Change target

- File: `.gitignore`
- Primary symbol or file scope: `REVIEWER.md entry`
- Subscope: `N/A`

## Objective

§ 4.1
Add planning/REVIEWER.md to .gitignore with a comment naming generate-reviewer.sh and the SKILL.md SHA-256 pin as the producer and its contract.

## Instructions

§ 5.1
Add planning/REVIEWER.md to .gitignore under its own section, with a comment naming planning/scripts/generate-reviewer.sh as the producer and the SKILL.md SHA-256 pin as the freshness contract. Run git rm --cached planning/REVIEWER.md.

## Acceptance criteria

§ 6.1
git ls-files planning/REVIEWER.md prints nothing; git check-ignore -v names the new rule; the file remains on disk so local reads keep working until consumers are reconciled.

## Handoff

§ 7.1
No downstream reliance beyond the untracked state that W18, W21 and W22 reconcile.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
