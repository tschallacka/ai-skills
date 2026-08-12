# Work-unit inventory: basic-test-proof-1.4.1-20260810T205301Z-pilot-142-final-isolated-plan

## Definition-of-done coverage

| Required outcome or proof | Work unit IDs | Notes |
|---|---|---|

| button-chain.html exists with one initial button and no extra initial UI | W01 | Root markup owns the future file and initial state contract. |

| pressing the current last button appends exactly one button below it | W02,W05 | The handler defines the behavior and US-01 verifies it through direct clicks. |

| pressing the fourth generated button clears the document | W03,W06 | The terminal branch defines the clear operation and US-02 verifies the destructive completion action. |

| completion state prints exact lowercase finished with visible white border | W04,W06 | The style unit owns the white border and US-02 verifies exact text and border visibility. |

| planning-only proof creates no HTML, browser, server, driver, or execution artifact | W05,W06 | Verification units are future instructions only in this run; no execution tooling is started. |

| implementation contract review confirms the future file contains all planned targets before browser stories | W07 | Goal-local verification satisfies the tagged skill testing requirement for W01 through W04. |

## Work units

| ID | Type | File | Primary symbol or file scope | Subscope | Intended change | Depends on | Goal | Step |
|---|---|---|---|---|---|---|---|---|

| W01 | markup | `button-chain.html` | `#button-chain-root` | `N/A` | Create the root document body subtree containing exactly one initial button and no completion message at load. | none | 01-button-chain-contract | 01-step-root-markup |

| W02 | source | `button-chain.html` | `appendNextButton()` | `N/A` | Define the click handler behavior so only the current last button appends exactly one new button below itself. | W01 | 01-button-chain-contract | 02-step-append-handler |

| W03 | source | `button-chain.html` | `finishOnFourthGeneratedButton()` | `N/A` | Define the terminal branch so pressing the fourth generated button clears the document and prints exactly lowercase finished. | W02 | 01-button-chain-contract | 03-step-finish-handler |

| W04 | style | `button-chain.html` | `.completion-message` | `N/A` | Style the completion text with a visible white border while keeping the exact text content finished. | W03 | 01-button-chain-contract | 04-step-completion-border |

| W05 | verification | `N/A` | `Browser flow US-01 incremental append` | `N/A` | Verify by direct browser clicks that each press on the current last button appends exactly one button below it before completion. | W01,W02,W07 | 02-user-story-verification | 01-step-verify-append-story |

| W06 | verification | `N/A` | `Browser flow US-02 fourth-generated finish` | `N/A` | Verify by direct browser clicks that pressing the fourth generated button clears the document and shows exactly finished with a visible white border. | W01,W02,W03,W04,W05,W07 | 02-user-story-verification | 02-step-verify-finish-story |

| W07 | verification | `N/A` | `Implementation contract review W01-W04` | `N/A` | Verify by bounded code review after future implementation that W01 through W04 are present and ready for browser story execution. | W01,W02,W03,W04 | 01-button-chain-contract | 05-step-contract-review |

## Decomposition review

- [x] Every definition-of-done item maps to one or more work units.
- [x] Every known affected file and changing symbol has its own work unit.
- [x] Every work unit has exactly one goal and one step.
- [x] Each goal has 2–10 work units, or records an allowed exception.
- [x] Each step has exactly one work unit and no unnamed incidental edits.
- [x] Dependencies form an executable order with no cycle.
