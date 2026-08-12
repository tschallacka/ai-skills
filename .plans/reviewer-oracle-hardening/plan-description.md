# Plan: Reviewer oracle and approval-state hardening

## Current state

§ 2.1
The current 1.4.2 pilot completed technically, but its oracle scored zero
because it required hidden seeded IDs (`SD-01`–`SD-03`) to equal reviewer
finding IDs. Reviewer B actually consolidated all three defects into `AR-01`.
The evidence is preserved in `benchmark/results/20260811T130218Z-current-fresh8/`.

§ 2.2
The harness extracts `approved_findings` from Reviewer B approval and grades
exact IDs in `benchmark/planning/grade-blinded-run.sh`. It also allows a false
`overall_plan_approval` in blinded-oracle mode, so a completed review can be
reported as accepted even when the plan was not approved.

## Desired outcome

§ 3.1
Harden the reviewer/oracle test protocol so consolidated reviewer findings are
semantically matched to hidden seeded defects, approval state is represented
separately from review completion, and regression tests prevent both failures.

§ 3.2
Definition of done: an end-to-end fixture with three defects in one file and
one consolidated reviewer finding reports three semantic true positives; a
negative overall approval cannot be marked adoptable; reports expose semantic,
mechanical, and approval metrics; focused and full relevant tests pass.

## Approach

§ 4.1
First define a private semantic defect contract and an independent adjudication
format. Then update seed, evidence, and grading paths to map findings to one or
more defects without revealing seed IDs to reviewers.

§ 4.2
Separately enforce approval-state semantics in the harness and reports. Add
regression fixtures for consolidated findings, multi-defect files, false
approval, missing approval, and independent-oracle role enforcement before
running the complete verification suite.

## Scope

§ 5.1
In scope: blinded defect manifest schema, semantic defect adjudication,
consolidated finding coverage, oracle metrics, Reviewer B approval-state
handling, protocol metadata, analyzer/report fields, and automated contract
tests.

§ 5.2
Out of scope: rerunning or modifying historical 1.3.1/1.4.1 archives, changing
the maintainer no-backward-compatibility contract, forcing one finding per
defect, changing the planning workflow's user-facing plan format, or claiming
adoption before current-protocol thresholds pass.

## Affected areas

§ 6.1
Primary harness files: `benchmark/planning/seed-blinded-defects.sh`,
`grade-blinded-run.sh`, `review-oracle.sh`, `setup-benchmark.sh`, analyzer
prompts/reporting, and `benchmark/planning/tests/`.

§ 6.2
Reviewer contract surfaces are `planning/SKILL.md`, generated
`planning/REVIEWER.md`, and reviewer prompt/schema validation where approval
and finding evidence are specified.

## Constraints and decisions

§ 7.1
Seed IDs and mutation keys remain private. Reviewers receive only the mutated
plan and normal review capsule; only the independent oracle/adjudicator may
read the decrypted semantic manifest.

§ 7.2
A finding may cover multiple seeded defects. Exact hidden-ID matching remains
an optional diagnostic, never the authoritative semantic score.

§ 7.3
`overall_plan_approval=false` is valid terminal review evidence for measuring
detection, but it makes the run non-adoptable and must be visible as such.

## Risks and open questions

§ 8.1
Semantic adjudication can become subjective. Bound it with required path,
location, issue summary, correction, adjudicator evidence, and deterministic
fixture cases; record ambiguous matches as unresolved rather than true.

§ 8.2
The semantic fixture oracle is deterministic: it receives a redacted candidate
finding envelope and the private defect manifest, then emits one adjudication
row per seeded defect. A row is `true_positive` only when path/location,
observed contradiction, and required correction are all supported; missing
evidence is `unresolved`, and conflicting evidence is `ambiguous`. Production
model selection is not part of this plan; the deterministic contract is the
release-gate test oracle.

§ 8.3
Public reviewer material is limited to `/tmp/<run>/<revision>/workspace` and
the reviewer capsule. Private material is
`/tmp/ai-skills-oracle-private/<run>/<revision>/`, containing the key, encrypted
manifest, seed metadata, and immutable target snapshot. Only the independent
oracle process may read the private root. Published archives may contain
redacted oracle metrics and evidence paths, never keys, decrypted manifests,
defect paths, or private-root names. Negative-access tests must assert these
boundaries.

