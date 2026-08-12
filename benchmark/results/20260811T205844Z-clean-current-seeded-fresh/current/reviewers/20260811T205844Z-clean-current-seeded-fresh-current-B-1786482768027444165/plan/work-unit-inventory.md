# Work-unit inventory: basic-test-proof-current-20260811T205844Z-clean-current-seeded-fresh-isolated-plan

## Definition-of-done coverage

| Required outcome or proof | Work unit IDs | Notes |
|---|---|---|

| Initial document contains one button and no premature completion state | W01 | Markup unit owns the load-state contract. |

| Buttons append exactly one below the current last button | W02,W03 | Layout and append handler units together cover visual order and click behavior. |

| Fourth generated button clears the document and prints exact finished text with white border | W04 | Completion handler owns the destructive final state. |

| Future proof includes automated DOM assertions and real browser interaction | W05,W06 | Testing and UI story verification units cover non-browser and browser proof. |

| Goal 01 implementation acceptance is checked before formal proof handoff | W07 | Verification unit satisfies the goal-level testing requirement without running during this planning-only proof. |

## Work units

| ID | Type | File | Primary symbol or file scope | Subscope | Intended change | Depends on | Goal | Step |
|---|---|---|---|---|---|---|---|---|

| W01 | markup | `button-chain.html` | `#button-chain` | `N/A` | Create the initial body structure with exactly one visible initial button and no completion text at load. | — | 01-button-chain-html | 01-step-initial-markup |

| W02 | style | `button-chain.html` | `.chain-button` | `N/A` | Style generated chain buttons as visible block-level controls stacked below the previous button. | W01 | 01-button-chain-html | 02-step-button-layout-style |

| W03 | source | `button-chain.html` | `appendNextButton()` | `N/A` | Implement the click handler so only the current last button appends exactly one new button below it. | W01,W02 | 01-button-chain-html | 03-step-append-handler |

| W04 | source | `button-chain.html` | `finishDocument()` | `N/A` | Implement the fourth generated button completion path so activation clears the document and prints lowercase finished with a visible white border. | W03 | 01-button-chain-html | 04-step-finish-handler |

| W05 | test | `button-chain.test.js` | `button-chain interaction test` | `N/A` | Add an automated DOM interaction test that clicks through the chain and asserts initial, append, fourth-generated clear, exact finished text, and visible white border behavior. | W01,W02,W03,W04 | 02-proof-and-handoff | 01-step-dom-test |

| W06 | verification | `N/A` | `Browser story US-01` | `N/A` | Run the planned user-facing browser story by clicking the current last button until the fourth generated button is pressed and record pass/fail evidence. | W01,W02,W03,W04,W05 | 02-proof-and-handoff | 02-step-browser-story |

| W07 | verification | `N/A` | `Implementation acceptance review` | `N/A` | Review the completed W01-W04 implementation against the exact five-click generated-button sequence before handing off to formal proof. | W04 | 01-button-chain-html | 05-step-implementation-acceptance-review |

## Decomposition review

- [x] Every definition-of-done item maps to one or more work units.
- [x] Every known affected file and changing symbol has its own work unit.
- [x] Every work unit has exactly one goal and one step.
- [x] Each goal has 2–10 work units, or records an allowed exception.
- [x] Each step has exactly one work unit and no unnamed incidental edits.
- [x] Dependencies form an executable order with no cycle.
