# Work-unit inventory: basic-test-proof-1-4-1-20260810t214045z-pilot-142-control2-isolated-plan

## Definition-of-done coverage

| Required outcome or proof | Work unit IDs | Notes |
|---|---|---|

| Future HTML starts with one initial button | W01 | The markup unit owns the initial visible button and stable root. |

| Pressing the current last button appends exactly one button below it | W02,W05 | The logic unit owns the append guard and the browser story proves it through direct clicks. |

| Pressing the fourth generated button clears the document | W02,W05 | The logic unit owns the generated-button counter and terminal clear behavior; the browser story exercises the terminal click. |

| Completion prints exact lowercase finished with a visible white border | W03,W05 | The style unit owns the visible border and the browser story checks the terminal state. |

| Planning-only proof creates no HTML and remains auditable | W06 | The artifact audit checks the isolated benchmark workspace and plan deliverables only. |

| Static source-level semantics are reviewed before browser story verification | W04,W07 | W07 reviews the build goal handoff and W04 performs the later implementation review. |

## Work units

| ID | Type | File | Primary symbol or file scope | Subscope | Intended change | Depends on | Goal | Step |
|---|---|---|---|---|---|---|---|---|

| W01 | markup | `button-chain.html` | `#button-chain-root` | `N/A` | Create the initial document structure with one button inside a stable root/container for the chain. | — | 01-build-button-chain | 01-step-initial-markup |

| W02 | source | `button-chain.html` | `appendNextButton(event)` | `N/A` | Add event logic that appends exactly one button below the current last button and clears the document when the fourth generated button is pressed. | W01 | 01-build-button-chain | 02-step-button-chain-logic |

| W03 | style | `button-chain.html` | `.finished-state` | `N/A` | Style the terminal finished state so the exact lowercase text finished has a visible white border. | W02 | 01-build-button-chain | 03-step-finished-style |

| W04 | verification | `N/A` | `Static implementation review` | `N/A` | Inspect button-chain.html after implementation for exact target count semantics, append guard, terminal text, and border requirement. | W01,W02,W03 | 02-verify-button-chain | 01-step-static-review |

| W05 | verification | `N/A` | `US-01 browser story` | `N/A` | Run the direct browser user story that clicks the current last button through the fourth generated button and observes the finished state. | W04 | 02-verify-button-chain | 02-step-browser-story-us-01 |

| W06 | verification | `N/A` | `Planning proof artifact audit` | `N/A` | Confirm this planning-only proof did not create HTML/HTM artifacts and that all required planning reports are present. | — | 02-verify-button-chain | 03-step-artifact-audit |

| W07 | verification | `N/A` | `Build readiness review` | `N/A` | Review the completed implementation work units W01-W03 for readiness before handing the file to the verification goal. | W01,W02,W03 | 01-build-button-chain | 04-step-build-readiness-review |

## Decomposition review

- [x] Every definition-of-done item maps to one or more work units.
- [x] Every known affected file and changing symbol has its own work unit.
- [x] Every work unit has exactly one goal and one step.
- [x] Each goal has 2–10 work units, or records an allowed exception.
- [x] Each step has exactly one work unit and no unnamed incidental edits.
- [x] Dependencies form an executable order with no cycle.