§ 8.4
The machine-readable state contract is: `review_completed`, `plan_approved`,
`oracle_completed`, and `adoptable` are booleans; `fail_closed_reasons` is an
array of enum strings. `adoptable` is true only when review and plan approval
are true, oracle completion is true, taint is absent, no conflict/ambiguity is
present, and configured semantic and independent-catch thresholds pass. A
missing threshold or denominator is fail-closed. False approval remains
gradeable but forces `adoptable=false`.

§ 8.5
The public state schema is JSON: `review_completed:boolean`,
`plan_approved:boolean`, `oracle_completed:boolean`, `adoptable:boolean`,
`semantic_true_positive_rate:number|null`,
`independent_catch_rate:number|null`, `seeded_denominator:integer`,
`fail_closed_reasons:string[]`, and `approval_conflict:boolean`. Allowed reason
enums are `REVIEW_INCOMPLETE`, `PLAN_NOT_APPROVED`, `APPROVAL_MISSING`,
`APPROVAL_CONFLICT`, `ORACLE_INCOMPLETE`, `ORACLE_AMBIGUOUS`, `TAINTED_RUN`,
`MISSING_DENOMINATOR`, `SEMANTIC_THRESHOLD_FAILED`, and
`INDEPENDENT_THRESHOLD_FAILED`. Reasons are sorted and deduplicated; all
applicable reasons are retained, with `TAINTED_RUN` and approval conflicts not
masking other causes. The only adoptable row is all four booleans true, no
reasons, no conflict, positive denominator, and both configured rates at or
above thresholds. Every other combination is non-adoptable.

§ 8.6
The semantic adjudication envelope is JSON. Candidate input contains
`finding_id`, `path`, `location`, `summary`, `evidence`, `required_correction`,
and `independent:boolean`. Private output contains one row per defect with
`defect_id`, `finding_ids:string[]`, `classification` (`true_positive`,
`false_positive`, `unresolved`, `ambiguous`, or `duplicate`),
`confidence` (`high`, `medium`, or `low`), and `rationale`. Matching normalizes
case and whitespace only; path and location must identify the target, and both
the contradiction and correction must be supported. Precedence is duplicate,
ambiguous, unresolved, true positive, then false positive. Published output
removes defect IDs, mutation strings, private paths, keys, and rationales that
would reveal the manifest, retaining only counts, rates, redacted evidence
references, and the mechanical exact-ID diagnostic.

§ 8.7
The mandatory current gate uses:
`BLINDED_ORACLE_SPEC=benchmark/planning/pilot-blinded-defects.json CODEX_MODEL=gpt-5.5 benchmark/planning/run-benchmark.sh hardening-current /tmp/reviewer-oracle-hardening-current --sequential --fresh-review --revisions current`.
W11 records the emitted run ID, then requires `current/evaluation.md`,
`current/oracle.json`, `current/telemetry.json`, `current/reviewer-lifecycle.jsonl`,
and batch `comparison.md`. It asserts worker/analyzer exit 0, validation and
structural/process audits pass, telemetry is available, lifecycle independence
is true, semantic coverage is 3/3 for the fixture, and `adoptable=false` when
the fixture's plan approval is false. Historical archives are never inputs.

§ 8.8
`.env` is tooling metadata, not review input. Capsule-local reviewers must
override `PLANS_ROOT`, `PLAN_ROOT`, `PLAN_DESCRIPTION_FILE`,
`PLAN_PROGRESS_FILE`, and `PLAN_WORK_UNIT_INVENTORY` to the capsule plan path;
they may not source host paths. The capsule review must record this override in
its scope evidence.

