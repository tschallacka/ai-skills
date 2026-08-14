# Work-unit inventory: basic-test-proof-1-3-1-20260811t074548z-pilot-142-fresh-131-restart5-isolated-plan

## Definition-of-done coverage

| Required outcome or proof | Work unit IDs | Notes |
|---|---|---|

| button-chain.html is the only future implementation artifact and loads with one initial button | W01 | The markup unit owns the document shell and initial state. |

| pressing the current last button appends exactly one button below it | W02,W06 | The handler unit implements the behavior and the browser story proves it through user clicks. |

| pressing the fourth generated button clears the document | W03,W06 | The completion branch owns the clearing behavior and the browser story covers it. |

| completion state prints exact lowercase finished with a visible white border | W04,W06 | The style unit and browser story cover exact text and border visibility. |

## Work units

| ID | Type | File | Primary symbol or file scope | Subscope | Intended change | Depends on | Goal | Step |
|---|---|---|---|---|---|---|---|---|

| W01 | markup | `button-chain.html` | `#button-chain-root` | `N/A` | Create the document body region with exactly one initial button and no generated buttons at load. | - | 01-button-chain-behavior | 01-step-initial-markup |

| W02 | source | `button-chain.html` | `handleButtonClick(event)` | `append-last-button branch` | Append exactly one new button below the current last button only when that current last button is pressed. | W01 | 01-button-chain-behavior | 02-step-append-handler |

| W03 | source | `button-chain.html` | `handleButtonClick(event)` | `fourth-generated branch` | When the fourth generated button is pressed, clear the document and render only the completion state. | W02 | 01-button-chain-behavior | 03-step-finished-transition |

| W04 | style | `button-chain.html` | `.completion-message` | `N/A` | Style the finished text so the exact lowercase word is visible with a visible white border. | W03 | 01-button-chain-behavior | 04-step-finished-style |

| W06 | verification | `N/A` | `US-01 browser click-chain flow` | `N/A` | Run the direct browser story by clicking through the button chain and observing the finished state. | W01,W02,W03,W04 | 02-acceptance-proof | 01-step-browser-story |

## Decomposition review

- [x] Every definition-of-done item maps to one or more work units.
- [x] Every known affected file and changing symbol has its own work unit.
- [x] Every work unit has exactly one goal and one step.
- [x] Each goal has 2–10 work units, or records an allowed exception.
- [x] Each step has exactly one work unit and no unnamed incidental edits.
- [x] Dependencies form an executable order with no cycle.
