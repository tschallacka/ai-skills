<!-- MODE: DEV -->
# ai-text-editor-usage benchmark

Reproducible comparison of a real Claude Code worker **with** ai-text-editor
available against one with **only the stock Read/Write/Edit tools**, on the
identical editing task, with an identical external file edit landing while
each worker is still mid-task. Built to answer two things with real numbers,
not opinion: does ai-text-editor actually save tokens on small edits, and
does it actually catch a collision that stock tools would silently clobber.

## Design

- **Task**: `task-prompt.md`, run verbatim against `fixtures/task_source.py`
  (copied fresh into an isolated workspace per run). Three edits: replace
  every `legacy_log(...)` call site with `logger.info(...)`, add a new
  `calculate_discount` function, and add a docstring to `apply_tax` --
  followed by two small export deliverables, `tax_rates.csv` then
  `departments.csv`, which double as progress checkpoints the two
  collision probes below wait for (see "Two collision probes").
- **The two conditions** differ in exactly one thing: whether
  `--mcp-config mcp-config/ai-text-editor.json --strict-mcp-config` is passed
  to `claude -p`. `--strict-mcp-config` with no `--mcp-config` at all
  (the baseline) guarantees zero MCP servers are available regardless of
  what's registered in the real environment's own settings — a clean,
  provable baseline, not just "whatever happens not to be configured".
- **The prompt never mentions ai-text-editor.** The treatment condition is
  only ever told the tool exists by its own MCP tool descriptions and the
  skill's own one-line pitch (if a skill description is loaded at all) —
  whether the worker reaches for it unprompted is itself part of what this
  measures.
- **Two collision probes, EVENT-driven, simulating two different everyday
  causes of the same problem** — a teammate's own concurrent edit, and
  their editor's format-on-save (PhpStorm, VS Code) rewriting the file
  underneath them. Both run from *separate, backgrounded* processes,
  independent of either condition's own tools, so each is a fair, identical
  intervention for both conditions. Each waits (bounded, default 90s) for
  its own checkpoint file to appear in the workspace rather than sleeping a
  fixed delay: a fixed delay cannot mean the same thing across workers of
  very different speed (one lever-factorial sweep's 8 runs ranged 23-55s
  wall clock, so a "+8s" collision landed at a wildly different fraction of
  completion run to run, confounding any comparison across conditions).
  Waiting for the task's own checkpoint files lands each collision at the
  same point in every worker's own progress instead:
  - `external-edit.sh`: waits for `tax_rates.csv` (task-prompt.md's own
    checkpoint right after the three code edits), then makes a single-line
    business-constant bump, `TAX_RATE = 0.07` -> `TAX_RATE = 0.08`, a
    constant the very function being edited (`apply_tax`) reads. Narrow:
    only one line is at risk.
  - `external-format.sh` / `external_reformat.py`: waits for
    `departments.csv` (task-prompt.md's final checkpoint, right before the
    test run), then reindents the whole file, every 3-space indent level
    rewritten at 4 spaces. Broad: it touches essentially every indented
    line in the file at once, so unlike the single-line bump it collides
    directly with whatever lines the worker is *also* editing, not only a
    line it never needed to go near. The fixture is deliberately written
    at 3 spaces per level so this has something to do.
  A worker that holds a stale whole-file copy and rewrites it from memory at
  the end will silently revert one or both; one that re-reads before writing
  (or whose tool tells it the file moved) will not. `apply_bulk_discount`'s
  body is never touched by any of the three requested edits, so its
  indentation in the final file is a clean probe for whether the reindent
  specifically survived, independent of what happened to `TAX_RATE`.
- **A shipped regression test, `test_task_source.py`, that the prompt tells
  the worker to run and make pass.** One of its assertions is pinned to the
  *pre-bump* tax rate (`apply_tax(100) == 107.0`), so once the external
  `TAX_RATE` edit lands, running the test starts failing — not because the
  worker did anything wrong, but because the file on disk changed out from
  under a stale expectation. This is the "how was the failure resolved"
  signal: a careful run updates the test's expectation to `108.0` (the
  bump is real and legitimate); a bad resolution is forcing `TAX_RATE` back
  to `0.07` just to make the stale assertion pass again, which `score.py`
  can also detect directly. The worker never sees anything about the
  benchmark in this file — it reads exactly like an ordinary repo's own
  test suite, because that's what it has to look like to mean anything.
- **One `claude -p` process per condition, not two.** An earlier design tried
  two separate invocations (phase 1 / external edit / phase 2) to force the
  interleaving, but ai-text-editor's own cross-process workspace reconnection
  is keyed by the harness's session id by default — two separate `claude -p`
  calls get two different session ids and so, by default, two unrelated
  servers, which would silently defeat the very collision this is supposed
  to probe. Landing the edit via a backgrounded sleep *while one process is
  still running* sidesteps that entirely and is a more honest simulation of
  "someone else touched the file" besides.

## Running it

