# Work-unit inventory: basic-test-proof-current-20260811t114255z-current-fresh4-isolated-plan-tmp

## Definition-of-done coverage

| Required outcome or proof | Work unit IDs | Notes |
|---|---|---|

| button-chain.html is the only planned implementation file | W01,W02,W03,W04 | All markup, behavior, and style targets are scoped to button-chain.html; W05 proves it. |

| one initial button exists before interaction | W01,W05 | Markup creates the initial button and the future story verifies the starting state. |

| pressing only the current last button appends exactly one button below it | W02,W05 | The handler owns the last-button guard and one-append rule; the story verifies each generated state. |

| pressing the fourth generated button clears the document | W03,W05 | The completion branch owns the destructive transition; the story verifies prior buttons are gone. |

| completion prints exact lowercase finished with visible white border | W03,W04,W05 | Completion text and styling are implemented separately and verified together. |

## Work units

| ID | Type | File | Primary symbol or file scope | Subscope | Intended change | Depends on | Goal | Step |
|---|---|---|---|---|---|---|---|---|

| W01 | markup | `button-chain.html` | `#button-chain-root` | `N/A` | Create the document body structure with one initial button and a stable container for generated buttons. | — | 01-button-chain | 01-step-markup |

| W02 | source | `button-chain.html` | `appendNextButton()` | `last-button click path` | Add the click handling that only responds when the clicked control is the current last button and appends exactly one new button below it. | W01 | 01-button-chain | 02-step-append-handler |

| W03 | source | `button-chain.html` | `completeChain()` | `fourth-generated-button branch` | Add the completion branch for the fourth generated button that clears the document and renders exactly finished. | W02 | 01-button-chain | 03-step-completion-branch |

| W04 | style | `button-chain.html` | `.completion-message` | `N/A` | Style the completion state so the lowercase finished text has a visible white border. | W03 | 01-button-chain | 04-step-completion-border |

| W05 | verification | `N/A` | `US-01 browser flow` | `N/A` | Future browser verification: click the current last button to create generated buttons one through four, then click the fourth generated button and confirm document clearing, exact finished text, and visible white border. | W01,W02,W03,W04 | 01-button-chain | 05-step-browser-verification |

## Decomposition review

- [x] Every definition-of-done item maps to one or more work units.
- [x] Every known affected file and changing symbol has its own work unit.
- [x] Every work unit has exactly one goal and one step.
- [x] Each goal has 2–10 work units, or records an allowed exception.
- [x] Each step has exactly one work unit and no unnamed incidental edits.
- [x] Dependencies form an executable order with no cycle.
