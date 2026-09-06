# Step: 01-step-ignore-portability

## Ownership

- Goal: `04-untrack-portability-md`
- Work unit: `W25`
- Type: `config`

## Change target

- File: `.gitignore`
- Primary symbol or file scope: `PORTABILITY.md entry`
- Subscope: `N/A`

## Objective

§ 4.1
Add PORTABILITY.md to .gitignore with a comment naming generate-portability.sh and portability-rules.json as the producer.

## Instructions

§ 5.1
Add PORTABILITY.md to .gitignore under its own section, with a comment naming ./generate-portability.sh and portability-rules.json as the producer. Run git rm --cached PORTABILITY.md.

## Acceptance criteria

§ 6.1
git ls-files PORTABILITY.md prints nothing; git check-ignore -v names the new rule; the file remains on disk for readers until the gates are reconciled.

## Handoff

§ 7.1
No downstream reliance beyond the untracked state that W26 through W28 reconcile.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
