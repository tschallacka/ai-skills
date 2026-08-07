# Work-unit inventory: basic-test-proof-1-4-1-isolated-plan-rerun

## Definition-of-done coverage

| Required outcome or proof | Work unit IDs | Notes |
|---|---|---|

| Future HTML file and initial button markup are specified exactly | W01 | The future target is repository-root button-chain.html and its named #button-chain subtree; this proof creates neither. |

| Append and fourth-generated-button completion behavior are specified without counting ambiguity | W02,W07 | The behavior and semantic review cover one append per current-last press, generated-button counting, document clearing, finished text, and white border. |

| Rendered direct-interaction proof is bounded | W03,W04 | US-01 uses real button presses and is explicitly excluded only during this user-authorized planning-only run. |

| Plan is decomposed, reviewed, validated, tracked, and handed off | W05,W06 | Validation, isolated artifact/process audit, context checks, trackers, report, and next-executor instructions are explicit. |

## Work units

| ID | Type | File | Primary symbol or file scope | Subscope | Intended change | Depends on | Goal | Step |
|---|---|---|---|---|---|---|---|---|

| W01 | markup | `button-chain.html` | `#button-chain` | `N/A` | Create the future root HTML button-chain subtree with one initial button and a completion output boundary capable of showing a white border. | — | 01-button-chain-contract | 01-step-markup |

| W02 | source | `button-chain.html` | `appendButtonChain()` | `current-last-button callback` | Implement the future behavior: pressing the current last button appends exactly one button below it; pressing the fourth generated button clears the document and prints finished with a white border. | W01 | 01-button-chain-contract | 02-step-behavior |

| W07 | verification | `N/A` | `future button-chain contract review` | `N/A` | Review the future contract for exact initial-button, one-append, fourth-generated-button, clear-document, finished-text, and white-border semantics before browser execution. | W02 | 01-button-chain-contract | 03-step-contract-proof |

| W03 | verification | `N/A` | `browser button-chain flow` | `N/A` | Verify the future rendered flow by pressing the initial/current last button through the fourth generated button and checking the final finished output and white border. | W02,W07 | 02-plan-proof-and-handoff | 01-step-browser-proof |

| W04 | docs | `ui-user-stories.md` | `US-01` | `N/A` | Record the direct-interaction UI acceptance story, explicit planning-only exclusion, cache, evidence boundary, and related verification unit. | W03 | 02-plan-proof-and-handoff | 02-step-story |

| W05 | verification | `N/A` | `planning readiness and isolation validation` | `N/A` | Run the structural plan validator and bounded planning context checks after review, then confirm the new plan directory contains no HTML and no prohibited browser/server/driver was started by this proof. | W04 | 02-plan-proof-and-handoff | 03-step-validator |

| W06 | docs | `1.4.1-analyze.md` | `execution report` | `N/A` | Record revision, timing, sequential mode, review result and limitation, validator/context results, no-HTML/process result, durable inventory, and concise execution handoff. | W05 | 02-plan-proof-and-handoff | 04-step-report |

## Decomposition review

- [x] Every definition-of-done item maps to one or more work units.
- [x] Every known affected file and changing symbol has its own work unit.
- [x] Every work unit has exactly one goal and one step.
- [x] Each goal has 2–10 work units, or records an allowed exception.
- [x] Each step has exactly one work unit and no unnamed incidental edits.
- [x] Dependencies form an executable order with no cycle.
