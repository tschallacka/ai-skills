<!-- MODE: DEV -->
# Skill/MCP adoption benchmark — scenarios

Measures, for every skill and MCP tool this repo ships, whether a real
agent-under-test reaches for it **unprompted**, and whether reaching for it
(or not) produced a correct outcome. Two separate numbers per scenario cell,
never conflated:

- **Adoption rate** — did the agent-under-test use the target skill/binary/MCP
  tool at all, with no scenario text naming it.
- **Regression rate** — did the task end up wrong/unsafe regardless of
  adoption. Reported three ways: overall, adopted-only, and non-adopted-only
  — because "didn't adopt it" and "adopted it and misused it" are different
  failures, and collapsing them hides which one is actually happening.

This document is the scenario set, and it is prose rationale, not the
executable form. Each scenario below (S1-S20) has a machine-readable twin
under `scenarios/<id>/scenario.json` (+ `scenario.sh` for its
setup/verification) — `run-cell.sh SCENARIO_ID CLI MODE [REP]` runs one
cell end to end against those files. `PROTOCOL.md` is the exact procedure
`run-cell.sh` and its launched experimenter follow (the subject/
experimenter/orchestrator roles, the launch sequence, the env-stripping fix
and its unverified opencode analogue); `DATA-SPEC.md` is the storage and
reproducibility specification (schemas, directory layout, versioning, the
metrics `aggregate.py` computes). Keep this file and the `scenarios/`
manifests in sync when either changes — this file explains *why* a
scenario measures what it claims to, the manifest is what actually runs.

It assumes a separate steering subworker (model: Haiku, per the request
that commissioned this) that drives the `interactive-shell` skill against
two real interactive CLIs, `claude` and `opencode`. `interactive-shell`
itself and the `*-board` mods are **not** scenario targets — see
"Exclusions" below.

## Harness contract for the steering subworker

Read this whole section before running any scenario; it is the procedure,
not background.

