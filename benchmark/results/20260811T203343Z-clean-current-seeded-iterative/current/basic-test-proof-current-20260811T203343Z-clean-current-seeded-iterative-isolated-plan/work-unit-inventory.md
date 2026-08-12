# Work-unit inventory: basic-test-proof-current-20260811t203343z-clean-current-seeded-iterative-isolated-plan

## Definition-of-done coverage

| Required outcome or proof | Work unit IDs | Notes |
|---|---|---|

| Initial document has one button. | W01 | W01 owns the initial markup contract. |

| Only the current last button appends exactly one new button below it. | W03 | W03 owns the append handler and last-button guard. |

| The fourth generated button clears the document. | W04 | W04 owns the generated-button counter and clear operation. |

| Completion prints exact lowercase finished with a visible white border. | W02,W04 | W02 owns the border style and W04 owns exact completion rendering. |

| Executor proves behavior without extra artifacts. | W05,W06 | W05 owns browser evidence and W06 owns artifact audit evidence. |

| Implementation goal has its own readiness proof before final UI verification. | W07 | W07 gives 01-build-button-chain an owned verification unit while W05 remains the browser acceptance proof. |

## Work units

| ID | Type | File | Primary symbol or file scope | Subscope | Intended change | Depends on | Goal | Step |
|---|---|---|---|---|---|---|---|---|

| W01 | markup | `button-chain.html` | `#button-chain-app` | `N/A` | Create the single initial-button DOM subtree and completion container target in button-chain.html. | - | 01-build-button-chain | 01-step-create-markup |

| W02 | style | `button-chain.html` | `.completion-message` | `N/A` | Define the visible white border styling for the finished completion state in button-chain.html. | W01 | 01-build-button-chain | 02-step-style-completion |

| W03 | source | `button-chain.html` | `appendNextButton()` | `N/A` | Add click behavior so pressing the current last button appends exactly one new button below it and disables or ignores prior buttons as append sources. | W01 | 01-build-button-chain | 03-step-append-behavior |

| W04 | source | `button-chain.html` | `finishOnFourthGeneratedButton()` | `N/A` | Add completion behavior so pressing the fourth generated button clears the document and renders exactly finished with the visible white border. | W02,W03 | 01-build-button-chain | 04-step-finish-behavior |

| W05 | verification | `N/A` | `US-01 browser flow` | `N/A` | Run the browser user story that clicks through the current-last-button chain and verifies completion output. | W01,W02,W03,W04 | 02-verify-button-chain | 01-step-browser-story |

| W06 | verification | `N/A` | `artifact audit` | `N/A` | Audit that only button-chain.html is created for implementation and that it contains the planned contract without unrelated execution artifacts. | W01,W02,W03,W04 | 02-verify-button-chain | 02-step-artifact-audit |

| W07 | verification | `N/A` | `implementation readiness check` | `N/A` | Check button-chain.html after W01-W04 for the planned markup, style, append handler, finish handler, and no unrelated implementation scope before handing to final UI verification. | W01,W02,W03,W04 | 01-build-button-chain | 05-step-implementation-readiness |

## Decomposition review

- [x] Every definition-of-done item maps to one or more work units.
- [x] Every known affected file and changing symbol has its own work unit.
- [x] Every work unit has exactly one goal and one step.
- [x] Each goal has 2–10 work units, or records an allowed exception.
- [x] Each step has exactly one work unit and no unnamed incidental edits.
- [x] Dependencies form an executable order with no cycle.
