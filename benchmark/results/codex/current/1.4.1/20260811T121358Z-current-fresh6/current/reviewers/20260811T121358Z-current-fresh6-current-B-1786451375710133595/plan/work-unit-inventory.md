# Work-unit inventory: basic-test-proof-current-20260811t121358z-current-fresh6-isolated-plan

## Definition-of-done coverage

| Required outcome or proof | Work unit IDs | Notes |
|---|---|---|

| Future file button-chain.html starts with exactly one visible initial button. | W01 | The markup work unit owns the initial DOM subtree. |

| Pressing the current last button appends exactly one generated button below it. | W02,W05,W06 | The source unit implements the append contract and both verification units check it. |

| Pressing generated button four clears the document. | W03,W05,W06 | The completion unit defines the fourth-generated branch and verification checks the resulting empty prior document state. |

| Completion prints exact lowercase text finished. | W03,W05,W06 | The completion branch owns the text and verification checks exact casing. |

| Completion text has a visible white border. | W04,W05,W06 | The style unit owns the border and verification checks visibility. |

| Planning proof contains no created, opened, served, inspected, or tested HTML. | W05,W06 | Both verification steps are planned instructions only in this proof. |

| Clicking a no-longer-current button must not append any generated button. | W02,W07 | The append logic owns the guard and US-02 verifies it through direct browser input. |

## Work units

| ID | Type | File | Primary symbol or file scope | Subscope | Intended change | Depends on | Goal | Step |
|---|---|---|---|---|---|---|---|---|

| W01 | markup | `button-chain.html` | `#button-chain-root` | `N/A` | Create the standalone document body subtree with exactly one initial button visible at load. | — | 01-build-contract | 01-step-initial-button-markup |

| W02 | source | `button-chain.html` | `handleButtonClick` | `N/A` | Add the click handler that appends exactly one generated button below the current last button and ignores non-last buttons for append behavior. | W01 | 01-build-contract | 02-step-last-button-append-logic |

| W03 | source | `button-chain.html` | `completeOnFourthGenerated` | `N/A` | Add the completion branch so pressing generated button four clears the document and renders the exact text finished. | W02 | 01-build-contract | 03-step-fourth-generated-completion |

| W04 | style | `button-chain.html` | `.completion-message` | `N/A` | Style the finished state so the text finished has a visible white border. | W03 | 01-build-contract | 04-step-finished-border-style |

| W05 | verification | `N/A` | `Static HTML acceptance review` | `N/A` | Review the future button-chain.html source against the initial button, append, completion, exact text, and white-border contract before browser validation. | W01,W02,W03,W04 | 01-build-contract | 05-step-static-acceptance-review |

| W06 | verification | `N/A` | `US-01 browser button-chain flow` | `N/A` | Run the future browser story that clicks the current last button until completion and records pass or bug evidence. | W01,W02,W03,W04,W05 | 02-validate-ui-story | 01-step-browser-button-chain-story |

| W07 | verification | `N/A` | `US-02 non-last-button guard flow` | `N/A` | Run the future browser story that verifies clicking a no-longer-last button does not append another button. | W01,W02,W05 | 02-validate-ui-story | 02-step-browser-non-last-guard-story |

## Decomposition review

- [x] Every definition-of-done item maps to one or more work units.
- [x] Every known affected file and changing symbol has its own work unit.
- [x] Every work unit has exactly one goal and one step.
- [x] Each goal has 2–10 work units, or records an allowed exception.
- [x] Each step has exactly one work unit and no unnamed incidental edits.
- [x] Dependencies form an executable order with no cycle.
