# Work-unit inventory: basic-test-proof-current-20260811t123516z-current-fresh7-isolated-plan

## Definition-of-done coverage

| Required outcome or proof | Work unit IDs | Notes |
|---|---|---|

| Create button-chain.html with one initial button. | W01 | W01 owns the future markup and initial control. |

| Pressing the current last button appends exactly one button below it. | W03,W06 | W03 owns the append behavior and W06 proves it through direct browser clicks. |

| Pressing the fourth generated button clears the document. | W04,W06 | W04 owns the completion transition and W06 proves the fourth generated button path. |

| Completion prints exact lowercase text finished with a visible white border. | W02,W04,W06 | W02 owns the border style, W04 owns the exact text output, and W06 observes both. |

| Every appended generated button renders below the previous last button. | W07,W06 | W07 owns the vertical layout and W06 observes the generated sequence in the browser. |

| Older non-last buttons do not append more buttons after a newer last button exists. | W03,W08 | W03 owns last-button authority and W08 proves a stale click does not append. |

| Static source review confirms the complete implementation contract before browser validation. | W05 | W05 checks the implemented file for all markup, layout, script, completion, and styling requirements before browser validation. |

## Work units

| ID | Type | File | Primary symbol or file scope | Subscope | Intended change | Depends on | Goal | Step |
|---|---|---|---|---|---|---|---|---|

| W01 | markup | `button-chain.html` | `#button-chain` | `N/A` | Create the document skeleton with one initial button and a stable container for generated buttons. | — | 01-build-button-chain | 01-step-markup |

| W02 | style | `button-chain.html` | `.completion-message` | `N/A` | Define the completion message styling so the exact text finished has a visible white border. | W01 | 01-build-button-chain | 02-step-completion-style |

| W03 | source | `button-chain.html` | `appendNextButton()` | `N/A` | Append exactly one generated button below the current last button and move last-button authority to the new button. | W01 | 01-build-button-chain | 03-step-append-behavior |

| W04 | source | `button-chain.html` | `completeDocument()` | `N/A` | When the fourth generated button is pressed, clear the document and render only the finished completion state with the visible white border. | W02,W03 | 01-build-button-chain | 04-step-completion-behavior |

| W05 | verification | `N/A` | `Static source review` | `N/A` | Inspect button-chain.html source after implementation for one initial button, vertical below-it layout, generated-count logic, last-button-only append behavior, fourth-generated completion, exact finished text, and visible white border styling. | W01,W02,W03,W04,W07 | 01-build-button-chain | 06-step-static-contract-review |

| W06 | verification | `N/A` | `Browser UI story US-01` | `N/A` | Open the implemented button-chain.html in a browser and click the current last button until the fourth generated button clears the document and shows finished with a visible white border. | W01,W02,W03,W04,W05,W07 | 02-validate-button-chain | 01-step-browser-story-us-01 |

| W07 | style | `button-chain.html` | `.button-chain` | `N/A` | Lay out the button chain vertically so each appended button renders below the previous last button. | W01 | 01-build-button-chain | 05-step-button-chain-layout |

| W08 | verification | `N/A` | `Browser UI story US-02` | `N/A` | After at least one generated button exists, click an older non-last button and verify it does not append another button before continuing the chain. | W01,W03,W07 | 02-validate-button-chain | 02-step-browser-story-us-02 |

## Decomposition review

- [x] Every definition-of-done item maps to one or more work units.
- [x] Every known affected file and changing symbol has its own work unit.
- [x] Every work unit has exactly one goal and one step.
- [x] Each goal has 2–10 work units, or records an allowed exception.
- [x] Each step has exactly one work unit and no unnamed incidental edits.
- [x] Dependencies form an executable order with no cycle.
