# Work-unit inventory: harden-test-reporting-and-plan-dir

## Definition-of-done coverage

| Required outcome or proof | Work unit IDs | Notes |
|---|---|---|

| Every helper taking a plan directory accepts --plan-dir, byte-identical to the positional form | W01,W02,W03,W04 | Proven by a differential case per helper in test-flag-form-equivalence.sh. |

| A finding raised inside a command substitution is not discarded | W05,W06 | The shared reporter records findings in a file rather than a counter variable. |

| No test changes behaviour on the passing path | W07 | Every test stdout, stderr and exit status matches the captured baseline. |

| A converted test still reports and still fails when an assertion breaks | W08 | Mutation spot-check across a sample of converted tests. |

## Work units

| ID | Type | File | Primary symbol or file scope | Subscope | Intended change | Depends on | Goal | Step |
|---|---|---|---|---|---|---|---|---|

| W01 | source | `planning/scripts/plan-document-lib.sh` | `plan_hoist_plan_dir` | `N/A` | No change; the hoister already exists and is the seam the other units use. | -- | 01-plan-dir-synonym | 01-step-confirm-hoister |

| W02 | source | `planning/scripts/add-goal.sh` | `argument parsing` | `N/A` | Source plan-document-lib.sh above first use of $1, then hoist --plan-dir to position 1. | W01 | 01-plan-dir-synonym | 02-step-hoist-simple-helpers |

| W03 | source | `planning/scripts/update-plan-content.sh` | `argument parsing` | `per-subcommand case` | Hoist at position 1 after command="$1"; shift, so every subcommand sees the plan directory positionally. | W01 | 01-plan-dir-synonym | 03-step-hoist-subcommand-helper |

| W04 | test | `planning/tests/test-flag-form-equivalence.sh` | `differential proof` | `N/A` | One case per converted helper: positional and --plan-dir produce identical trees, output and exit status. | W02,W03 | 01-plan-dir-synonym | 04-step-prove-equivalence |

| W05 | source | `planning/tests/lib-test.sh` | `t_fail and t_end` | `N/A` | No change; the shared reporter already exists and is the seam the conversion targets. | -- | 02-shared-test-reporting | 01-step-confirm-reporter |

| W06 | test | `planning/tests/test-progress-bar-shape.sh` | `reporter definition` | `note_fail` | Point the local reporter at t_fail and replace the counter epilogue with t_end, leaving every message and call site unchanged. | W05 | 02-shared-test-reporting | 02-step-delegate-reporters |

| W07 | verification | `N/A` | `suite output` | `N/A` | Every test stdout, stderr and exit status is byte-identical to the captured baseline on the passing path. | W06 | 02-shared-test-reporting | 03-step-diff-against-baseline |

| W08 | verification | `N/A` | `failure reporting` | `N/A` | A planted failure in a converted test still names its finding and still exits non-zero. | W07 | 02-shared-test-reporting | 04-step-mutation-spotcheck |

## Decomposition review

- [x] Every definition-of-done item maps to one or more work units.
- [x] Every known affected file and changing symbol has its own work unit.
- [x] Every work unit has exactly one goal and one step.
- [x] Each goal has 2–10 work units, or records an allowed exception.
- [x] Each step has exactly one work unit and no unnamed incidental edits.
- [x] Dependencies form an executable order with no cycle.
