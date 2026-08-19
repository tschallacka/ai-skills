# Progress: 01-plan-dir-synonym

**Progress:** `100%  ####################  100%` ✅

| Goalname | Stepname | Description | Completion status |
|---|---|---|---|
| 01-plan-dir-synonym | 01-step-confirm-hoister | No change; the hoister already exists and is the seam the other units use. | ✅ completed |
| 01-plan-dir-synonym | 02-step-hoist-simple-helpers | Source plan-document-lib.sh above first use of $1, then hoist --plan-dir to position 1. | ✅ completed |
| 01-plan-dir-synonym | 03-step-hoist-subcommand-helper | Hoist at position 1 after command="$1"; shift, so every subcommand sees the plan directory positiona... | ✅ completed |
| 01-plan-dir-synonym | 04-step-prove-equivalence | One case per converted helper: positional and --plan-dir produce identical trees, output and exit st... | ✅ completed |
