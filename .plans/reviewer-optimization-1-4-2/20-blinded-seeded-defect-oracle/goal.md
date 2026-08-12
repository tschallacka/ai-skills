# Goal: Implement the blinded seeded-defect oracle protocol

## Current state and prior-goal handoffs

The plan contains an oracle calculator and synthetic contract fixtures, but no
real protocol that creates hidden defects, isolates the encrypted defect map
from targets, and has an independent oracle grade the completed review.

## Outcome and definition of done

A real benchmark protocol creates an isolated defective copy, encrypts the
defect mapping with an ephemeral key held only by the seeder/oracle session,
runs worker and reviewer targets without the key or plaintext mapping, and
uses a separate independent oracle process to decrypt and classify the result.
The run retains auditable hashes and a machine-readable report without
publishing the defect map or key.

## Why this goal is needed

Synthetic oracle tests prove arithmetic but not independent defect detection.
The release gate needs genuine blinded classifications to compare iterative
and fresh-review modes without allowing reviewers to see the answer key.

## Scope

Include defect seeding, encrypted manifest/key custody, target isolation,
independent oracle grading, durable evidence, cleanup, and focused tests.
Exclude production-code mutation, reviewer access to the key, same-process
self-grading, and plaintext defect-map publication.

## Affected files, systems, data, and interfaces

W86 owns the seeder and encrypted defect-manifest format. W87 owns isolated
target launch and key/capsule boundary enforcement. W88 owns independent
oracle grading and report output. W89 owns lifecycle and secrecy tests.

## Dependencies and handoffs

Depends on W42, W53, W57-W59, W60, W80-W85, and the existing
`benchmark/planning/review-oracle.sh`. Hands off genuine oracle reports to the
pilot release gate and comparison analyzer.

## Implementation approach, risks, and edge cases

The seeder writes only an isolated temporary copy and encrypted mapping, keeps
the random key in session memory or a private oracle capsule, and never passes
it through target arguments or inherited environment. The target receives only
the defective workspace and normal task inputs. After terminal target
evidence, a separate oracle process decrypts the mapping, verifies input and
transcript hashes, classifies true/false positives, false negatives,
duplicates, unresolved findings, and independent catches, then writes the
report. Interrupted runs retain a rejection/blocker record and never infer a
pass.

## Testing requirement

| Test required | Rationale |
|---|---|
| yes | Blinding, key isolation, independent grading, and evidence retention are executable safety properties. |

## Owned work units

§ 9.1
W86 — Create isolated seeded defects and encrypted mapping. W87 — Enforce target isolation and independent-oracle handoff. W88 — Grade blinded runs and write the independent oracle report. W89 — Test seeding, isolation, secrecy, and complete oracle classification. W98 — Provide the standalone independent blinded-run grader. W100 — Verify the standalone grader through the blinded protocol regression.

## Goal-size exception

Not applicable: seeding, target isolation, and independent grading are three
distinct outcomes with separate ownership and verification boundaries.
