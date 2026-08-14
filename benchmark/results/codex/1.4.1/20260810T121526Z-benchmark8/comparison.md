# Planning benchmark comparison: 20260810T121526Z-benchmark8

The six workers all exited successfully and were marked accepted by their
case evaluations. Telemetry below is taken only from the UUID-matched
`telemetry.txt` and `evaluation.md` in the corresponding result archive; no
other session was used to fill a value.

## Batch overview

- Revisions benchmarked: **6**
- Completed successfully: **6**
- Tainted: **0**
- Missing or unusable telemetry: **0**
- Total recorded usage: **25,595,676 tokens**
- Largest plan inventory: **13 work units** in 1.2.0
- Largest archived result: **55 files** in 1.2.0

## Legacy latency

These are exact worker elapsed times recorded by the harness, not estimates.
The batch wall time is measured from the first worker start to the last worker
end; analyzer time is not included because it is not recorded in the worker
evaluations.

| Revision | Worker start | Worker end | Elapsed seconds | Elapsed |
|---|---|---|---:|---:|
| 1.0.0 | 12:15:36Z | 12:19:25Z | 229 | 3m49s |
| 1.1.0 | 12:15:36Z | 12:19:24Z | 228 | 3m48s |
| 1.2.0 | 12:15:36Z | 12:38:52Z | 1,396 | 23m16s |
| 1.3.1 | 12:43:18Z | 13:06:50Z | 1,412 | 23m32s |
| 1.4.0 | 12:15:36Z | 12:43:17Z | 1,661 | 27m41s |
| 1.4.1 | 12:15:36Z | 12:23:03Z | 447 | 7m27s |

Worker elapsed total was 5,373 seconds across six revisions; the mean was
895.5 seconds and the median was 812.5 seconds. The worker batch wall time was
3,074 seconds (51m14s), reflecting the parallel queue and the five-slot limit.
The rollout files referenced by the matched telemetry database do contain
timestamped agent events, so coarse phase boundaries can be reconstructed
after the fact. They do not contain native phase fields, and analyzer time is
not part of a worker rollout; the new protocol should still record setup,
reading, drafting, review, fix, validation, and analysis explicitly.

## Reconstructed phase latency

These intervals are inferred from timestamped rollout messages, not read from
phase columns in SQLite. “Draft” ends at the worker's first explicit draft
completion message. “Review/fix” runs from that point to the final approval
message where one exists. “Final validation/report” runs from that approval to
the worker completion message. A combined finalization interval is shown when
the legacy run did not emit a distinct approval boundary. Setup and reading
are included in the draft interval because the old protocol did not mark them
separately; analyzer time remains unavailable.

| Revision | Draft/setup + reading | Review/fix | Final validation/report | Boundary quality |
|---|---:|---:|---:|---|
| 1.0.0 | 142s (2m22s) | Not recorded; no review boundary | 87s (1m27s) | Explicit draft and completion messages; validator unavailable |
| 1.1.0 | 176s (2m56s) | Not recorded; no review boundary | 52s (0m52s) | Explicit draft and completion messages; validator unavailable |
| 1.2.0 | 198s (3m18s) | 1,043s (17m23s) | 155s (2m35s) | Explicit draft, final approval, and completion messages |
| 1.3.1 | 283s (4m43s) | 985s (16m25s) | 143s (2m23s) | Explicit draft, final approval, and completion messages |
| 1.4.0 | 275s (4m35s) | 1,216s (20m16s) | 169s (2m49s) | Explicit draft, final approval, and completion messages |
| 1.4.1 | 340s (5m40s) | Not separately emitted | 106s (1m46s) combined finalization | Explicit draft and completion; one correction occurred before draft marker |

The phase totals reconcile to the recorded worker elapsed time within message
boundary rounding. These results show that the large latency increase in
1.2.0–1.4.0 is concentrated in review/fix activity, while 1.4.1 is shorter
because it emits a compact one-goal run with no recorded multi-round review
interval. They should be treated as useful historical estimates, not as a
replacement for first-class phase telemetry.

