# Work-unit inventory: basic-test-proof-1.2.0-20260810T121526Z-benchmark8-isolated-plan

## Definition-of-done coverage

| Required outcome or proof | Work unit IDs | Notes |
|---|---|---|
| Deterministic initial button and named subtree | W01 | Fixes accessible initial label `Button 0` and the workspace-root file target. |
| One-button append behavior | W02 | Owns the append function. |
| Fourth generated-button completion behavior | W03 | Owns terminal `Button 4`, exact text, and clearing branch. |
| Visible white completion border | W04 | Owns the completion selector. |
| Tagged validator execution and saved output | W06, W07 | Separate command and report ownership. |
| HTML artifact audit and process cleanup audit | W09, W10 | Separate bounded safety verifications. |
| Browser acceptance and story/cache evidence | W05, W11, W12 | Separate browser flow, story result, and cache result ownership. |
| Bug register and benchmark analysis | W13, W08 | Separate bug register and report ownership. |

## Work units

| ID | Type | File | Primary symbol or file scope | Subscope | Intended change | Depends on | Goal | Step |
|---|---|---|---|---|---|---|---|---|
| W01 | markup | `button-chain.html` | `#button-chain` | `N/A` | Create one initial accessible `Button 0` in a stable workspace-root document subtree. | — | 01-button-chain | 01-step-initial-markup |
| W02 | source | `button-chain.html` | `appendNextButton()` | `click handler callback` | Append exactly one next button labelled sequentially `Button 1` through `Button 4` after the current last button. | W01 | 01-button-chain | 02-step-append-behavior |
| W03 | source | `button-chain.html` | `clearAndShowFinished()` | `fourth generated-button branch` | Clear the document when generated `Button 4` is pressed and insert exact lowercase `finished`. | W02 | 01-button-chain | 03-step-completion-behavior |
| W04 | style | `button-chain.html` | `.completion-message` | `N/A` | Give the completion text a visible white border. | W03 | 01-button-chain | 04-step-completion-style |
| W06 | verification | `N/A` | `tagged validator command` | `N/A` | Run the tagged validator against the plan directory only. | W01, W02, W03, W04 | 01-button-chain | 06-step-static-handoff |
| W07 | docs | `validation.md` | `Validator output report` | `N/A` | Save the exact validator command, output, and exit code. | W06 | 01-button-chain | 07-step-validation-report |
| W09 | verification | `N/A` | `workspace HTML/HTM artifact audit` | `N/A` | Audit only the isolated workspace for forbidden HTML/HTM files. | W01, W02, W03, W04 | 01-button-chain | 09-step-html-audit |
| W10 | verification | `N/A` | `worker process-group cleanup audit` | `N/A` | Confirm no matching browser/server/driver process remains from this worker. | W06 | 01-button-chain | 10-step-process-audit |
| W05 | verification | `N/A` | `US-01 button-chain acceptance flow` | `N/A` | Execute the exact five-click browser flow against the future workspace-root file. | W01, W02, W03, W04 | 02-ui-validation | 01-step-browser-acceptance |
| W11 | docs | `ui-user-stories.md` | `US-01 result row` | `N/A` | Record the browser story status and evidence without weakening its direct-input contract. | W05 | 02-ui-validation | 02-step-story-result |
| W12 | docs | `ui-story-runs/US-01.md` | `US-01 run cache` | `N/A` | Record every click, readiness wait, and actual run result. | W05 | 02-ui-validation | 03-step-story-cache |
| W13 | docs | `bugs.md` | `UI bug register` | `N/A` | Preserve any observed bug with investigation, fix, and retest traceability; retain an empty register when none is found. | W05 | 02-ui-validation | 04-step-bug-register |
| W08 | docs | `analysis-report.md` | `Benchmark execution report` | `N/A` | Record revision, exact timestamps, elapsed time, worker result, validation/review results, artifact/process audit, thread source, and token evidence. | W05, W07, W09, W10, W11, W12, W13 | 02-ui-validation | 05-step-analysis-report |

## Decomposition review

- [x] Every definition-of-done item maps to one or more work units.
- [x] Every known affected file and changing symbol has its own work unit.
- [x] Every work unit has exactly one goal and one step.
- [x] Each goal has 2–10 work units, or records an allowed exception.
- [x] Each step has exactly one work unit and no unnamed incidental edits.
- [x] Dependencies form an executable order with no cycle.
