# Work-unit inventory: basic-test-proof-1-4-0-20260810t121526z-benchmark8-isolated-plan

## Definition-of-done coverage

| Required outcome or proof | Work unit IDs | Notes |
|---|---|---|

| The future HTML contains one initial button and appends exactly one button below the current last button on each of the first three clicks | W01,W03 | Markup establishes the initial control and behavior unit owns the append rule; proof is recorded by the downstream browser story. |

| Pressing generated button 4 clears the document and shows exact lowercase finished with a visibly distinguishable white border | W03,W02,W04 | The initial click creates generated button 1; generated buttons 1–3 create 2–4; pressing generated button 4 terminates. The behavior unit, style unit, and browser story prove the contract. |

| The plan is executable without creating or testing HTML during this proof | W01,W02,W03,W04 | All implementation targets are future-work instructions and W04 is a bounded verification flow; this benchmark itself performs no HTML work. |

| Goal 01's future implementation contract is internally consistent before browser execution | W01,W02,W03,W05 | W05 is an atomic verification owned by Goal 01; it checks target/dependency/terminal invariants without starting browser tooling during this proof. |

| The future initializer establishes the button-chain root, initial control, generated-button counter, and handler attachment without bundling handler logic | W01,W06 | W06 is a separate atomic source unit so the click handler W03 has exactly one primary symbol. |

## Work units

| ID | Type | File | Primary symbol or file scope | Subscope | Intended change | Depends on | Goal | Step |
|---|---|---|---|---|---|---|---|---|

| W01 | markup | `button-chain.html` | `#button-chain-root` | `N/A` | Define the single initial button and its containing DOM subtree in the future button-chain.html document. | — | 01-button-chain | 01-step-markup |

| W02 | style | `button-chain.html` | `.completion-message` | `N/A` | Define the visible white border and presentation for the terminal finished message. | — | 01-button-chain | 02-step-completion-style |

| W03 | source | `button-chain.html` | `button-chain click handler` | `N/A` | Implement the future click behavior: append exactly one button below the current last button through generated buttons 1–4, then clear the document when generated button 4 is pressed and render exact lowercase finished text using the completion-message styling hook. | W01,W02,W06 | 01-button-chain | 03-step-behavior |

| W04 | verification | `N/A` | `US-01 button-chain browser flow` | `N/A` | Verify through direct mouse clicks that initial state, one-button-per-click growth through generated button 4, generated-button-4 clearing, exact finished text, contrasting background, and visible white border all match the acceptance contract. | W01,W02,W03,W05 | 02-ui-validation | 01-step-us-01 |

| W05 | verification | `N/A` | `future button-chain implementation contract review` | `N/A` | Check that the four implementation units specify one concrete HTML file, one initial button, current-last traversal, exact append placement/counts, terminal DOM invariant, and explicit white-border declaration before handing off to browser validation. | W01,W02,W03,W06 | 01-button-chain | 04-step-contract-review |

| W06 | source | `button-chain.html` | `button-chain initializer` | `N/A` | Initialize #button-chain-root with one initial button, the generated-button counter state, and the current-last click-handler attachment point without implementing the click branch. | W01 | 01-button-chain | 05-step-initializer |

## Decomposition review

- [x] Every definition-of-done item maps to one or more work units.
- [x] Every known affected file and changing symbol has its own work unit.
- [x] Every work unit has exactly one goal and one step.
- [x] Each goal has 2–10 work units, or records an allowed exception.
- [x] Each step has exactly one work unit and no unnamed incidental edits.
- [x] Dependencies form an executable order with no cycle.