1. **Never say the target's name.** Each scenario lists a `forbidden
   vocabulary` set — the skill name, its binary names, its MCP tool names,
   and the distinctive jargon from its own `SKILL.md` description. Neither
   the initial prompt nor any follow-up reply you send as "the user" may use
   those words, even if the agent-under-test asks a clarifying question
   using them first (echoing a word the agent itself introduced back to it
   is fine — introducing one yourself is not). If you cannot answer a
   clarifying question without using a forbidden word, answer "use your own
   judgment" and move on; do not steer.
2. **Never run a scenario against this repository's own checkout.** Every
   scenario runs inside a disposable workspace, never
   `/home/tschallacka/git/ai-skills` itself. That checkout's `BUGS.json`,
   `TODO.json`, `DECISIONS.json` are the real registers on the real
   `registers` branch — a scenario that calls `bugs add`/`todo add`/`decisions
   add` for real against them is not a benchmark, it's data corruption. Build
   each workspace as:
   ```bash
   WORK=$(mktemp -d "${WA_WORK_BASE:-$HOME/.cache/ai-skills-benchmark/skill-adoption}/cell.XXXXXXXXXX")
   # never a plain `mktemp -d` here -- that defaults to /tmp, a tmpfs
   # ramdisk on most Linux hosts, and a full concurrent pass holds many
   # complete HOME trees at once; see PROTOCOL.md's launch sequence, step 1.
   git init -q "$WORK/project"          # or seed from a scenario's fixture tree
   # install the full current skill collection fresh into $WORK, never into $HOME:
   /home/tschallacka/git/ai-skills/installer/bootstrap.sh install \
     --target "$WORK/home" --yes \
     --integration decisions=<skill|mcp> --integration chat=<skill|mcp> \
     --integration tailpipe=<skill|mcp> --integration ai-text-editor=<skill|mcp>
   ```
   then point `HOME`/`XDG_CONFIG_HOME`/`CLAUDE_CONFIG_DIR` (whichever the CLI
   under test reads) at `$WORK/home` before launching it, so the agent's
   registers, chat home, tailpipe home, and editor sessions all live inside
   the disposable workspace. Install the **whole collection**, not just the
   target skill — real usage has every skill available at once, and
   cross-skill interference (e.g. `question-etiquette` firing when
   `decisions` would have been the better call) is itself signal, not noise.
3. **One scenario cell = one disposable workspace, one CLI, one mode.** For
   the four dual-surface skills (`ai-text-editor`, `chat`, `tailpipe`,
   `decisions`), run the same scenario twice: once with that skill installed
   `--integration <name>=skill` (CLI/binary only) and once `=mcp` (typed
   tool calls). Every other skill has one mode.
4. Launch the CLI under test inside the workspace via `interactive-shell`:
   ```bash
   cd "$WORK/project"
   interactive-shell --session <scenario>-<cli>-<mode>-<rep> --cols 120 --rows 40 \
     --idle-timeout 180 -- env HOME="$WORK/home" <claude|opencode>
   ```
   Wait for the CLI's own ready prompt (`wait '<TEXT>'` on whatever that CLI
   shows at rest — a `>` prompt box for `claude`, the model/mode line for
   `opencode`) before sending anything.
5. Send the scenario's `prompt` verbatim with `paste` (not `text`, so no
   literal character is interpreted as a shortcut) followed by ENTER.
6. Poll with `view`/`view-delta` until one of: the CLI returns to an idle
   prompt with no pending tool call, 10 minutes wall-clock elapses, or the
   transcript shows the agent asking the user something — answer per rule 1,
   then keep polling. Do not inject anything else unprompted.
7. Pull the full scrollback once idle (`observe`, or `view 1-<rows>` across
   the session's recorded scrollback) and save it verbatim to
   `results/<scenario>-<cli>-<mode>-<rep>.transcript.txt` before touching
   the workspace further.
8. Score **adoption** by substring/regex match against the transcript using
   each scenario's `adoption signal` list — a literal binary invocation
   (`bugs add`, `ai-text-editor replace …`), an MCP tool name as the CLI's
   own UI renders a tool call (`mcp__decisions__add`, `Decisions.add`,
   `tailpipe_wait`, etc. — check both, CLIs render MCP calls differently),
   or, for the jargon-free "convention" scenarios, the literal behavioral
   marker named in that scenario (e.g. a line matching `^Q\d+[.)]` for
   `question-etiquette`).
9. Score **regression** by running that scenario's `verification` command
   against the workspace's final state — never by re-reading the
   transcript's own claims about what it did. A transcript that says
   "done, tests pass" is a claim; the verification command is the check.
10. Append one JSON line per run to `results/runs.jsonl`:
    ```json
    {"scenario":"S4-decisions","cli":"claude","mode":"mcp","rep":1,
     "adopted":true,"adoption_evidence":"mcp__decisions__add",
     "task_correct":true,"regression_notes":"","duration_s":94,
     "transcript":"results/S4-decisions-claude-mcp-1.transcript.txt"}
    ```
11. Tear down: `rm -rf "$WORK"`. Never leave a workspace behind, and never
    `git push`/network-publish anything a scenario created.
12. Default rep count: **2 reps per (scenario × cli × mode) cell** for a
    first full-collection pass (≈19 scenarios × 2 CLIs × up to 2 modes × 2
    reps ≈ 130 interactive runs). This is a real-model, real-wall-clock
    benchmark, not a unit test — budget accordingly, and run cells in
    parallel across separate workspaces rather than serially where the
    orchestrator allows it.

### Metrics (compute once `runs.jsonl` is complete)

For each scenario × cli × mode cell, and rolled up per skill:

- `adoption_rate` = adopted / total reps
- `regression_rate` = (NOT task_correct) / total reps — overall cost,
  independent of adoption
- `regression_rate_adopted` = (NOT task_correct AND adopted) / adopted —
  did using the tool itself cause the failure (wrong flag, wrong revision,
  misread schema)
- `regression_rate_unadopted` = (NOT task_correct AND NOT adopted) /
  non-adopted — the cost of reaching for stock tools instead

A skill whose `regression_rate_adopted` is noticeably higher than
`regression_rate_unadopted` is a documentation/ergonomics problem with the
skill itself, not a benchmark artifact — flag it by name rather than
averaging it away.

### Exclusions

- **`interactive-shell`** is the harness, not a target — testing it would
  be circular (the steering subworker's own driving of it is not a
  real-world "did the agent-under-test reach for this" decision).
- **The `*-board` mods** (`decision-board`, `chat-board`, `tailpipe-board`,
  `bugs-board`, `todo-board`, `plan-board`, `ci-board`, `brainstorm-board`,
  `register-board`, `tui-hint-board`, `boards`, `signal-bus`) are UI side
  effects of the skills above (several open automatically, e.g.
  `decision-board` on `decisions add`) rather than something a plain prompt
  causes an agent to independently reach for. Their adoption is implied by
  their owning skill's own MCP-mode scenario; they are not scored
  separately.
- **`www`**'s literal typed-trigger path is excluded — typing the word
  "www" to invoke it would itself be steering, by the skill's own
  definition. Only its *other* trigger (self-noticing thrashing) is tested
  (S19).
- **`rjq`**'s scenario measures a fallback choice, not a primary one — see
  S10.

---

## A. Dual-surface skills (CLI *and* MCP tool — run both modes)

### S1 — `ai-text-editor`

This skill already has a deep, mature dedicated benchmark
(`benchmark/ai-text-editor-usage/`, with its own fixtures, collision probes,
and wording sweep). Do not re-derive its task from scratch here. For the
interactive leg, reuse `benchmark/ai-text-editor-usage/task-prompt.md`
verbatim as the `prompt` and `fixtures/task_source.py` as the seeded file,
run it through the procedure above (both modes, both CLIs) instead of that
benchmark's own headless `claude -p` runner, and score adoption/regression
exactly as defined there (does it reach for the editor's typed verbs
unprompted; does it survive the external-edit/external-format collision
probes without silently clobbering them). Treat this cell as extending that
existing benchmark's coverage to the interactive surface, not a parallel
design.

- **Forbidden vocabulary**: ai-text-editor, open/insert/replace/large_edit
  (as verbs), revision guard, tab_id.
- **Adoption signal**: `ai-text-editor ` CLI invocation, or an MCP tool call
  named `open`/`read`/`insert`/`replace`/`search`/`save` in the editor's own
  namespace (not a bare shell `sed`/`python -c`).
- **Verification**: `benchmark/ai-text-editor-usage/score.py` against the
  resulting file.

### S2 — `chat` / `chat-mcp`

- **Setup**: start a real `chat-server-rs` inside the workspace. A plain
  script (not an LLM) using `chat-client-rs` joins channel `#proj` under
  nick `teammate-bot`, waits 45 seconds, then sends
  `@<agent's own nick> part 2 is done, the shared schema is now v3 — go ahead`.
  The agent-under-test's nick in this channel is whatever it registers as;
  if it never joins, the bot's message simply goes unread.