## Legacy telemetry composition and activity

These values come from the matched parent rollout plus the SQLite thread
spawn edges. Token composition is especially useful for the optimization
discussion: the large totals are dominated by input/context processing, not
by generated answer text. The token columns below are parent-worker totals;
reviewer child threads are counted separately and are not folded into them.

| Revision | Input tokens | Cached input | Output tokens | Reasoning output | Cached/input | Agent messages | Reasoning events | Shell calls | Spawned child threads |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 1.0.0 | 510,015 | 452,608 | 10,799 | 1,476 | 88.7% | 6 | 16 | 15 | 0 |
| 1.1.0 | 450,335 | 415,488 | 10,108 | 1,612 | 92.3% | 4 | 17 | 16 | 0 |
| 1.2.0 | 5,369,754 | 5,222,400 | 40,935 | 4,847 | 97.3% | 11 | 63 | 61 | 6 |
| 1.3.1 | 9,582,635 | 9,395,968 | 39,170 | 9,000 | 98.1% | 7 | 107 | 102 | 6 |
| 1.4.0 | 7,229,774 | 7,068,160 | 48,705 | 6,450 | 97.8% | 8 | 77 | 74 | 5 |
| 1.4.1 | 2,283,598 | 2,161,408 | 19,848 | 3,938 | 94.7% | 4 | 41 | 40 | 0 |

The most actionable historical signal is that 1.2.0–1.4.0 increased mainly
through repeated input/context evaluation and review activity. Generated
output remained comparatively small. This supports measuring context bytes,
cache hits, file-read volume, and reviewer subthread cost explicitly in the
new protocol rather than using total tokens alone.

## Recoverable reviewer and command metrics

Reviewer totals and durations are exact for child threads recorded in
`thread_spawn_edges`. Validator counts are rollout command-pattern counts.
Read/write counts are conservative heuristics over recorded `exec` command
payloads; they indicate activity volume, not actual filesystem syscalls.

| Revision | Reviewer threads | Reviewer tokens | Reviewer elapsed | Read-like commands | Write-like commands | Validator command matches |
|---|---:|---:|---:|---:|---:|---:|
| 1.0.0 | 0 | 0 | 0s | 9 | 7 | 5 |
| 1.1.0 | 0 | 0 | 0s | 9 | 4 | 3 |
| 1.2.0 | 6 | 1,431,242 | 548s | 20 | 30 | 11 |
| 1.3.1 | 6 | 1,990,635 | 620s | 41 | 21 | 9 |
| 1.4.0 | 5 | 1,627,414 | 616s | 26 | 33 | 7 |
| 1.4.1 | 0 | 0 | 0s | 24 | 15 | 5 |

The complete extracted records, including source paths, thread IDs, child
reviewer records, token fields, and extraction qualifications, are preserved
in [`legacy-telemetry-raw.json`](./legacy-telemetry-raw.json).

The rollouts also preserve lower-level activity volume:

| Revision | Rollout records | Tool input chars | Tool output chars | Function calls | Patch events | World-state records |
|---|---:|---:|---:|---:|---:|---:|
| 1.0.0 | 87 | 32,024 | 69,050 | 0 | 3 | 1 |
| 1.1.0 | 88 | 30,986 | 36,328 | 0 | 4 | 1 |
| 1.2.0 | 364 | 127,325 | 179,986 | 18 | 19 | 7 |
| 1.3.1 | 492 | 106,820 | 294,466 | 10 | 12 | 7 |
| 1.4.0 | 409 | 162,400 | 204,217 | 19 | 15 | 6 |
| 1.4.1 | 189 | 59,019 | 205,576 | 0 | 9 | 1 |

Payload character totals measure serialized telemetry volume, not billed
tokens. They are useful proxies for command/result verbosity and retained
tool-output pressure.

## Deliverable inventory by revision

