# Work-unit inventory: basic-test-proof-1-3-0-plan

## Definition-of-done coverage

| Required outcome or proof | Work unit IDs | Notes |
|---|---|---|

| Future HTML file and initial button-chain markup are specified exactly | W01 | The future implementation target is button-chain.html and its named #button-chain subtree; this proof creates neither. |

| Button-chain append and fourth-generated-button completion behavior are specified | W02 | The behavior contract includes one-button-per-current-last-button press, clear, finished, and white border. |

| Rendered UI interaction proof is defined and bounded | W03 | The future executor must use real browser clicks; planning-time browser execution is explicitly excluded by the request. |

| Plan is resumable, reviewable, validated, and handed off | W04,W05 | UI story, cache, review, validator, no-artifact check, progress, and handoff evidence are required. |

| Proof execution evidence is durably reported | W06 | The analysis report preserves exact timing, status, review result, validator result, safety-boundary result, and token evidence status. |

| Future implementation contract has an independent semantic proof target | W07 | This goal-level proof catches counting and completion-state ambiguity before any future browser run; it is not executed during this planning-only proof. |

## Work units

| ID | Type | File | Primary symbol or file scope | Subscope | Intended change | Depends on | Goal | Step |
|---|---|---|---|---|---|---|---|---|

| W01 | markup | `button-chain.html` | `#button-chain` | `N/A` | Create the future root HTML button-chain subtree with one initial button and the required white-border completion output boundary. | — | 01-button-chain-contract | 01-step-markup |

| W02 | source | `button-chain.html` | `appendButtonChain()` | `current-last-button callback` | Implement the future button-chain behavior: each current last-button press appends exactly one button below it, and the fourth generated-button press clears the document and prints finished with a white border. | W01 | 01-button-chain-contract | 02-step-behavior |

| W03 | verification | `N/A` | `browser button-chain flow` | `N/A` | Verify the future rendered flow by clicking the initial/current last button through the fourth generated button and checking the final finished output and white border. | W02 | 02-plan-proof-and-handoff | 01-step-browser-proof |

| W04 | docs | `ui-user-stories.md` | `US-01` | `N/A` | Record the direct-interaction UI acceptance story, explicit planning-only exclusion, cache, evidence boundary, and related verification unit. | W03 | 02-plan-proof-and-handoff | 02-step-story |

| W05 | verification | `N/A` | `validate-plan.sh` | `N/A` | Run the historical structural plan validator after review approval and confirm the plan contains no HTML, browser, server, or implementation artifact. | W04 | 02-plan-proof-and-handoff | 03-step-validator |

| W06 | docs | `1.3.1-analyze.md` | `execution report` | `N/A` | Record revision, timestamps, elapsed seconds, worker result, validator/review findings, no-artifact/process result, and token-cost evidence or explicit unavailability. | W05 | 02-plan-proof-and-handoff | 04-step-report |

| W07 | verification | `N/A` | `future button-chain contract review` | `N/A` | Review the future button-chain implementation contract for exact initial-button, one-append, fourth-generated-button, clear, finished, and white-border semantics before browser execution. | W02 | 01-button-chain-contract | 03-step-contract-proof |

## Decomposition review

- [x] Every definition-of-done item maps to one or more work units.
- [x] Every known affected file and changing symbol has its own work unit.
- [x] Every work unit has exactly one goal and one step.
- [x] Each goal has 2–10 work units, or records an allowed exception.
- [x] Each step has exactly one work unit and no unnamed incidental edits.
- [x] Dependencies form an executable order with no cycle.