§ 8.9
The required capsule override is executable as:
`CAPSULE_PLAN="$PWD"; export PLANS_ROOT="$CAPSULE_PLAN" PLAN_ROOT="$CAPSULE_PLAN" PLAN_DESCRIPTION_FILE="$CAPSULE_PLAN/plan-description.md" PLAN_PROGRESS_FILE="$CAPSULE_PLAN/progress.md" PLAN_WORK_UNIT_INVENTORY="$CAPSULE_PLAN/work-unit-inventory.md"; test "$PLAN_ROOT" = "$CAPSULE_PLAN"; test "$PLAN_DESCRIPTION_FILE" = "$CAPSULE_PLAN/plan-description.md"; test "$PLAN_PROGRESS_FILE" = "$CAPSULE_PLAN/progress.md"; test "$PLAN_WORK_UNIT_INVENTORY" = "$CAPSULE_PLAN/work-unit-inventory.md"; case "$PLANS_ROOT:$PLAN_ROOT:$PLAN_DESCRIPTION_FILE:$PLAN_PROGRESS_FILE:$PLAN_WORK_UNIT_INVENTORY" in *'/home/mdibbets/.plans'*|*'/home/mdibbets/git/ai-skills/.plans'*|*'/ai-skills-oracle-private/'*) exit 1;; esac`.
The reviewer must record the command result and may not source the copied
`.env`; the capsule-local variables are authoritative. The negative check is
against the effective exported plan variables, because the copied `.env` may
retain host-specific defaults that are deliberately non-authoritative, while
the active capsule itself may validly reside under `/tmp/`.

§ 8.10
Artifact boundary contract:

| Artifact | Owner | Visibility | Allowed fields | Forbidden fields | Required test |
|---|---|---|---|---|---|
| reviewer capsule/plan | harness | reviewer | mutated plan and public task context | seed IDs, keys, manifest, private root | W09 |
| candidate envelope | reviewer/oracle bridge | oracle input | finding ID, path, location, summary, evidence, correction, independence | seed IDs, keys, mutation old/new text | W09 |
| encrypted manifest/key/snapshot | seed/oracle | oracle-only | encrypted defect data and target snapshot | reviewer/analyzer access | W09 |
| private adjudication rows | oracle | oracle-only | defect ID, classifications, confidence, rationale | publication access | W09 |
| oracle.json | oracle | published redacted | counts, rates, denominators, state, redacted evidence refs | defect IDs, keys, private paths, mutation strings | W09 |
| telemetry/lifecycle/evaluation | harness | published | session, lifecycle, usage, state, taint reasons | keys, manifests, private roots, defect details | W09 |
| comparison.md | analyzer | published | cohort metrics, state, thresholds, reasons | secrets and hidden defect identities | W09 |

Paths in reviewer-visible evidence are repository-relative public paths only;
private filesystem paths are replaced with the literal `<private>` token before
publication. Hidden defect IDs are replaced with aggregate counts.

§ 8.11
State truth table:

| Scenario | review_completed | plan_approved | oracle_completed | fail_closed_reasons | adoptable |
|---|---:|---:|---:|---|---:|
| complete approval, valid oracle, all thresholds pass | true | true | true | `[]` | true |
| false overall approval | true | false | true | `PLAN_NOT_APPROVED` | false |
| missing approval | true | false | true | `APPROVAL_MISSING`, `PLAN_NOT_APPROVED` | false |
| conflicting approvals | true | false | true | `APPROVAL_CONFLICT`, `PLAN_NOT_APPROVED` | false |
| reviewer incomplete | false | false | false | `REVIEW_INCOMPLETE`, `ORACLE_INCOMPLETE` | false |
| oracle incomplete | true | true | false | `ORACLE_INCOMPLETE` | false |
| oracle ambiguous | true | true | true | `ORACLE_AMBIGUOUS` | false |
| tainted run | true | true | true | `TAINTED_RUN` | false |
| missing denominator/threshold | true | true | true | `MISSING_DENOMINATOR` | false |
| semantic threshold failed | true | true | true | `SEMANTIC_THRESHOLD_FAILED` | false |
| independent threshold failed | true | true | true | `INDEPENDENT_THRESHOLD_FAILED` | false |

Multiple reasons are sorted lexically and deduplicated. Malformed or duplicate
state input maps to the corresponding missing/conflict reason and never to
adoptable.

## UI classification

- UI affected: no
- Rationale: This changes benchmark/oracle tooling and reports, not a user-facing UI.

## Adversarial review

- Artifact: `adversarial-review.md`
- Status: ✅ approved
