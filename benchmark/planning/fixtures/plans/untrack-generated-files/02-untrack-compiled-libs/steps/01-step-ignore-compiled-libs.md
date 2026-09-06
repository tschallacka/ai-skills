# Step: 01-step-ignore-compiled-libs

## Ownership

- Goal: `02-untrack-compiled-libs`
- Work unit: `W07`
- Type: `config`

## Change target

- File: `.gitignore`
- Primary symbol or file scope: `compiled plan libraries section`
- Subscope: `N/A`

## Objective

§ 4.1
Add a compiled-libraries section ignoring planning/scripts/plan-core-lib.sh, plan-crypt-lib.sh, plan-document-lib.sh, plan-progress-lib.sh and plan-table-lib.sh, with a comment naming build-plan-libs.sh as the producer.

## Instructions

§ 5.1
Add a compiled-libraries section to .gitignore listing the five paths planning/scripts/plan-core-lib.sh, plan-crypt-lib.sh, plan-document-lib.sh, plan-progress-lib.sh, plan-table-lib.sh, with a comment naming planning/scripts/build-plan-libs.sh as the producer and MAINTAINER.md section 2.15 as the rule. Run git rm --cached on each of the five.

## Acceptance criteria

§ 6.1
git ls-files names none of the five; git check-ignore -v names the new rule for each; the lib/ source tree and build-plan-libs.sh remain tracked; the working tree still has the five files so the suite keeps running.

## Handoff

§ 7.1
No downstream reliance beyond the ignored, still-built-on-disk state the suite bootstrap depends on.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
