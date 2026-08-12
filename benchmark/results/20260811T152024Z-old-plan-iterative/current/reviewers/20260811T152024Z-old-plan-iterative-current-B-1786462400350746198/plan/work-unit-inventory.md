# Work-unit inventory: basic-test-proof-current-20260811t152024z-old-plan-iterative-isolated-plan

## Definition-of-done coverage

| Required outcome or proof | Work unit IDs | Notes |
|---|---|---|

| button-chain.html creation is planned without creating it during the proof | W01,W02,W03,W04 | The future file is represented by separate markup, behavior, and style work units. |

| current last button appends exactly one button below it | W02,W05 | The handler work unit defines the state transition and the browser story verifies it by repeated clicks. |

| fourth generated button clears the document | W03,W05 | The completion branch work unit defines generated-button counting and destructive replacement; the browser story verifies the result. |

| finished text is exact lowercase with visible white border | W04,W05 | The style work unit owns the visible border and the browser story verifies the observable completion state. |

| implementation goal has a local verification gate before UI story execution | W06 | W06 verifies that W01 through W04 are complete before the browser story is run. |

## Work units

| ID | Type | File | Primary symbol or file scope | Subscope | Intended change | Depends on | Goal | Step |
|---|---|---|---|---|---|---|---|---|

| W01 | markup | `button-chain.html` | `#button-chain-root` | `N/A` | Create the minimal HTML document body with exactly one initial button and no pre-rendered generated buttons. | — | 01-create-button-chain | 01-step-initial-markup |

| W02 | source | `button-chain.html` | `button-chain script` | `append-last-button handler` | Attach click handling so pressing only the current last button appends exactly one new button below it. | W01 | 01-create-button-chain | 02-step-append-handler |

| W03 | source | `button-chain.html` | `button-chain script` | `fourth-generated completion branch` | When the fourth generated button is pressed, clear the document and render only the completion state. | W02 | 01-create-button-chain | 03-step-completion-branch |

| W04 | style | `button-chain.html` | `.completion-message` | `N/A` | Style the completion state so the exact lowercase text finished has a visible white border. | W03 | 01-create-button-chain | 04-step-completion-style |

| W05 | verification | `N/A` | `Browser story US-01` | `N/A` | After implementation, click through the button chain and verify the fourth generated button clears the document and shows finished with a visible white border. | W01,W02,W03,W04 | 02-verify-and-handoff | 01-step-browser-story |

| W06 | verification | `N/A` | `Implementation review proof` | `N/A` | Review the completed button-chain.html against W01 through W04 before running browser story US-01. | W01,W02,W03,W04 | 01-create-button-chain | 05-step-implementation-review |

## Decomposition review

- [x] Every definition-of-done item maps to one or more work units.
- [x] Every known affected file and changing symbol has its own work unit.
- [x] Every work unit has exactly one goal and one step.
- [x] Each goal has 2–10 work units, or records an allowed exception.
- [x] Each step has exactly one work unit and no unnamed incidental edits.
- [x] Dependencies form an executable order with no cycle.
