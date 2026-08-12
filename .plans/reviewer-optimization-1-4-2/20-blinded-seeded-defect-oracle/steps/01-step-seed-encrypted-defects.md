# Step: 01-step-seed-encrypted-defects

## Ownership

- Goal: `20-blinded-seeded-defect-oracle`
- Work unit: `W86`
- Type: `source`

## Change target

- File: `benchmark/planning/seed-blinded-defects.sh`
- Primary symbol or file scope: `encrypted seeded-defect fixture creation`
- Subscope: `N/A`

## Objective

Create realistic defects in an isolated copy and encrypt the mapping before
targets start.

## Instructions

Define stable opaque defect IDs, apply deterministic or recorded modifications
to a temporary target copy, write the mapping as encrypted data, hash the
modified inputs and encrypted manifest, and keep the key outside target
arguments, environment, capsules, and logs.

## Acceptance criteria

The target copy contains the intended defects, the plaintext mapping is absent
from target-visible paths, and the encrypted manifest/key handoff is auditable.

## Handoff

Pass only the defective workspace and non-secret hashes to W87.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
