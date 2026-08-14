# Work-unit inventory: basic-test-proof-1.4.1-20260810T210953Z-pilot-142-control-isolated-plan

## Definition-of-done coverage

| Required outcome or proof | Work unit IDs | Notes |
|---|---|---|

| Create button-chain.html with exactly one initial button | W01 | The markup unit owns the future file and initial control. |

| Append exactly one button below the current last button | W03,W06,W05 | The handler owns state transitions, the button-stack style owns vertical placement, and the browser story verifies direct clicks. |

| Pressing the fourth generated button clears the document | W03,W05 | The handler owns the terminal generated-button count and browser verification exercises it. |

| Completion prints exact lowercase finished with visible white border | W02,W03,W04,W05 | Style, script, static inspection, and browser verification cover the terminal state. |

| Generated buttons appear below the previous current-last button | W06,W05 | The button-stack style owns vertical placement and the browser story verifies it through direct clicks. |

| Document shell goal has local proof before behavior work | W07 | Goal-local verification checks initial markup, terminal style, and vertical layout. |

| Chain behavior goal has local proof before final browser work | W08 | Goal-local verification checks current-last guard and fourth-generated terminal logic. |

## Work units

| ID | Type | File | Primary symbol or file scope | Subscope | Intended change | Depends on | Goal | Step |
|---|---|---|---|---|---|---|---|---|

| W01 | markup | `button-chain.html` | `#button-chain-root` | `N/A` | Create the HTML document body with one initial button and a stable root for appended buttons. | — | 01-document-shell | 01-step-create-root-markup |

| W02 | style | `button-chain.html` | `.completion-state` | `N/A` | Add visible completion styling with a white border around the finished state. | W01 | 01-document-shell | 02-step-style-finished-state |

| W03 | source | `button-chain.html` | `handleChainClick(event)` | `N/A` | Implement delegated click handling so only the current last button appends one button and the fourth generated button clears the document to finished. | W07 | 02-chain-behavior | 01-step-add-chain-handler |

| W04 | verification | `N/A` | `Static acceptance inspection command` | `N/A` | Inspect button-chain.html source after implementation for exact file, initial button, generated-button counter, lowercase finished text, below-button layout, and white-border CSS. | W07,W08 | 03-verification-handoff | 01-step-static-acceptance-check |

| W05 | verification | `N/A` | `US-01 browser flow` | `N/A` | Run the cached direct-click browser story against button-chain.html and record pass/fail evidence. | W04 | 03-verification-handoff | 02-step-browser-story-check |

| W06 | style | `button-chain.html` | `.chain-button` | `N/A` | Style chain buttons so each generated button appears below the previous current-last button. | W01 | 01-document-shell | 03-step-style-button-stack |

| W07 | verification | `N/A` | `Document shell source inspection` | `N/A` | Verify the future document shell has one initial button, completion styling, and vertical chain-button layout before behavior work begins. | W01,W02,W06 | 01-document-shell | 04-step-verify-document-shell |

| W08 | verification | `N/A` | `Chain behavior source inspection` | `N/A` | Verify the future click handler accepts only the current last button and treats the fourth generated button as terminal before final browser proof. | W03 | 02-chain-behavior | 02-step-verify-chain-handler |

## Decomposition review

- [x] Every definition-of-done item maps to one or more work units.
- [x] Every known affected file and changing symbol has its own work unit.
- [x] Every work unit has exactly one goal and one step.
- [x] Each goal has 2–10 work units, or records an allowed exception.
- [x] Each step has exactly one work unit and no unnamed incidental edits.
- [x] Dependencies form an executable order with no cycle.
