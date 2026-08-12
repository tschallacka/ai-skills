# Work-unit inventory: basic-test-proof-current-20260811t112337z-current-fresh3-isolated-plan

## Definition-of-done coverage

| Required outcome or proof | Work unit IDs | Notes |
|---|---|---|

| Create button-chain.html with exactly one initial button in the initial state. | W01 | The markup work unit owns the initial visible DOM contract. |

| Pressing the current last button appends exactly one button below it. | W02,W03 | W02 owns the append function and W03 owns event delegation so only the current last button appends. |




| The completion branch prints exact lowercase finished, and the style unit supplies a visible white border. | W04,W05 | W04 owns semantic completion text and W05 owns border presentation. |

| Earlier non-last buttons are inert after later buttons exist. | W08 | W08 is a mandatory direct-click negative browser story for the last-button guard. |

| Future verification proves the complete UI path, negative last-button behavior, and static acceptance contract without planning-proof HTML execution. | W06,W07,W08 | W06 covers the direct-click completion story, W08 covers the non-last negative story, and W07 covers the static audit after browser verification. |

## Work units

| ID | Type | File | Primary symbol or file scope | Subscope | Intended change | Depends on | Goal | Step |
|---|---|---|---|---|---|---|---|---|

| W01 | markup | `button-chain.html` | `#button-chain-root` | `N/A` | Create the standalone document structure with one initial button and no generated buttons in the initial DOM. | — | 01-build-button-chain | 01-step-initial-markup |

| W02 | source | `button-chain.html` | `appendGeneratedButton()` | `N/A` | Append exactly one new generated button immediately below the current last button each time the append path runs. | W01 | 01-build-button-chain | 02-step-append-function |

| W03 | source | `button-chain.html` | `handleButtonClick()` | `last-button guard` | Ignore non-last buttons and route only the current last button click to append or completion behavior. | W02 | 01-build-button-chain | 03-step-last-button-handler |

| W04 | source | `button-chain.html` | `handleButtonClick()` | `fourth-generated completion branch` | When the clicked current last button is the fourth generated button, clear the document body instead of appending another button. | W03 | 01-build-button-chain | 04-step-fourth-generated-finish |




| W06 | verification | `N/A` | `US-01 browser flow` | `N/A` | Using direct clicks, verify the initial button, four generated-button progression, no extra append on completion, document clear, exact finished text, and visible white border. | W01,W02,W03,W04,W05 | 01-build-button-chain | 06-step-browser-story |

| W05 | style | `button-chain.html` | `.completion-state` | `N/A` | Style the completion state with a visible white border while preserving the exact text supplied by the completion branch. | W04 | 01-build-button-chain | 05-step-completion-style |

| W07 | verification | `N/A` | `static acceptance audit` | `N/A` | Inspect the implemented file after browser verification to confirm no extra files are required and the acceptance-critical strings, handlers, and bordered completion selector are present. | W01,W02,W03,W04,W05,W06 | 02-verify-button-chain | 02-step-static-audit |

| W08 | verification | `N/A` | `US-02 non-last inert browser flow` | `N/A` | Using direct clicks, verify that clicking an earlier non-last button after generated buttons exist does not append a button, clear the document, or trigger finished. | W01,W02,W03 | 01-build-button-chain | 07-step-non-last-inert-story |

## Decomposition review

- [x] Every definition-of-done item maps to one or more work units.
- [x] Every known affected file and changing symbol has its own work unit.
- [x] Every work unit has exactly one goal and one step.
- [x] Each goal has 2–10 work units, or records an allowed exception.
- [x] Each step has exactly one work unit and no unnamed incidental edits.
- [x] Dependencies form an executable order with no cycle.