```bash
cargo build --release -p ai-text-editor-mcp    # once, if not already built
benchmark/ai-text-editor-usage/run-comparison.sh
```

Optional: `run-comparison.sh <results-base-dir> <edit-timeout-seconds>
<reformat-timeout-seconds>` (default 90s each -- how long to wait for the
worker's own checkpoint file before giving up on that collision, not a
fixed landing delay). Model defaults to `sonnet` (fast, inexpensive, this
session's own current model); override with `AI_TEXT_EDITOR_BENCH_MODEL=opus`
etc.

Each run writes `results/<run-id>/{treatment,baseline}/` (workspace copy,
`worker-stdout.json`/`worker-stderr.log`, `telemetry.txt`/`telemetry.json`,
`score.json`, `external-edit.log`, `evaluation.md`) and
`results/<run-id>/comparison.md`, the side-by-side report. `results/` is
gitignored — a run is real evidence for one invocation, not a committed
baseline, until someone decides a particular run is worth keeping.

## What's measured

From `extract_telemetry.py` (reads the real on-disk session transcript the
same way `benchmark/planning/runtime/claude/agent.sh`'s own
`agent_telemetry` does, extended with tool-call and timing detail):
input/cache-creation/cache-read/output/thinking tokens, total tokens,
tool-call counts by name, wall-clock duration.

**Subagent work is tracked, not just the top-level chain.** If the worker
dispatches its own subagents (the `Task` tool), their transcript lines carry
`isSidechain: true` in the same transcript file; `extract_telemetry.py` sums
tokens and tool calls separately for the main chain and for sidechains
(`main_chain` / `subagent_sidechain` in `telemetry.json`), reports
`subagent_dispatch_count` (how many `Task` calls the main chain made), and
the combined totals still cover both — so a run that quietly offloads the
edit to a subagent isn't undercounted.

**Adoption is observed, never prompted.** `task-prompt.md` never names
ai-text-editor; whether the treatment worker reaches for it is itself a
result, not a precondition, and a run where it doesn't is as much a data
point as one where it does. `telemetry.json`'s `ai_text_editor_adoption`
records whether any `mcp__ai-text-editor__*` tool was ever called, at what
position in the tool-call sequence (so "reached for stock tools first, then
switched" is visible, not just "used it or didn't" — useful for seeing
whether adoption happens later in a longer task rather than immediately),
and which stock tools were called before that point, if any.
`ai_text_editor_text_mentions` separately scans assistant text/thinking
blocks for the tool's name, to catch a worker that reasons about the tool
without ever calling it — raw qualitative input for tuning the tool's own
description/guidance text later, surfaced verbatim in `comparison.md` rather
than interpreted here.

From `score.py`: whether all three requested edits landed, whether the file
still parses as valid Python, whether each of the two external edits
survived or was clobbered (`tax_rate_external_edit_preserved` /
`tax_rate_reverted_to_original`, `external_reformat_preserved` /
`external_reformat_clobbered` via `apply_bulk_discount`'s indentation), and
whether the shipped `test_task_source.py` passes, plus whether its pinned
expectation was updated to match the legitimate rate bump
(`test_updated_to_new_rate`) or left stale (`test_pinned_to_original_rate`)
— the actual regression test runs as a subprocess, not a static guess.
`score.py` also reads the worker's own final summary
(`worker-stdout.json`'s `result`) through `detect_feedback_request`, a
curated phrase list (`your call`, `let me know`, `should I...?`, etc.) that
is deliberately narrower than "contains a question mark" — a run that
explicitly hands an ambiguous call back to the user gets `outcome:
deferred`, distinct from both `pass` and `fail`, since a binary
`tests_pass` cannot otherwise tell "asked rather than guessed" apart from
"got it wrong."

From `extract_edit_timeline.py`: a chronological, per-tool-call
classification of every file-mutating action as an internal edit
(`Edit`/`Write`/`MultiEdit`) or an mcp edit/resolve
(`mcp__ai-text-editor__{insert,replace,save,...}` / `...resolve`), each
with its wall-clock timestamp, plus every mechanism switch (internal↔mcp
mid-task) and, for every switch-to-mcp and every `resolve` call, how many
seconds after the nearest preceding external edit it landed —
`plausibly_disk_triggered` flags anything within 15s as a candidate causal
reaction rather than coincidence. This is what answers "how many edits
went through which mechanism" and "did a real on-disk change actually
trigger the switch," as opposed to `telemetry.json`'s single
`adopted: true/false` bit.

## A real isolation leak, found and fixed by running it

The first run with the two-collision-surface fixture showed the baseline
condition with 5 text mentions of "ai-text-editor" and a `Skill` tool call --
despite `--strict-mcp-config` with no `--mcp-config`, which should mean zero
exposure. It turned out `--strict-mcp-config` only gates MCP *servers*; the
harness's separate `Skill` tool still loads the globally-installed
`ai-text-editor` SKILL.md regardless. The baseline worker had gone looking
for it on its own ("Locating the ai-text-editor CLI so I can make the edits
through it.") and only backed off because the CLI binary sat outside
`--add-dir`, not because it was unaware of the tool. `run-case.sh` now also
passes `--disallowedTools Skill` for the baseline condition, so it is a
genuine zero-exposure control rather than "MCP unavailable, skill still
discoverable." Left as a documented lesson rather than silently patched
away, because it is itself exactly the kind of adoption-path data point this
framework exists to surface: a tool surfaced as a directly-callable MCP tool
(treatment) got silently ignored with zero narrated consideration in two
separate runs, while the same tool reached via the skill-search path
(baseline, pre-fix) got explicit, narrated deliberation before being
rejected for an environmental reason, not a knowledge-of-the-tool reason.

A second lesson from the same batch of runs: `--permission-mode acceptEdits`
pre-approves file edits but not `Bash`, so every run's attempt to actually
run the shipped `test_task_source.py` (which the prompt requires) sat
blocked on an approval prompt a headless `claude -p` invocation can never
answer. Both conditions, across every run under that mode, correctly
diagnosed the TAX_RATE collision in their own final summary and explicitly
asked for guidance on updating the test vs. reverting the rate -- good
judgment that the harness itself prevented from being carried out. Switched
to `--permission-mode bypassPermissions`, appropriate here because every
run's workspace is already a disposable copy under
`results/<run-id>/<condition>/workspace`, never the live repo.

## Testing what it takes to change adoption: the lever factorial

**`task-prompt.md` is byte-identical across every run this framework
makes, with no exception.** `run-case.sh` hardcodes it; there is no flag to
override it. An earlier version of this section described a "nudge sweep"
that varied the *prompt* across five hand-written variants -- that tested
whether coaching the end user into asking for the tool by name changes
adoption, which is not a question this benchmark is for (an end user
should never have to nudge a prompt to get good tool behavior) and that
code and its results have been deleted. See FINDINGS.md's "METHODOLOGY
CORRECTION" section for the full account.

`run-comparison.sh` answers "does adoption happen unprompted, prompt and
everything else held at their defaults." `run-hook-matrix-sweep.sh`
answers the actual follow-on question this benchmark exists for: given the
identical, tool-silent prompt, which combination of levers the **tool
author** controls actually changes that? There are exactly three such
levers today:

- **`mcp`** -- `ai-text-editor-mcp`'s own top-level MCP `instructions`
  pitch (shipped absent; set via `AI_TEXT_EDITOR_MCP_INSTRUCTIONS`, the
  same mechanism `chat-mcp` and `decisions-mcp` already use for their own
  `INSTRUCTIONS` consts).
- **`edit`** -- `editor-gate-plugin`'s existing Edit/Write PreToolUse hook
  reminder (shipped present; suppressed via `EDITOR_GATE_EDIT_WRITE_TEXT=""`).
- **`read`** -- `editor-gate-plugin`'s existing Read PreToolUse hook
  reminder (shipped present; suppressed via `EDITOR_GATE_READ_TEXT=""`).

```bash
cargo build --release -p ai-text-editor-mcp    # once, if not already built
benchmark/ai-text-editor-usage/run-hook-matrix-sweep.sh
```

Runs the *treatment* condition only (baseline has no tool to adopt), once
per combination -- `"none"`, `"mcp"`, `"edit"`, `"read"`, `"mcp+edit"`,
`"mcp+read"`, `"edit+read"`, `"mcp+edit+read"` -- the full `2^3` factorial,
8 real `claude -p` runs. Writes `results/<sweep-id>-leverfactorial/
<case-name>/treatment/` (same per-case layout as `run-comparison.sh`) and
`results/<sweep-id>-leverfactorial/hook-matrix-comparison.md`: adopted, raw
internal-vs-mcp edit counts (from `edit-timeline.json`, see below),
resolve-call count, mechanism-switch count, disk-change correlation, and
the usual correctness/outcome columns. `run-case.sh` itself takes an
optional 6th argument, `[levers]` (`"none"` or a `"+"`-joined subset of
`mcp`/`edit`/`read`; default `edit+read`, today's real shipped default),
which is what the sweep script drives -- pass one directly to try a
one-off combination without running the whole factorial. See `FINDINGS.md`'s
"Lever factorial" section for what the first pass found.

## What this does not cover (yet)

- Only one task shape and one model. A single run is a data point, not a
  trend — run it several times (varying `AI_TEXT_EDITOR_BENCH_MODEL`)
  before treating any single `comparison.md` as conclusive.
- No multi-turn / multi-file collision scenario, and no test of
  ai-text-editor's own `symbol`/`jump_points` (CodeGraph) addressing, since
  that needs an indexed project (`.codegraph/`) the tiny fixture doesn't
  have — a worthwhile follow-up case, not built here.
- `score.py`'s checks are regex/AST-based, not semantic review; a
  pathological-but-matching rewrite could pass them. Fine for this
  comparison's purpose (collision + token cost), not a substitute for a real
  review pass if this framework grows a second use case.
