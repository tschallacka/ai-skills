# Plan: Adopt shared test reporting and accept --plan-dir on every helper

## Current state

§ 2.1
Two helper families diverge. plan-context.sh and run-adversary-probe.sh take --plan-dir; 20 other helpers take the plan directory positionally, so a reader who learned the flag from the bounded reader has the call refused elsewhere. Separately, 33 test files each define their own note_fail or fail reporter over a local counter, and a counter incremented inside a command substitution is discarded with the subshell, which made two tests inert until a mutation exposed it.

## Desired outcome

§ 3.1
Every helper that takes a plan directory accepts --plan-dir as a synonym, proven byte-identical to the positional form. Every test that reports findings uses one shared reporter whose findings survive a subshell.

## Approach

§ 4.1
Reuse plan_hoist_plan_dir, which already moves the flag value into the positional slot and prints the argument list %q-quoted, so each helper keeps its own parser and adopting the flag is one line. For the tests, point each local reporter at the shared t_fail and replace the counter epilogue with t_end, leaving all 489 call sites and every message untouched.

## Scope

§ 5.1
Included: the 20 positional helpers, and the 33 tests that already report findings. Excluded: cleanup-plans.sh, which operates on a plans root rather than a plan directory, so --plan-dir would be a lie there. Excluded: the 16 tests already converted to t_trap_assertions, and rewriting the 489 individual call sites.

## Affected areas

§ 6.1
planning/scripts/*.sh for the flag; planning/tests/*.sh and benchmark/planning/tests/*.sh for the reporters; planning/tests/lib-test.sh as the shared seam; test-flag-form-equivalence.sh and test-flag-coverage.sh as the gates.

## Constraints and decisions

§ 7.1
No new dependency. bash 3.2 remains the floor, so no declare -A, no mapfile, and printf %q must round-trip. Behaviour must not change on the passing path: a captured baseline of all 59 test outputs and exit statuses is the acceptance evidence.

## Risks and open questions

§ 8.1
Risk: a hoist inserted before the library is sourced leaves plan_hoist_plan_dir undefined, which happened once already and shows up as command not found. Risk: renaming a reporter also renames the fail counter variable. Open question: whether to follow the delegation with a full rename of the local reporter names.

## Environment facts

§ 9.1
Verified in this repository: 57 tests pass on bash 5.3 and on real bash 3.2.57; installer/build.sh --check, generate-portability.sh --check and ./blast-radius.sh are clean. The before-baseline lives outside the repository in the session scratchpad.

## Approach decisions

§ 10.1
Delegation over rewriting: pointing 33 reporters at one implementation touches four lines per file instead of 489 call sites, and buys the whole correctness property. A full rename is a separable second step that can be abandoned without leaving the tree inconsistent.

## UI classification

- UI affected: no
- Rationale: <why>

## Adversarial review

- Artifact: `adversarial-review.md`
- Status: 💤 pending
