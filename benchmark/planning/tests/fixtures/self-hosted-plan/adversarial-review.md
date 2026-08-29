# Adversarial review: harden-test-reporting-and-plan-dir

## Review scope

§ 1.1
- Request: Accept --plan-dir on every helper that takes a plan directory, and give the reporting tests one shared reporter implementation.
- Repository/context inspected: planning/scripts argument parsing across 26 helpers, the 59 tests under planning/tests and benchmark/planning/tests, planning/tests/lib-test.sh, and a baseline of every test output captured before any change.

## Findings

| AR-01 | The hoist was inserted above the line defining script_dir, so plan_hoist_plan_dir was undefined at call time and twelve helpers failed at load. The differential harness compared two equally broken paths and reported them identical. | Define script_dir before sourcing the library, and require each differential case to prove the invocation had an effect. | resolved | N/A |
|---|---|---|---|---|
| AR-02 | Hoisting the flag in plan-context.sh broke it: that script consumes --plan-dir natively. | Exclude the two scripts that already handle the flag, and cover both in the differential test. | resolved | N/A |
| AR-03 | lib-test.sh ran set -euo pipefail at load, so sourcing it forced errexit on its caller and aborted a test that deliberately runs without it. | A sourced library must not change its caller shell options. | resolved | N/A |

## Verdict

- Status: `✅ approved`
- Rationale: Every finding was found during implementation, is fixed, and is covered by an assertion. Both goals are proven against a baseline captured before any change, on bash 5.3 and on real bash 3.2.57.
