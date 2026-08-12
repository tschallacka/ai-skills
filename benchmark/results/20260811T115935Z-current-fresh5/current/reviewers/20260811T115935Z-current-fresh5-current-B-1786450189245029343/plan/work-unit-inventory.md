# Work-unit inventory: basic-test-proof-current-20260811t115935z-current-fresh5-isolated-plan

## Definition-of-done coverage

| Required outcome or proof | Work unit IDs | Notes |
|---|---|---|

| button-chain.html is the only future implementation artifact | W01,W02,W03,W04 | Markup, script, terminal behavior, and style units cover the single file without allowing extra artifacts. |

| Initial page shows exactly one button | W01 | The document structure unit owns the initial rendered button. |

| Pressing the current last button appends exactly one button below it | W02,W05 | The append function owns behavior and the browser story proves it through clicks. |

| Pressing the fourth generated button clears the document | W03,W05 | The terminal branch owns clearing and the browser story proves it. |

| Completion state prints exact lowercase finished with visible white border | W03,W04,W05 | Text, border styling, and browser verification collectively prove the final state. |

## Work units

| ID | Type | File | Primary symbol or file scope | Subscope | Intended change | Depends on | Goal | Step |
|---|---|---|---|---|---|---|---|---|

| W01 | markup | `button-chain.html` | `#button-chain-root` | `N/A` | Create the HTML document structure containing one initial button and a vertical host for subsequently generated buttons. | — | 01-button-chain-html | 01-step-document-structure |

| W02 | source | `button-chain.html` | `appendNextButton()` | `N/A` | Implement the click path for the current last button so one and only one new button is appended below it. | W01 | 01-button-chain-html | 02-step-append-current-last-button |

| W03 | source | `button-chain.html` | `finishOnFourthGeneratedButton()` | `N/A` | Implement the terminal branch so pressing the fourth generated button clears the document and renders exactly finished. | W02 | 01-button-chain-html | 03-step-finish-on-fourth-generated |

| W04 | style | `button-chain.html` | `.completion-message` | `N/A` | Style the completion state so the finished text has a visible white border. | W03 | 01-button-chain-html | 04-step-finished-border |

| W05 | verification | `N/A` | `Browser user story US-01` | `N/A` | Verify through real browser clicks that the button chain appends exactly one button per click and completes with bordered finished text after the fourth generated button is pressed. | W01,W02,W03,W04 | 01-button-chain-html | 05-step-browser-story-verification |

## Decomposition review

- [x] Every definition-of-done item maps to one or more work units.
- [x] Every known affected file and changing symbol has its own work unit.
- [x] Every work unit has exactly one goal and one step.
- [x] Each goal has 2–10 work units, or records an allowed exception.
- [x] Each step has exactly one work unit and no unnamed incidental edits.
- [x] Dependencies form an executable order with no cycle.
