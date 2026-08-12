# Work-unit inventory: basic-test-proof-1-3-1-20260811t053701z-pilot-142-matrix-iterative-isolated-plan

## Definition-of-done coverage

| Required outcome or proof | Work unit IDs | Notes |
|---|---|---|

| button-chain.html exists with one initial button | W01 | W01 owns the future document body and initial button target. |

| pressing the current last button appends exactly one button below it | W02 | W02 owns the future append handler and last-button guard. |

| pressing the fourth generated button clears the document | W03 | W03 owns the generated-button counter and clear path. |

| completion state prints exact lowercase finished with a visible white border | W03,W04 | W03 owns the exact text and W04 owns the visible white border. |

| planned regression proof covers behavior without this planning-only run creating HTML | W05,W06 | W05 covers DOM assertions and W06 covers the real-click browser story for future execution. |

## Work units

| ID | Type | File | Primary symbol or file scope | Subscope | Intended change | Depends on | Goal | Step |
|---|---|---|---|---|---|---|---|---|

| W01 | markup | `button-chain.html` | `#button-chain-root` | `N/A` | Create the initial document body with exactly one starting button and a vertical container for generated buttons. | — | 01-define-button-chain-page | 01-step-initial-markup |

| W02 | source | `button-chain.html` | `appendNextButton()` | `N/A` | Define click handling so only pressing the current last button appends exactly one new button below it. | W01 | 01-define-button-chain-page | 02-step-append-handler |

| W03 | source | `button-chain.html` | `finishOnFourthGeneratedButton()` | `N/A` | Define the fourth generated button click path so it clears the document and renders exact lowercase text finished. | W02 | 01-define-button-chain-page | 03-step-finish-handler |

| W04 | style | `button-chain.html` | `.completion-message` | `N/A` | Style the completion state so the finished text has a visible white border. | W03 | 01-define-button-chain-page | 04-step-completion-border |

| W05 | test | `button-chain.html` | `button chain DOM regression test` | `N/A` | Add a planned DOM-level regression check for initial button count, append count, last-button-only behavior, and completion text. | W01,W02,W03,W04 | 02-prove-button-chain-behavior | 01-step-dom-regression |

| W06 | verification | `N/A` | `Browser flow: click initial, generated one, generated two, generated three, generated four` | `N/A` | Run the future browser story through real clicks and confirm the document clears to finished with a visible white border. | W01,W02,W03,W04,W05 | 02-prove-button-chain-behavior | 02-step-browser-story |

## Decomposition review

- [x] Every definition-of-done item maps to one or more work units.
- [x] Every known affected file and changing symbol has its own work unit.
- [x] Every work unit has exactly one goal and one step.
- [x] Each goal has 2–10 work units, or records an allowed exception.
- [x] Each step has exactly one work unit and no unnamed incidental edits.
- [x] Dependencies form an executable order with no cycle.
