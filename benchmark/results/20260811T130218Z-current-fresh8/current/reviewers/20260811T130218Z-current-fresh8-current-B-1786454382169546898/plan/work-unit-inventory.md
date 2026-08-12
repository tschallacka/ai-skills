# Work-unit inventory: basic-test-proof-current-20260811t130218z-current-fresh8-isolated-plan

## Definition-of-done coverage

| Required outcome or proof | Work unit IDs | Notes |
|---|---|---|

| button-chain.html exists as the sole future implementation file with one initial button | W01 | The markup unit creates the standalone document and initial visible button. |

| pressing the current last button appends exactly one button below it | W03,W05,W06 | Behavior implementation, automated check, and browser story cover the append contract. |

| pressing the fourth generated button clears the document | W04,W05,W06 | Completion behavior is implemented and then verified by test and browser story. |

| completion prints exact lowercase finished with a visible white border | W02,W04,W05,W06 | Style, completion behavior, automated check, and browser story cover the final visual state. |

| planning-only benchmark safety boundary is preserved | W06 | The plan records future browser verification but this run does not create or execute HTML. |

## Work units

| ID | Type | File | Primary symbol or file scope | Subscope | Intended change | Depends on | Goal | Step |
|---|---|---|---|---|---|---|---|---|

| W01 | markup | button-chain.html | #button-chain-root | `N/A` | Create a valid standalone HTML document containing exactly one initial button in the body and a container/order that supports generated buttons below it. | — | 01-button-chain-implementation | 01-step-document-markup |

| W02 | style | `button-chain.html` | `.completion-message` | `N/A` | Define the visible completion presentation for the exact text finished, including a visible white border against a contrasting completion state so the border is visibly apparent. | W01 | 01-button-chain-implementation | 02-step-completion-style |

| W03 | source | `button-chain.html` | `appendButtonAfterLastClick` | `N/A` | Handle clicks so only the current last button appends exactly one new button directly below it. | W01 | 01-button-chain-implementation | 03-step-append-last-button |

| W04 | source | `button-chain.html` | `finishOnFourthGeneratedButton` | `N/A` | When the fourth appended/generated button is clicked as the current last button, clear the document and render only the completion state with exact lowercase text finished. | W02,W03 | 01-button-chain-implementation | 04-step-fourth-generated-finish |

| W05 | test | `button-chain.behavior.test.mjs` | `button chain behavior test` | `N/A` | Add the focused node button-chain.behavior.test.mjs automated test using built-in Node.js modules only to prove initial state, exact one-button append per last-button click, ignored non-last clicks, fourth-generated completion, exact text, visible white border, and border contrast. | W01,W02,W03,W04 | 01-button-chain-implementation | 05-step-behavior-test |

| W06 | verification | `N/A` | `US-01 browser click flow` | `N/A` | Open the implemented file in a browser and use direct clicks on visible buttons from the initial state through the fourth generated button, confirming completion evidence and no unresolved bug rows. | W05 | 02-browser-story-validation | 01-step-browser-story-us-01 |

## Decomposition review

- [x] Every definition-of-done item maps to one or more work units.
- [x] Every known affected file and changing symbol has its own work unit.
- [x] Every work unit has exactly one goal and one step.
- [x] Each goal has 2–10 work units, or records an allowed exception.
- [x] Each step has exactly one work unit and no unnamed incidental edits.
- [x] Dependencies form an executable order with no cycle.
