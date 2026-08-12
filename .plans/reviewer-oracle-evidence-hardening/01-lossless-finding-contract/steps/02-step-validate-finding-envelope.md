# Step: 02-step-validate-finding-envelope

## Ownership

- Goal: `01-lossless-finding-contract`
- Work unit: `W02`
- Type: `source`

## Change target

- File: `benchmark/planning/grade-blinded-run.sh`
- Primary symbol or file scope: `valid_envelope()`
- Subscope: `N/A`

## Objective

§ 4.1
Require the complete finding envelope, classify malformed records explicitly with per-finding reasons and an aggregate count, and retain consolidated-finding matching across multiple seeded defects.

## Instructions

§ 5.1
Validate finding_id, path, location, summary, observed_contradiction, impact, evidence, required_correction, and boolean independent before semantic matching. For every rejected finding emit this exact canonical JSON shape: `{ "schema_status": "malformed", "malformed_findings": [{ "finding_id": null, "index": 0, "reasons": ["MISSING_FIELD"] }], "malformed_count": 1, "review_state": { "reason": "REVIEW_FINDING_SCHEMA_INVALID" }, "adoption": false }`. `finding_id` is either a non-empty JSON string or JSON null; `index` is a non-negative integer; `reasons` is a non-empty array of strings from the fixed map (`MISSING_FIELD` = key absent, `EMPTY_FIELD` = required string empty, `WRONG_TYPE` = value type incorrect, `AMBIGUOUS_FINDING` = identity or target cannot be uniquely resolved). `malformed_count` equals the array length, and no semantic score is emitted for malformed input; never silently treat malformed evidence as a semantic miss.

## Acceptance criteria

§ 6.1
Valid consolidated evidence produces three true positives and independent catches. Missing path, location, summary, contradiction, impact, evidence, correction, or independence produces the explicit malformed result above and cannot produce adoption; the result must retain the per-finding reason and aggregate count in the archive. Wrong types are rejected for every field, including non-boolean independent and non-array reasons/malformed_findings.

## Handoff

§ 7.1
W03 and W04 can cite one enforced schema and the direct oracle tests cover its accepted and rejected forms.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
