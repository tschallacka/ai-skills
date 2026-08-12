# Work-unit inventory: basic-test-proof-v1-4-1-20260811t061045z-pilot-142-matrix-fresh-isolated-plan

## Definition-of-done coverage

| Required outcome or proof | Work unit IDs | Notes |
|---|---|---|

| button-chain.html exists with one initial button | W01 | W01 owns the document root and initial button contract. |

| pressing the current last button appends exactly one button below it | W02 | W02 owns the last-button-only click handler and append behavior. |

| pressing the fourth generated button clears the document and prints finished with visible white border | W03,W04,W07 | W03 owns the white-border presentation, W04 owns terminal rendering, and W07 owns the fourth-generated handler dispatch. |

| future UI acceptance is verified through direct browser clicks | W05 | W05 owns the five-click browser-story proof without running it during this planning-only benchmark. |

| implementation goal has its own verification unit before final UI story | W06 | W06 proves W01-W04 as an integrated implementation contract before W05 runs the browser story. |

| handler owns the fourth-generated terminal dispatch without widening renderFinishedState() | W07 | W07 owns the handleButtonClick(event) terminal branch; W04 stays limited to rendering the finished state. |

## Work units

| ID | Type | File | Primary symbol or file scope | Subscope | Intended change | Depends on | Goal | Step |
|---|---|---|---|---|---|---|---|---|

| W01 | markup | `button-chain.html` | `#button-chain-root` | `N/A` | Create the document body with exactly one initial button and a root container for the button chain. | — | 01-button-chain-contract | 01-step-initial-button-markup |

| W02 | source | `button-chain.html` | `handleButtonClick(event)` | `non-terminal append branch` | Add click handling so only pressing the current last button appends exactly one new button below it before the fourth-generated terminal branch applies. | W01 | 01-button-chain-contract | 02-step-last-button-append-handler |

| W03 | style | `button-chain.html` | `.completion-message` | `N/A` | Define the completion presentation so the final lowercase text finished is visible with a white border. | W01 | 01-button-chain-contract | 03-step-finished-border-style |

| W04 | source | `button-chain.html` | `renderFinishedState()` | `N/A` | Clear the document and render only the exact lowercase text finished using the completion style when called by the handler terminal branch. | W02,W03 | 01-button-chain-contract | 04-step-fourth-generated-terminal-state |

| W05 | verification | `N/A` | `Browser story US-01` | `N/A` | Future browser verification performs five direct current-last-button clicks: initial, generated 1, generated 2, generated 3, and generated 4, confirming four append-producing clicks and the terminal finished state after pressing generated 4. | W01,W02,W03,W04,W07 | 02-ui-story-verification | 01-step-browser-story-us-01 |

| W06 | verification | `N/A` | `Implementation contract checklist` | `N/A` | Future executor verifies W01-W04 and W07 together before UI story execution: one initial button, one append per current-last click until generated 4 exists, terminal fourth-generated click, and finished with visible white border. | W01,W02,W03,W04,W07 | 01-button-chain-contract | 05-step-implementation-contract-check |

| W07 | source | `button-chain.html` | `handleButtonClick(event)` | `fourth-generated terminal branch` | Add the handler branch that recognizes a click on the fourth generated current-last button and delegates to renderFinishedState() instead of appending another button. | W02,W04 | 01-button-chain-contract | 06-step-fourth-generated-handler-branch |

## Decomposition review

- [x] Every definition-of-done item maps to one or more work units.
- [x] Every known affected file and changing symbol has its own work unit.
- [x] Every work unit has exactly one goal and one step.
- [x] Each goal has 2–10 work units, or records an allowed exception.
- [x] Each step has exactly one work unit and no unnamed incidental edits.
- [x] Dependencies form an executable order with no cycle.
