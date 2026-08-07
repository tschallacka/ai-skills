# Work-unit inventory: basic-test-proof-1-4-1-isolated-plan

## Definition-of-done coverage

| Required outcome or proof | Work unit IDs | Notes |
|---|---|---|

| Future HTML file and exactly one initial button are specified | W01 | The future target is repository-root button-chain.html and its #button-chain subtree; this proof creates neither. |

| Current-last-button append and fourth-generated-button completion behavior are specified exactly | W02,W07 | The implementation and contract-review units cover one append below, generated-button counting, document clearing, finished text, and white border. |

| Rendered direct-interaction UI proof is bounded and deferred honestly | W03,W04 | US-01 uses real mouse clicks and records the user-approved planning-only exclusion. |

| Plan is reviewed, validated, tracked, and handed off without executing HTML | W05,W06 | Validation, artifact safety, progress, and the revision report provide durable proof. |

## Work units

| ID | Type | File | Primary symbol or file scope | Subscope | Intended change | Depends on | Goal | Step |
|---|---|---|---|---|---|---|---|---|

| W01 | markup | `button-chain.html` | `#button-chain` | `N/A` | Create the future root HTML button-chain subtree with exactly one initial button. | — | 01-button-chain-contract | 01-step-markup |

| W02 | source | `button-chain.html` | `appendButtonChain()` | `current-last-button callback` | Implement the future button-chain behavior: pressing the current last button appends exactly one button below it, and pressing the fourth generated button clears the document and prints finished with a white border. | W01 | 01-button-chain-contract | 02-step-behavior |

| W07 | verification | `N/A` | `future button-chain contract review` | `N/A` | Review the future button-chain contract for exact initial-button, one-append, fourth-generated-button, clear, finished, and white-border semantics before browser execution. | W02 | 01-button-chain-contract | 03-step-contract-proof |

| W03 | verification | `N/A` | `browser button-chain flow` | `N/A` | Verify the future rendered flow by clicking the initial and current-last buttons through the fourth generated button and checking the final finished output and white border. | W07 | 02-plan-proof-and-handoff | 01-step-browser-proof |

| W04 | docs | `ui-user-stories.md` | `US-01` | `N/A` | Record the direct-interaction UI acceptance story, explicit planning-only exclusion, cache, evidence boundary, and related verification unit. | W03 | 02-plan-proof-and-handoff | 02-step-story |

| W05 | verification | `N/A` | `validate-plan.sh` | `N/A` | Run structural plan validation after review approval and confirm the plan directory contains no HTML or implementation artifact. | W04 | 02-plan-proof-and-handoff | 03-step-validator |

| W06 | docs | `1.4.1-analyze.md` | `execution report` | `N/A` | Record revision, timestamps, worker result, validator and review findings, safety-boundary result, and concise execution handoff. | W05 | 02-plan-proof-and-handoff | 04-step-report |

## Decomposition review

- [x] Every definition-of-done item maps to one or more work units.
- [x] Every known affected file and changing symbol has its own work unit.
- [x] Every work unit has exactly one goal and one step.
- [x] Each goal has 2–10 work units, or records an allowed exception.
- [x] Each step has exactly one work unit and no unnamed incidental edits.
- [x] Dependencies form an executable order with no cycle.