- **Prompt**: "I'm handing you part 1 of a two-part data migration
  (`migrate_users.py` in this repo). A colleague is doing part 2, the schema
  change, in their own session right now. Don't finish writing part 1's
  final commit until you've actually confirmed from them that the schema
  change landed — I don't want you guessing at the new shape. Go ahead and
  start on part 1 in the meantime."
- **Forbidden vocabulary**: chat, channel, nick, join, MCP, tail, IRC.
- **Adoption signal**: `chat-client-rs join`/`send`/`read`/`tail`, or an MCP
  tool call named `join`/`read`/`wait`/`send`/`who` in the chat adapter's
  namespace.
- **Non-adoption baseline**: the agent either stalls and asks the human
  "let me know when part 2 is ready" (a legitimate non-adoption outcome —
  score it `adopted:false`), or — the regression case — proceeds and
  guesses the schema anyway despite being told not to.
- **Verification**: inspect the final `migrate_users.py` for a hardcoded
  schema assumption written *before* the bot's message timestamp vs. after;
  `task_correct` requires the final code matches `v3` and was written (or
  confirmed) after the bot's send.

### S3 — `tailpipe` / `tailpipe-mcp`

- **Setup**: start a `tailpipe-server-rs` inside the workspace. A plain
  script pipes 200 lines of synthetic nightly-import log output (ending in
  `IMPORT OK: 48213 rows`) into a stream named `nightly-import`, finishing
  about 30 seconds after launch.
- **Prompt**: "Someone kicked off the nightly data import earlier and
  piped its output somewhere so other agents could check on it without
  re-running the whole thing. Before you touch the `orders` table, check
  whether that import actually finished and finished cleanly."
- **Forbidden vocabulary**: tailpipe, stream, ingest, snapshot.
- **Adoption signal**: `tailpipe-client-rs list`/`read`/`search`/`tail`, or
  an MCP tool call named `list_streams`/`read`/`search`/`wait` in the
  tailpipe adapter's namespace.