| Revision | Goals | Work units | UI story/cache files | Testing companions | Review reports | Bug-register files | Context artifacts | Validation/analysis reports | Plan files | Result files | Telemetry records | Tokens | Status |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| 1.0.0 | 1 | 8 | 2 | 2 | 1 | 1 | 1 | 2 | 16 | 30 | 1 | 520,814 | Accepted |
| 1.1.0 | 1 | 6 | 2 | 1 | 1 | 1 | 1 | 2 | 14 | 26 | 1 | 460,443 | Accepted |
| 1.2.0 | 2 | 13 | 2 | 13 | 1 | 1 | 1 | 2 | 40 | 55 | 1 | 5,410,689 | Accepted |
| 1.3.1 | 2 | 5 | 2 | 5 | 1 | 1 | 1 | 2 | 24 | 37 | 1 | 9,621,805 | Accepted |
| 1.4.0 | 2 | 6 | 2 | 6 | 1 | 2 | 7 | 2 | 35 | 49 | 1 | 7,278,479 | Accepted |
| 1.4.1 | 1 | 5 | 2 | 5 | 1 | 1 | 4 | 2 | 26 | 39 | 1 | 2,303,446 | Accepted |

Counting note: work units are rows whose IDs begin with `WU-` or `W` in the
work-unit inventory; plan files are all files below the selected plan
directory; result files are all files below the revision result directory.
UI story/cache files include the user-story document and the corresponding
`ui-story-runs/` file. Review and bug columns count archived report files, not
individual findings. Context artifacts count the context snapshot report and
any archived snapshot metadata files. The run predates the archive-layout
fix, so these result directories do not contain the later-added tagged
`planning/` skill copy.

## Token usage progression

Revisions are ordered semantically. Each percentage is calculated as
`(current - previous) / previous * 100`; these are observed usage changes, not
proof of causation.

| Revision | Previous revision | Previous tokens | Current tokens | Delta | Change | Evidence-based interpretation |
|---|---|---:|---:|---:|---|---|
| 1.1.0 | 1.0.0 | 520,814 | 460,443 | -60,371 (-11.59%) | Decrease | Slightly smaller one-goal plan and fewer testing companions. |
| 1.2.0 | 1.1.0 | 460,443 | 5,410,689 | +4,950,246 (+1075.11%) | Increase | Strongly associated with expansion to two goals, 13 work units, and a much denser review/evidence package. |
| 1.3.1 | 1.2.0 | 5,410,689 | 9,621,805 | +4,211,116 (+77.83%) | Increase | Despite only five work units, the journey records multiple review/fix passes and extensive correction evidence; the archive supports expansion, not a simple size-only explanation. |
| 1.4.0 | 1.3.1 | 9,621,805 | 7,278,479 | -2,343,326 (-24.35%) | Decrease | Usage fell while the plan retained two goals and six work units; the evidence suggests a less expensive review path than 1.3.1, not reduced quality. |
| 1.4.1 | 1.4.0 | 7,278,479 | 2,303,446 | -4,975,033 (-68.35%) | Decrease | Compact one-goal/five-unit plan with fewer review artifacts and a shorter journey; likely reduced planning expansion, though token totals alone cannot establish cause. |

Overall, usage expands sharply through 1.3.1, then contracts across 1.4.0 and
1.4.1. The strongest observable relationship is between token expansion and
review/correction density, but these are post-hoc correlations rather than a
controlled causal measurement.

