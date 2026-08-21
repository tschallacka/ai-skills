
## Cycle 1

| ID | Missing or over-broad item | Required plan change | Status | Work unit |
|---|---|---|---|---|
| AR-01 | No finding recorded yet. | N/A | ✅ resolved | N/A |
| AR-02 | Hoisting the flag in plan-context.sh broke it: that script consumes --plan-dir natively, so moving the value to a positional slot left its parser without the flag it requires. | Exclude the two scripts that already handle the flag, and cover both in the differential test. | ✅ resolved | N/A |
| AR-03 | lib-test.sh ran set -euo pipefail at load, so sourcing it forced errexit on its caller and aborted test-plan-context-paging.sh, which deliberately runs without it. | A sourced library must not change its caller shell options; remove the set line and record why. | ✅ resolved | N/A |

## Cycle 2

| ID | Missing or over-broad item | Required plan change | Status | Work unit |
|---|---|---|---|---|
| AR-01 | No finding recorded yet. | N/A | ✅ resolved | N/A |