- **Non-adoption baseline**: the agent re-runs the import itself, or asks
  the human whether it finished — both are `adopted:false`, not failures by
  themselves.
- **Verification**: `task_correct` requires the agent's own stated
  conclusion ("import finished OK") to match the actual stream content, and
  requires it not to have re-run the import (check for a second invocation
  of the import script, which would itself be wasteful/wrong given the
  prompt's "without re-running it" framing).

### S4 — `decisions` / `decisions-mcp`

- **Setup**: a small HTTP client module with no existing retry logic.
- **Prompt**: "Add retry-with-backoff to the HTTP client in `client.py`.
  Pick whatever defaults seem reasonable — I might want different numbers
  later but I'm not going to be able to review that right now, so don't
  wait on me, just make a sensible call, note what you picked and why
  somewhere I'll see it, and keep going."
- **Forbidden vocabulary**: decisions, non-blocking, stub, lettered option,
  register.
- **Adoption signal**: `decisions add` CLI invocation, or an MCP tool call
  named `add` in the decisions adapter's namespace.
- **Non-adoption baseline**: the agent asks inline in chat instead (a
  `question-etiquette`-shaped numbered question) despite being told not to
  wait — that is `adopted:false` for this scenario, and arguably a partial
  regression against the explicit "don't wait on me" instruction; note it
  rather than silently folding it into plain non-adoption.
- **Verification**: `task_correct` requires retry logic actually present in
  `client.py` with the work having continued past this point (a later
  commit/file touched after the retry code), i.e. the agent did not in fact
  block.

---

## B. CLI-only skills (one mode — skill, no MCP surface)

### S5 — `bug-report`

- **Setup**: a small module where calling `parse_config(None)` raises an
  unhandled `TypeError` — real, reproducible, not simulated.
- **Prompt**: "While you're adding the new `timeout` field to
  `config.py`, you'll probably notice `parse_config` blows up on a `None`
  input. Don't fix that right now, it's out of scope for this change — just
  make sure it doesn't get lost, and move on to the timeout field."
- **Forbidden vocabulary**: bug-report, bugs, defect register, reproduction.
- **Adoption signal**: `bugs add` CLI invocation against the **workspace's**
  `BUGS.json` (never the real repo's).
- **Verification**: `task_correct` requires the `timeout` field actually
  landed in `config.py`, and that the `TypeError` is mentioned *somewhere*
  durable in the workspace (a `BUGS.json` entry, at minimum — a comment
  alone is a partial, not full, pass; note which it was).

### S6 — `todo`

- **Prompt**: "Before you start, here's today's list: rename the `utils`
  module to `helpers` everywhere it's imported, add type hints to
  `parser.py`, and write a README section on the new CLI flag. Go in that
  order."
- **Forbidden vocabulary**: todo, queue, register, sub-task.
- **Adoption signal**: `todo add` CLI invocation **and** the agent asking
  the human first (per the skill's own documented rule: never auto-file
  without asking). Score `adopted:true` only if both happened; score it a
  *separate* regression note (not folded into `task_correct`) if it filed
  without asking.
- **Verification**: `task_correct` requires all three items actually done,
  in the stated order, by end of run.

### S7 — `ci-failures`

- **Setup**: the workspace's own throwaway repo with a GitHub remote is not
  realistic to fake; instead seed a local file `ci-status.txt` the scenario
  script writes to simulate `gh run view`-shaped output for a run that
  failed one job with a real stack trace in it, and stub `gh`/`glab` in
  `$PATH` inside the workspace to read from that file instead of the
  network. (If stubbing both forges is too heavy for a first pass, run this
  scenario GitHub-only and mark GitLab `UNCONFIGURED` rather than skip it
  silently.)
- **Prompt**: "The last push's CI run is red. Can you find out what
  actually broke?"
- **Forbidden vocabulary**: ci-failures, run id, pipeline, job log.
- **Adoption signal**: invocation of `ci-failures.sh`.
- **Non-adoption baseline**: ad hoc `gh run view`/`gh api` calls assembled
  by hand — legitimate non-adoption.
- **Verification**: `task_correct` requires the agent to correctly name the
  failing job and quote the real stack-trace line from the stub, not a
  guess.

### S8 — `resource-limited-testing`

- **Setup**: a crate/test suite in the workspace that is deliberately slow
  and memory-heavy (e.g. a test that allocates a large buffer and sleeps),
  real enough that an unbounded run is a genuine risk, not just a prompt
  claiming it is.
- **Prompt**: "Run the full test suite for this crate before you say we're
  done — it's known to be a heavy one, it's eaten the whole machine before."
- **Forbidden vocabulary**: resource-limited-testing, resource cap,
  limited-run.
- **Adoption signal**: invocation routed through
  `resource-limited-testing/scripts/limited-run.sh` (or equivalent cgroup/
  ulimit wrapping) rather than a bare `cargo test`/`pytest`.
- **Verification**: `task_correct` requires the suite to have actually run
  to completion (not just been wrapped) — a wrapper that was invoked but
  whose limits killed the run early and was then silently reported "done"
  is a regression, not a pass.

### S9 — `rjq`

- **Setup**: a workspace where `jq` is deliberately absent from `PATH`
  (confirm with `command -v jq` returning nonzero before launching the
  agent), containing a moderately nested JSON file to filter.
- **Prompt**: "Pull every `id` where `status == "failed"` out of
  `results.json` and list them."
- **Forbidden vocabulary**: rjq, jq-compatible.
- **Adoption signal**: invocation of the shipped `rjq` binary.
- **Non-adoption baseline**: a hand-rolled Python/awk parse — legitimate,
  since this scenario measures a *fallback* choice (rjq's own trigger is
  explicitly "when jq is not installed"), not a primary one. A run where
  `jq` turns out to still be reachable some other way is not a valid rep —
  re-verify the `PATH` setup rather than scoring it.
- **Verification**: `task_correct` requires the exact right id list.

---

## C. Convention skills (no binary — adoption is a behavioral marker)

### S10 — `question-etiquette`

- **Prompt**: "Design the caching layer for this service. There are at
  least two real open questions in here — what to use as the eviction
  policy, and whether entries should be namespaced per tenant — go ahead
  and ask me whatever you need before you commit to an approach."
- **Forbidden vocabulary**: question-etiquette, numbered, lettered.
- **Adoption signal**: transcript contains lines matching `^Q\d+[.)]` with
  sub-items matching `^\s*[a-z][.)]`, and a final escape option per
  question ("something else", "tell me", or equivalent free text).
- **Verification**: `task_correct` requires every raised question to
  actually be answerable from the transcript (no bare unlabeled bullets
  standing in for a question).

### S11 — `decisions` cross-check (persistence of a question)

Reuses S4's setup but phrased to test the *question-etiquette →
decisions* hand-off specifically, now that the two skills are cross-linked:

- **Prompt**: "I'm going to be away from the keyboard for a while after
  this. If you hit something you'd normally ask me about while building
  the retry logic in `client.py`, don't just ask in the chat — I won't see
  it. Make sure it's somewhere I'll actually find it when I'm back, then
  keep going."
- **Forbidden vocabulary**: decisions, register, persist.
- **Adoption signal**: `decisions add`/`mcp__decisions__add`, specifically
  *instead of* (or in addition to) an inline chat question.
- **Verification**: same as S4.

### S12 — `text-etiquette`

- **Setup**: mid-way through any other multi-step scenario in this set,
  the steering subworker asks one short factual question as "the user":
  "quick — is the migration branch rebased on main yet?"
- **Adoption signal**: the reply to that one line is itself short and
  direct (roughly ≤2 sentences, no preamble, no restated question) rather
  than a multi-paragraph recap.
- **Verification**: this is a style conformance check, not a correctness
  one — record the reply length/shape, no `task_correct` gate.

### S13 — `merge-request-etiquette`

- **Setup**: a small, real, isolated bugfix already made on a branch.
- **Prompt**: "Open a PR for this fix."
- **Forbidden vocabulary**: merge-request-etiquette, TLDR.
- **Adoption signal**: the PR body opens with a one-line summary before any
  detail section, and has no apologetic/padded preamble.
- **Verification**: `task_correct` requires the PR to actually exist and
  correctly describe the real diff (not a generic restatement).

### S14 — `project-specifics`

- **Setup**: plant `.agents/knowledge/deviations-example.md` in the
  workspace describing one concrete, checkable local quirk (e.g. "this
  project's `build.sh` requires `NODE_OPTIONS=--max-old-space-size=4096` or
  it OOMs silently and exits 0 anyway").
- **Prompt**: "Run the build and make sure it actually succeeded."
- **Forbidden vocabulary**: project-specifics, deviations, quirk.
- **Adoption signal**: the agent sets the documented env var (or otherwise
  routes around the documented quirk) before/while invoking `build.sh`.
- **Verification**: `task_correct` requires it to correctly report build
  success/failure — the deliberately-planted trap (silent exit 0 on OOM)
  means a non-adopting agent will confidently report success on a build
  that actually failed, which is the regression this scenario is built to
  surface.

### S15 — `brainstorm`

- **Prompt**: "Redesign how this service handles caching. I don't have
  strong opinions yet, just want it to be better."
- **Forbidden vocabulary**: brainstorm, pre-planning.
- **Adoption signal**: the agent runs a short back-and-forth narrowing the
  space (asking about constraints, options, tradeoffs) *before* committing
  to a plan or writing code, rather than immediately producing an
  implementation.
- **Non-adoption baseline**: immediately writes code against a guessed
  design — `adopted:false`, and likely also `task_correct:false` if the
  guess conflicts with constraints revealed only once asked.

### S16 — `planning`

- **Prompt**: "This migration is going to take several sessions over the
  next few weeks — I'll be in and out, and it's likely someone else picks
  up where I left off at some point. Get started."
- **Forbidden vocabulary**: planning skill, durable, resumable.
- **Adoption signal**: the agent creates an on-disk plan directory with
  separate files (goals/steps/progress) rather than only a chat-level
  checklist.
- **Verification**: `task_correct` requires the plan artifacts to actually
  exist on disk and be internally consistent (steps reference real files in
  the workspace).

### S17 — `post-implementation-review`

- **Setup**: give the agent a genuinely nontrivial, multi-file
  implementation task (several interacting functions, at least one subtle
  edge case) with no mention of review at all.
- **Adoption signal**: once the implementation is reported done, the agent
  *itself* proposes or performs a review pass (naming specific risks or
  spawning a check) without being asked.
- **Verification**: `task_correct` requires the review (if any) to surface
  a real issue if one was deliberately seeded in the task, or to correctly
  report none if none was.

### S18 — `git-worktrees`

- **Prompt**: "I want to try two different approaches to this refactor at
  the same time, on this repo, without one messing up the other while I
  compare them."
- **Forbidden vocabulary**: worktree.
- **Adoption signal**: `git worktree add` invoked, vs. branch-switching in
  place in the single checkout.
- **Verification**: `task_correct` requires both approaches to end up
  independently inspectable/buildable at the same time (branch-switching in
  place would make this impossible without discarding one).

### S19 — `www` (implicit trigger only)

- **Setup**: a task whose only path to success requires noticing that a
  command is subtly lying (e.g. a wrapper script that always exits 0
  regardless of the real test result, so re-running the same failing fix
  attempt "passes" every time).
- **Prompt**: "This test keeps failing on this one assertion — can you fix
  it?" (the test in question is the one behind the always-exits-0 wrapper).
- **Forbidden vocabulary**: www, thrashing.
- **Adoption signal**: the transcript shows the agent explicitly stopping
  and re-grounding ("what do we actually have / what are the real values /
  what are we trying to achieve" or equivalent) after at least one failed
  retry, rather than repeating near-identical fix attempts three or more
  times in a row.
- **Verification**: `task_correct` requires the agent to eventually notice
  the wrapper always exits 0 and stop trusting it, not just eventually
  produce code that happens to pass the real assertion by luck.

### S20 — `git-merge-resolving`

- **Setup**: two real branches off a common base, each independently
  editing different parts of the same function, including one hunk that is
  a genuine union (both branches add an independent guard clause to the
  same block) rather than a true either/or choice.
- **Prompt**: "Merge `feature-a` into `feature-b`."
- **Forbidden vocabulary**: git-merge-resolving.
- **Adoption signal**: the agent reads both branches' history/intent
  (`git log`/`git show` on both sides) before resolving, rather than
  immediately picking `--ours`/`--theirs` wholesale.
- **Verification**: `task_correct` requires the union hunk to actually
  contain both guard clauses after resolution, not just one side's.
