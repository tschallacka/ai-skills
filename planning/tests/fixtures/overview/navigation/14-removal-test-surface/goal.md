# Goal: The removal's test surface

## Current state and prior-goal handoffs

§ 2.1
Depends on goal 13 for the files to be gone. Confirmed by reading the suite: five tests name the removed renderer, its wrapper or its template. Two drive them directly with twelve and four assertions, one delegates its own coverage to a test that is being retired, one counts a canonicalisation site inside the renderer, and one carries an allowlist arm exempting the wrapper from a portability rule.

## Outcome and definition of done

§ 3.1
Every test that exercised the old renderer is retired or rewritten against the binary, so the suite neither tests a deleted file nor silently loses the coverage it had. Demonstrated by a green suite in which no test names the removed renderer and the checks that mattered have named replacements.

## Why this goal is needed

§ 4.1
Retiring a test is how coverage is lost quietly. Each of these tests asserted something real; the point of this goal is that the assertion survives even when the file it pointed at does not, so the suite ends up green because it is right and not because it stopped looking.

## Scope

§ 5.1
In scope: the five tests that name the removed files, and the replacement assertions against the binary. Out of scope: new tests for behaviour the old renderer never had, which belong to the goals that introduce that behaviour.

## Affected files, systems, data, and interfaces

§ 6.1
planning/tests/test-plan-overview.sh, test-overview-serve.sh, test-plan-dir-synonym.sh, test-duplication-ratchet.sh and test-portability-contract.sh.

## Dependencies and handoffs

§ 7.1
Depends on goal 13. Hands to goal 09 a suite with no test naming a deleted file, which is one of the repository gates its verification records.

## Implementation approach, risks, and edge cases

§ 8.1
Approach: for each retired assertion, name its replacement in the same step rather than deleting it and trusting a later goal to notice. Risk: a test that is deleted rather than rewritten takes its coverage with it silently, which is why each step states what the assertion becomes. Edge cases: the duplication ratchet is a count, so removing a site without correcting the number reddens the suite for the right reason at the wrong place; and a stale allowlist arm in the portability contract hides the next genuine violation rather than merely being untidy.

## Owned work units

§ 9.1
`W85` — Retire the twelve assertions that drive render-plan-overview.sh directly and replace them with the equivalents against the binary, so the coverage the suite had is not lost with the file it tested.

§ 9.2
`W86` — Retire the four assertions that drive overview-serve.sh and replace them with serve-mode assertions against the binary, including the port-printed-before-first-request property.

§ 9.3
`W87` — Correct the note that delegates plan-dir coverage to test-overview-serve.sh, and cover the synonym directly rather than by reference to a retired test.

§ 9.4
`W88` — Correct the ratchet entry that counts render-plan-overview.sh cells() as a canonicalisation site. The count is the assertion, so removing a site without correcting it reddens the suite.

§ 9.5
`W89` — Remove the allowlist arm that exempts overview-serve.sh from the python3-shipped rule. With the wrapper gone the exemption has nothing to exempt, and a stale allowlist arm hides the next real violation.

## Testing requirement

| Test required | Rationale |
|---|---|
| yes | The goal is entirely test work: five suite files are retired or rewritten against the binary, and the risk it exists to manage is coverage lost silently with a deleted test. Each step names the replacement assertion, and the goal is demonstrated by running the rewritten tests green while a fault injection proves each replacement bites. |

## Goal-size exception
