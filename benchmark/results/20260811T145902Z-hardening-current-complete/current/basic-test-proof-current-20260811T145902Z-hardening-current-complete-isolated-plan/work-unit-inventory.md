# Work-unit inventory: basic-test-proof-current-20260811T145902Z-hardening-current-complete-isolated-plan

## Definition-of-done coverage

| Required outcome or proof | Work unit IDs | Notes |
|---|---|---|

| Initial document has exactly one button | W01 | W01 owns the initial markup contract. |

| Clicking the current last button appends exactly one button below it | W03,W04,W05 | W03 implements the behavior; W04 and W05 prove it by automated and browser checks. |

| Clicking the fourth generated button clears the document | W03,W04,W05 | W03 owns the fourth-generated-button branch; W04 and W05 verify the terminal clearing. |

| Completion state prints exact lowercase finished with a visible white border | W02,W03,W04,W05 | W02 owns border styling, W03 renders the text, and W04/W05 verify the result. |

| Build goal has an owned verification unit | W06 | W06 gives the implementation goal its own proof target without replacing the final DOM and browser checks. |

## Work units

| ID | Type | File | Primary symbol or file scope | Subscope | Intended change | Depends on | Goal | Step |
|---|---|---|---|---|---|---|---|---|

| W01 | markup | `button-chain.html` | `#button-chain-root` | `N/A` | Create the document body root containing exactly one initial button and no pre-rendered generated buttons. | — | 01-build-button-chain | 01-step-markup-root |

| W02 | style | `button-chain.html` | `.completion-message` | `N/A` | Define completion-state styling with a visible white border around the exact finished message. | W01 | 01-build-button-chain | 02-step-completion-style |

| W03 | source | `button-chain.html` | `appendNextButton` | `N/A` | Implement click behavior so only the current last button appends exactly one next button, and the fourth generated button clears the document and renders finished. | W01,W02 | 01-build-button-chain | 03-step-append-behavior |

| W06 | verification | `N/A` | `Build-goal implementation review` | `N/A` | Review the completed future button-chain.html source and rendered initial state to confirm the markup, style, and append behavior are present before final validation. | W01,W02,W03 | 01-build-button-chain | 04-step-build-review |

| W04 | verification | `N/A` | `DOM behavior command` | `N/A` | Run a bounded automated verification that loads button-chain.html, performs click-equivalent checks through the DOM event path, and asserts append count, clearing, exact text, and white border style. | W01,W02,W03 | 02-verify-button-chain | 01-step-dom-verification |

| W05 | verification | `N/A` | `US-01 browser flow` | `N/A` | Run the browser user story by clicking the current last button until the fourth generated button triggers the finished state. | W01,W02,W03,W04 | 02-verify-button-chain | 02-step-browser-story |

## Decomposition review

- [x] Every definition-of-done item maps to one or more work units.
- [x] Every known affected file and changing symbol has its own work unit.
- [x] Every work unit has exactly one goal and one step.
- [x] Each goal has 2–10 work units, or records an allowed exception.
- [x] Each step has exactly one work unit and no unnamed incidental edits.
- [x] Dependencies form an executable order with no cycle.
