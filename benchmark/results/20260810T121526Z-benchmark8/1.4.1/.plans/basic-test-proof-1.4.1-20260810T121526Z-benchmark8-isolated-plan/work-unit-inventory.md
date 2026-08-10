# Work-unit inventory: basic-test-proof-1-4-1-20260810t121526z-benchmark8-isolated-plan

## Definition-of-done coverage

| Required outcome or proof | Work unit IDs | Notes |
|---|---|---|

| Single initial button and deterministic button-chain behavior | W01,W02 | Markup establishes one initial control; append logic adds exactly one below the current last button. |

| Fourth generated button terminal state | W03,W04 | Terminal branch clears the document and the completion style makes finished visibly bordered. |

| Executable acceptance proof | W05 | One bounded direct-input UI flow covers the full future behavior. |

## Work units

| ID | Type | File | Primary symbol or file scope | Subscope | Intended change | Depends on | Goal | Step |
|---|---|---|---|---|---|---|---|---|

| W01 | markup | `button-chain.html` | `#button-chain` | `N/A` | Create the initial document structure containing exactly one initial button and the container needed for the interaction. | — | 01-button-chain-plan | 01-step-initial-markup |

| W02 | source | `button-chain.html` | `appendButton()` | `click callback` | Append exactly one button after the current last button, preserving vertical DOM order. | W01 | 01-button-chain-plan | 02-step-append-button |

| W03 | source | `button-chain.html` | `handleButtonClick()` | `fourth-generated-button branch` | Count generated-button activations and clear the document on the fourth generated button. | W02 | 01-button-chain-plan | 03-step-fourth-button-terminal |

| W04 | style | `button-chain.html` | `.completion-message` | `N/A` | Give the finished text a visible white border while preserving the exact lowercase text. | W03 | 01-button-chain-plan | 04-step-completion-style |

| W05 | verification | `N/A` | `US-01 button-chain browser flow` | `N/A` | Verify initial count, one-at-a-time append behavior, fourth-button clearing, exact finished text, and visible white border through direct UI input. | W04 | 01-button-chain-plan | 05-step-ui-verification |

## Decomposition review

- [x] Every definition-of-done item maps to one or more work units.
- [x] Every known affected file and changing symbol has its own work unit.
- [x] Every work unit has exactly one goal and one step.
- [x] Each goal has 2–10 work units, or records an allowed exception.
- [x] Each step has exactly one work unit and no unnamed incidental edits.
- [x] Dependencies form an executable order with no cycle.
