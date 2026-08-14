# Work-unit inventory: basic-test-proof-current-20260811t200821z-clean-current-iterative-isolated-plan

## Definition-of-done coverage

| Required outcome or proof | Work unit IDs | Notes |
|---|---|---|

| Initial button exists in button-chain.html | W01 | The markup work unit owns the initial DOM subtree. |

| Pressing the current last button appends exactly one button below it | W02,W04,W06 | The handler, automated test design, and browser story cover the append contract. |

| Pressing the fourth generated button clears the document | W03,W04,W06 | The completion branch and both proof units cover the destructive terminal state. |

| Completion prints exact lowercase finished with a visible white border | W03,W05,W04,W06 | The source branch owns exact text, the style unit owns the white border, and proof units verify both. |

## Work units

| ID | Type | File | Primary symbol or file scope | Subscope | Intended change | Depends on | Goal | Step |
|---|---|---|---|---|---|---|---|---|

| W01 | markup | `button-chain.html` | `#button-chain-root` | `N/A` | Create the initial document subtree with one initial button and a container for appended buttons. | none | 01-button-chain-behavior | 01-step-initial-markup |

| W02 | source | `button-chain.html` | `appendNextButton` | `N/A` | Add the click handler behavior that appends exactly one button below the current last button only when that current last button is pressed. | W01 | 01-button-chain-behavior | 02-step-append-handler |

| W03 | source | `button-chain.html` | `finishOnFourthGeneratedButton` | `N/A` | Add the branch that clears the document when the fourth generated button is pressed and prints exactly finished. | W02 | 01-button-chain-behavior | 03-step-fourth-generated-finish |

| W04 | test | `button-chain.html` | `buttonChainBehaviorTest` | `N/A` | Define an automated test target that exercises initial state, exact single append per current-last click, ignored non-last clicks, and fourth-generated completion text. | W03 | 01-button-chain-behavior | 04-step-behavior-test |

| W05 | style | `button-chain.html` | `.finished-message` | `N/A` | Style the finished completion state so the exact text finished has a visible white border. | W03 | 02-completion-and-ui-proof | 01-step-finished-border-style |

| W06 | verification | `N/A` | `US-01` | `N/A` | Run the future browser user story by clicking the current last button through the fourth generated button and confirming finished with a visible white border. | W01,W02,W03,W05 | 02-completion-and-ui-proof | 02-step-ui-story-verification |

## Decomposition review

- [x] Every definition-of-done item maps to one or more work units.
- [x] Every known affected file and changing symbol has its own work unit.
- [x] Every work unit has exactly one goal and one step.
- [x] Each goal has 2–10 work units, or records an allowed exception.
- [x] Each step has exactly one work unit and no unnamed incidental edits.
- [x] Dependencies form an executable order with no cycle.
