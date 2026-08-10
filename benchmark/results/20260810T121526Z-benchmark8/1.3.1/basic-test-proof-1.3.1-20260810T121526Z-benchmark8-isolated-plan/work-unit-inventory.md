# Work-unit inventory: basic-test-proof-1-3-1-20260810t121526z-benchmark8-isolated-plan

## Definition-of-done coverage

| Required outcome or proof | Work unit IDs | Notes |
|---|---|---|

| Initial document has exactly one button | W01 | W01 owns the initial body subtree and initial control contract. |

| Each current-last-button activation appends exactly one button below it | W01,W02 | W01 supplies the initial target; W02 owns the append behavior. |

| Pressing generated button 4 clears the document | W02,W03 | W02 creates generated buttons 1–4; W03 owns pressing generated button 4 on click five. |

| Completion prints exact lowercase finished with visible white border | W03,W04 | W03 emits the message; W04 owns its visible border styling. |

| Browser acceptance proof and handoff | W05 | W05 is the independent bounded UI verification flow. |

## Work units

| ID | Type | File | Primary symbol or file scope | Subscope | Intended change | Depends on | Goal | Step |
|---|---|---|---|---|---|---|---|---|

| W01 | markup | `button-chain.html` | `#button-chain` | `N/A` | Define the document shell and one initial button as the only initial interactive control. | — | 01-button-chain-implementation | 01-step-initial-markup |

| W02 | source | `button-chain.html` | `appendButton()` | `generated-button creation branch` | Append exactly one new button immediately below the current last button for four sequential clicks, label generated buttons 1–4, and preserve the current-last target. | W01 | 01-button-chain-implementation | 02-step-append-button |

| W03 | source | `button-chain.html` | `handleButtonActivation()` | `generated-button-4 completion branch` | Clear the document when generated button 4 is pressed on click five and emit the completion message contract. | W01,W02 | 01-button-chain-implementation | 03-step-completion-branch |

| W04 | style | `button-chain.html` | `.completion-message` | `N/A` | Define a visible white border around the exact lowercase finished completion text. | W03 | 01-button-chain-implementation | 04-step-completion-style |

| W05 | verification | `N/A` | `US-01 button-chain browser flow` | `N/A` | Run five rendered-button mouse clicks: four one-at-a-time appends followed by pressing generated button 4, then verify document clearing, exact finished text, and visible white border. | W01,W02,W03,W04 | 02-button-chain-ui-validation | 01-step-us-01-validation |

## Decomposition review

- [x] Every definition-of-done item maps to one or more work units.
- [x] Every known affected file and changing symbol has its own work unit.
- [x] Every work unit has exactly one goal and one step.
- [x] Each goal has 2–10 work units, or records an allowed exception.
- [x] Each step has exactly one work unit and no unnamed incidental edits.
- [x] Dependencies form an executable order with no cycle.
