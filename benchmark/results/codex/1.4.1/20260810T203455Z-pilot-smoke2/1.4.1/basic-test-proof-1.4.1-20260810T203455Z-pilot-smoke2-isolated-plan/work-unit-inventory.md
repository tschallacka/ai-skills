# Work-unit inventory: basic-test-proof-v1-4-1-20260810t203455z-pilot-smoke2-isolated-plan

## Definition-of-done coverage

| Required outcome or proof | Work unit IDs | Notes |
|---|---|---|

| button-chain.html is created as the sole future deliverable | W01 | Markup work unit owns the file shell and initial button. |

| Pressing the current last button appends exactly one button below it | W02 | Script work unit owns the click handler and last-button rule. |

| Pressing the fourth generated button clears the document | W03 | Script terminal-state branch owns clear behavior for generated button number four. |

| Completion prints exact lowercase text finished with visible white border | W03,W04 | Script writes exact text; style work unit owns visible white border. |

| Direct UI verification proves the button-chain behavior | W05 | Browser-story verification work unit owns direct click proof after future implementation. |

| Implementation goal has a bounded readiness proof before handoff to UI validation | W06 | Readiness verification reviews the implemented file after future implementation without replacing the direct browser story. |

| Analysis report records revision, timestamps, elapsed, worker result, thread ID, token usage, validation, review, and audits | W07 | The report work unit owns the required benchmark evidence artifact. |

| Final workspace audit proves no HTML/HTM artifact and mandatory plan deliverables are present | W08 | The audit work unit owns the final isolated workspace inspection. |

## Work units

| ID | Type | File | Primary symbol or file scope | Subscope | Intended change | Depends on | Goal | Step |
|---|---|---|---|---|---|---|---|---|

| W01 | markup | `button-chain.html` | `#button-chain-root` | `N/A` | Create a valid HTML document containing exactly one initial button and no pre-rendered generated buttons. | — | 01-create-button-chain-file | 01-step-create-initial-markup |

| W02 | source | `button-chain.html` | `appendNextButton()` | `N/A` | Add click behavior so only pressing the current last button appends exactly one new button directly below it. | W01 | 01-create-button-chain-file | 02-step-add-append-handler |

| W03 | source | `button-chain.html` | `handleTerminalGeneratedButton()` | `fourth generated button branch` | When the fourth generated button is pressed, clear the document and render only the completion state with exact text finished. | W02 | 01-create-button-chain-file | 03-step-add-terminal-state |

| W04 | style | `button-chain.html` | `.completion-message` | `N/A` | Style the completion state with a visible white border while preserving the exact lowercase text finished. | W03 | 01-create-button-chain-file | 04-step-style-completion-border |

| W05 | verification | `N/A` | `Browser UI story US-01` | `N/A` | Open the implemented button-chain.html in a browser and use direct clicks to verify one-button appends, fourth-generated terminal clearing, exact finished text, and visible white border. | W01,W02,W03,W04 | 02-verify-button-chain-flow | 01-step-run-ui-story |

| W06 | verification | `N/A` | `Implementation readiness review` | `N/A` | After W01-W04 are implemented in the future, review the file-level behavior contract before handing off to browser UI story validation. | W01,W02,W03,W04 | 01-create-button-chain-file | 05-step-review-implementation-readiness |

| W07 | docs | `analysis-report.md` | `Benchmark evidence report` | `N/A` | Write the analysis report with exact start/end timestamps, elapsed time, worker result, validation results, review result, artifact/process audit, thread ID, and token-usage availability. | W01,W02,W03,W04,W05,W06 | 03-record-benchmark-evidence | 01-step-write-analysis-report |

| W08 | verification | `N/A` | `Isolated workspace final audit` | `N/A` | Audit only the benchmark workspace for expected plan output, mandatory non-empty artifacts, and absence of forbidden HTML/HTM files before completion. | W07 | 03-record-benchmark-evidence | 02-step-run-final-artifact-audit |

## Decomposition review

- [x] Every definition-of-done item maps to one or more work units.
- [x] Every known affected file and changing symbol has its own work unit.
- [x] Every work unit has exactly one goal and one step.
- [x] Each goal has 2–10 work units, or records an allowed exception.
- [x] Each step has exactly one work unit and no unnamed incidental edits.
- [x] Dependencies form an executable order with no cycle.
