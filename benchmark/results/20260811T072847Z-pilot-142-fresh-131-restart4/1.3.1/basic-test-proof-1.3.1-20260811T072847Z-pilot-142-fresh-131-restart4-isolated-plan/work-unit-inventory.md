# Work-unit inventory: basic-test-proof-1-3-1-20260811t072847z-pilot-142-fresh-131-restart4-isolated-plan

## Definition-of-done coverage

| Required outcome or proof | Work unit IDs | Notes |
|---|---|---|

| button-chain.html exists with one initial button | W01 | W01 owns the file creation and initial DOM contract. |

| Pressing the current last button appends exactly one button below it | W03,W04,W05 | W03 owns the click behavior; W04 and W05 prove it by simulation and browser interaction. |

| Pressing the fourth generated button clears the document | W03,W04,W05 | W03 owns the fourth-generated-button branch; W04 and W05 verify the clearing behavior. |

| Completion state prints exact lowercase finished with a visible white border | W02,W03,W04,W05 | W02 owns the visible border styling, W03 renders the exact text, and W04/W05 verify both. |

| Planning-only benchmark artifacts and no HTML execution during this run | W04,W05 | The proof records future verification steps but leaves them incomplete and unexecuted during this planning-only run. |

## Work units

| ID | Type | File | Primary symbol or file scope | Subscope | Intended change | Depends on | Goal | Step |
|---|---|---|---|---|---|---|---|---|

| W01 | markup | `button-chain.html` | `#button-chain-root` | `N/A` | Create the single-page HTML body with one initial button inside a stable root container and no extra initial buttons. | — | 01-create-button-chain-page | 01-step-initial-markup |

| W02 | style | `button-chain.html` | `.completion-message` | `N/A` | Add completion-state styling that gives the finished text container a visible white border on a contrasting background. | W01 | 01-create-button-chain-page | 02-step-completion-style |

| W03 | source | `button-chain.html` | `button click handler` | `N/A` | Add JavaScript that appends exactly one button below the current last button on each last-button click and replaces the document with the completion state when the fourth generated button is pressed. | W01,W02 | 01-create-button-chain-page | 03-step-click-behavior |

| W04 | verification | `N/A` | `static-and-simulated HTML behavior check` | `N/A` | Run a bounded local verification that inspects button-chain.html and simulates the click sequence to confirm initial count, exact one-button appends, fourth generated button clearing, exact finished text, and white border styling. | W01,W02,W03 | 02-verify-button-chain-behavior | 01-step-automated-check |

| W05 | verification | `N/A` | `browser story US-01 direct button chain flow` | `N/A` | Open the future button-chain.html file in a browser and use direct clicks through the rendered UI to confirm the complete user-visible flow. | W01,W02,W03,W04 | 02-verify-button-chain-behavior | 02-step-browser-story |

## Decomposition review

- [x] Every definition-of-done item maps to one or more work units.
- [x] Every known affected file and changing symbol has its own work unit.
- [x] Every work unit has exactly one goal and one step.
- [x] Each goal has 2–10 work units, or records an allowed exception.
- [x] Each step has exactly one work unit and no unnamed incidental edits.
- [x] Dependencies form an executable order with no cycle.
