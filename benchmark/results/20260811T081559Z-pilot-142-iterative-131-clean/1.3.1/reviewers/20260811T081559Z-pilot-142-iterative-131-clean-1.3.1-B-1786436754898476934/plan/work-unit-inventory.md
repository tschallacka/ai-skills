# Work-unit inventory: basic-test-proof-1-3-1-20260811t081559z-pilot-142-iterative-131-clean-isolated-plan

## Definition-of-done coverage

| Required outcome or proof | Work unit IDs | Notes |
|---|---|---|

| button-chain.html exists with one initial button | W01 | The #button-chain-app markup work unit defines the future single initial button. |

| pressing the current last button appends exactly one button below it | W03,W05 | The click-handler work unit defines last-button-only append behavior and the UI story verifies the exact interaction. |

| pressing the fourth generated button clears the document | W04,W05 | The completion renderer work unit clears the document, and the UI story reaches the fourth generated button by direct clicks. |

| completion prints exact lowercase finished with a visible white border | W02,W04,W05 | The style and renderer work units define the message and visible border, and the UI story verifies the result. |

## Work units

| ID | Type | File | Primary symbol or file scope | Subscope | Intended change | Depends on | Goal | Step |
|---|---|---|---|---|---|---|---|---|

| W01 | markup | `button-chain.html` | `#button-chain-app` | `N/A` | Create the document subtree with one initial button and no generated buttons present at load. | — | 01-button-chain-contract | 01-step-initial-markup |

| W02 | style | `button-chain.html` | `.completion-message` | `N/A` | Define the completion message presentation with a visible white border. | W01 | 01-button-chain-contract | 02-step-completion-style |

| W03 | source | `button-chain.html` | `handleButtonClick(event)` | `N/A` | Append exactly one new button below the current last button when that current last button is pressed, ignoring older buttons for append behavior. | W01 | 01-button-chain-contract | 03-step-last-button-click |

| W04 | source | `button-chain.html` | `renderFinishedState()` | `N/A` | When the fourth generated button is pressed, clear the document and render only the exact lowercase text finished using the completion-message styling. | W02,W03 | 01-button-chain-contract | 04-step-finished-state |

| W05 | verification | `N/A` | `US-01 browser click flow` | `N/A` | Open the implemented local file and click the initial button plus generated buttons until the fourth generated button proves the finished state with visible white border. | W01,W02,W03,W04 | 02-ui-story-verification | 01-step-ui-story-us-01 |

## Decomposition review

- [x] Every definition-of-done item maps to one or more work units.
- [x] Every known affected file and changing symbol has its own work unit.
- [x] Every work unit has exactly one goal and one step.
- [x] Each goal has 2–10 work units, or records an allowed exception.
- [x] Each step has exactly one work unit and no unnamed incidental edits.
- [x] Dependencies form an executable order with no cycle.
