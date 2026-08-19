# Step: 01-step-preserve-finding-envelope

## Ownership

- Goal: `01-lossless-finding-contract`
- Work unit: `W01`
- Type: `source`

## Change target

- File: `benchmark/planning/setup-benchmark.sh`
- Primary symbol or file scope: `approval-to-oracle evidence serialization block`
- Subscope: `N/A`

## Objective

§ 4.1
Preserve every approved finding field when constructing oracle-terminal-evidence.json, including finding ID, repository-relative path, precise location, summary, observed contradiction, impact, evidence, required correction, and independence provenance.

## Instructions

§ 5.1
Copy the approved_findings object field-for-field into oracle-terminal-evidence.json. Normalize only the external finding_id alias and add independent provenance from the selected Reviewer B role; never replace semantic fields with a classification label.

## Acceptance criteria

§ 6.1
A complete consolidated finding reaches the grader with non-empty finding_id, path, location, summary, observed_contradiction, impact, evidence, required_correction, and boolean independent fields. No source evidence is silently dropped.

## Handoff

§ 7.1
W02 receives a stable envelope and can distinguish a valid consolidated finding from an ID-only artifact.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