| Revision | Worker exit status | Validation result | HTML/HTM artifact audit | Session UUID | Telemetry records | Token total | Status | Taint reasons |
|---|---:|---|---|---|---:|---:|---|---|
| 1.0.0 | 0 | Tagged validator unavailable/not run (`validate-plan.sh` absent, exit 127); equivalent structural validation pass | Pass; 0 files | `019feb99-8bdc-7131-a08d-dca2438a815f` | 1 | 520,814 | Accepted | None |
| 1.1.0 | 0 | Tagged validator unavailable/not run (`validate-plan.sh` absent, exit 127); manual artifact gate pass | Pass; 0 files | `019feb99-8bfe-7be3-a8df-9e648b2b5a7e` | 1 | 460,443 | Accepted | None |
| 1.2.0 | 0 | Pass: 13 work units across 2 goals | Pass; 0 files | `019feb99-8be0-7dd1-84e7-154b95333ee7` | 1 | 5,410,689 | Accepted | None |
| 1.3.1 | 0 | Pass: 5 work units across 2 goals | Pass; 0 files | `019febb2-e780-7c71-8c11-ae298689c82a` | 1 | 9,621,805 | Accepted | None |
| 1.4.0 | 0 | Pass: 6 work units across 2 goals | Pass; 0 files | `019feb99-8c22-7400-8b1e-6208e3472a16` | 1 | 7,278,479 | Accepted | None |
| 1.4.1 | 0 | Pass: 5 work units across 1 goal | Pass; 0 files | `019feb99-8bfe-73a0-b67d-cb39762eb397` | 1 | 2,303,446 | Accepted | None |

The 1.0.0 and 1.1.0 validators were unavailable in those tagged source
trees, but both equivalent/manual structural gates passed and the harness
accepted the cases. All revisions also passed their process audits; the UI
story remained planned rather than browser-executed, as required by the
benchmark constraint.

## Developer journey by revision

### 1.0.0

The worker created a one-goal durable plan with the button-chain decomposition,
testing companion, UI story/cache, trackers, review, bug register, and context
snapshot, while explicitly keeping HTML and browser execution out of scope.
Review and correction counts are not recorded; the review artifact reports an
internally consistent plan. The tagged validator was attempted but absent
(exit 127), after which the worker recorded a passing equivalent structural
gate and a clean zero-HTML audit.

### 1.1.0

The worker produced a one-goal plan with six atomic work units covering setup,
counting, appending, terminal clearing, rendering, and future verification.
Review and correction-cycle counts are not recorded; the saved review passed
plan completeness while retaining open risks in the bug register. The tagged
validator was invoked and unavailable (exit 127), but all mandatory artifacts,
the manual gate, and the no-HTML/process audit passed.

### 1.2.0

The worker started with a six-unit draft, then used five fresh secondary review
rounds to expose and resolve the off-by-one click sequence, work-unit ownership,
deterministic browser targets, scope mismatches, and a stale approval status.
The plan was strengthened into two goals and 13 atomic work units, including a
separate UI-validation goal and explicit report/audit ownership; one context
count was also corrected before the final pass. Tagged validation passed, the
review ended approved, and the browser story stayed explicitly untested with
no HTML artifact.

### 1.3.1

The worker decomposed the task into two goals and five atomic units, then
corrected the central ambiguity by defining four generated-button appends plus
a fifth click that presses generated button 4 and clears the document. Multiple
review passes are observable, with the exact count not recorded; the documented
fixes covered the off-by-one sequence, missing/ mismatched artifacts and
selectors, testing cache, bug feedback, and final status synchronization. The
tagged validator passed and the final independent review approved the plan;
telemetry was available in the runner archive, while browser execution remained
excluded.

### 1.4.0

The worker built a six-unit, two-goal plan and moved through five fresh review
rounds: four rejected drafts surfaced ten initial findings, followed by three
remaining structural/behavioral issues. Corrections made the five-click
sequence explicit, required a contrasting background with a `1px solid white`
border, gated UI verification on the terminal behavior, and narrowed one unit
to a single symbol. The fifth review approved the result, tagged validation
passed, and the final audit found no HTML/browser artifacts.

### 1.4.1

The worker used the tagged helper scripts to create a compact one-goal,
five-unit plan, with the UI story and cache explicitly marked as future proof
only. One observable correction removed prohibited shortcut wording from the
UI evidence; the approved review also records resolved findings for coverage,
the fourth-generated-button wording, exact casing, and the visible border, but
the total review-round count is not recorded. Tagged validation passed and all
mandatory artifacts were present, while the worker made no browser or HTML
execution claim.
