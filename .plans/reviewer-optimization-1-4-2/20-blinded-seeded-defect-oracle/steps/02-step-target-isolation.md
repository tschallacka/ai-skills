# Step: 02-step-target-isolation

## Ownership

- Goal: `20-blinded-seeded-defect-oracle`
- Work unit: `W87`
- Type: `source`

## Change target

- File: `benchmark/planning/setup-benchmark.sh`
- Primary symbol or file scope: `blinded target launch boundary`
- Subscope: `N/A`

## Objective

Run targets against the defective copy without exposing the key or plaintext
defect mapping, then hand immutable evidence to a separate oracle process.

## Instructions

Launch seeder, worker, iterative reviewer, fresh reviewer, analyzer, and oracle
with separate capsules and roles. Ensure target processes inherit only allowed
inputs. Refuse self-grading, missing role identity, key exposure, or incomplete
terminal evidence.

## Acceptance criteria

Target audit records prove no key/map access, and the oracle starts only after
target completion with separate session identity and input hashes.

## Handoff

Pass target transcripts, lifecycle evidence, and hashes to W88 without the
plaintext defect map in the target capsule.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
