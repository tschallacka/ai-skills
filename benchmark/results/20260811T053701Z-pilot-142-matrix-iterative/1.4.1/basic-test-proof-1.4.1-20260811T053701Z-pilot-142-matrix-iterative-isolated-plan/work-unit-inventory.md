# Work-unit inventory: basic-test-proof-1-4-1-20260811t053701z-pilot-142-matrix-iterative-isolated-plan

## Definition-of-done coverage

| Required outcome or proof | Work unit IDs | Notes |
|---|---|---|

| Create button-chain.html as the only planned HTML artifact | W01,W02,W03 | Markup, script, and style units together cover the single-file implementation contract. |

| Pressing the current last button appends exactly one button below it | W02,W05 | The click handler owns the behavior and the browser story proves it through direct clicks. |

| Pressing the fourth generated button clears the document | W02,W05 | The click handler owns generated-button count and the browser story reaches the fourth generated button through normal UI input. |

| Completion state prints exact lowercase finished with a visible white border | W03,W04,W05 | Style, static inspection, and browser story cover exact text and visible border. |

| Planning-only proof creates no HTML, browser, server, or driver process | W04 | The verification goal records this as an execution constraint and later static audit target. |

| Goal 01 has an owned proof work unit before downstream acceptance validation | W06 | Handoff inspection verifies the implementation goal's own contract before goal 02 executes. |

## Work units

| ID | Type | File | Primary symbol or file scope | Subscope | Intended change | Depends on | Goal | Step |
|---|---|---|---|---|---|---|---|---|

| W01 | markup | `button-chain.html` | `#button-chain-root` | `N/A` | Create the initial page structure with one visible starting button and no pre-rendered generated buttons. | — | 01-create-button-chain-html | 01-step-initial-markup |

| W02 | source | `button-chain.html` | `button-chain script` | `click handler` | Add JavaScript so only the current last button appends exactly one new button below itself, with generated-button counting tracked deterministically. | W01 | 01-create-button-chain-html | 02-step-append-handler |

| W03 | style | `button-chain.html` | `.completion-message` | `N/A` | Add the completion-state styling rule that makes the lowercase finished text visibly bordered in white after document clearing. | W02 | 01-create-button-chain-html | 03-step-finished-style |

| W04 | verification | `N/A` | `static artifact inspection` | `N/A` | Inspect button-chain.html after implementation to confirm the required file exists, contains exactly one initial button in source markup, contains no unrelated files, and encodes the exact finished text. | W06 | 02-verify-button-chain-behavior | 01-step-static-inspection |

| W05 | verification | `N/A` | `US-01 browser story` | `N/A` | Run the direct browser story that clicks the current last button five times, confirms one button is appended when clicking the initial button and generated buttons 1 through 3, and confirms clicking generated button 4 clears the document and shows finished with a visible white border. | W04 | 02-verify-button-chain-behavior | 02-step-browser-story |

| W06 | verification | `N/A` | `goal 01 handoff inspection` | `N/A` | Confirm the implementation handoff names the initial button target, generated button labeling/count rule, terminal trigger as the fourth generated button, and .completion-message styling before goal 02 begins. | W03 | 01-create-button-chain-html | 04-step-implementation-handoff-inspection |

## Decomposition review

- [x] Every definition-of-done item maps to one or more work units.
- [x] Every known affected file and changing symbol has its own work unit.
- [x] Every work unit has exactly one goal and one step.
- [x] Each goal has 2–10 work units, or records an allowed exception.
- [x] Each step has exactly one work unit and no unnamed incidental edits.
- [x] Dependencies form an executable order with no cycle.
