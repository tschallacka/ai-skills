<!-- MODE: DEV -->
# Protocol

How one cell — one `(scenario, cli, mode, rep)` — actually gets run. This
generalizes `benchmark/ai-text-editor-usage/LAB-PROTOCOL.md`'s proven
subject/experimenter/orchestrator split across every scenario in
`SCENARIOS.md`/`scenarios/`, two CLIs instead of one, and up to two install
modes instead of a treatment/baseline pair. Read that file first if you
haven't — this one assumes its vocabulary and does not re-derive it.

## Roles

- **The subject** — the agent-under-test. A genuine interactive `claude` or
  `opencode` session inside a real PTY (`interactive-shell`), with the
  scenario's own disposable workspace as its working directory and a
  disposable `HOME` holding a fresh install of the full skill collection at
  the cell's mode.
- **The experimenter** — a real Agent-tool subagent of the orchestrating
  Claude Code session, `model: haiku`, given one cell's full brief
  (`make-experimenter-brief.py`'s output) as its prompt. Drives the subject
  exactly as a human operator would: launches it, pastes the scenario
  prompt, answers any clarifying question under the forbidden-vocabulary
  constraint, clears permission prompts, detects completion, runs the
  scenario's own verification script, and writes one JSON bundle matching
  `schema/run-record.schema.json` to disk (the brief's own Step 6) — this is
  the graded artifact, not the subagent's conversational reply. This is the
  "Haiku subworker." **Never a `claude -p` subprocess** — see the note below.
- **The orchestrator** — the live Claude Code session itself, not a detached
  shell script. `prepare-cell.sh` builds the workspace, installs the skill
  collection, captures the environment, and writes the brief; the
  orchestrating session then launches the experimenter as an Agent-tool
  subagent and waits for it to finish; `finish-cell.sh WORK_DIR` then
  validates the bundle the experimenter wrote, hands it plus
  orchestrator-known fields to `record-run.py`, writes the registry entry,
  and tears the workspace down. See "Orchestrating a full pass" below for
  how many cells run at once and who assigns them.

## Why the experimenter is a real subagent, not a prompt template

Neither `prepare-cell.sh`/`finish-cell.sh` nor a plain prompt template can
drive the subject's TUI — recognizing an idle prompt, a permission dialog,
or a subject's clarifying question needs actual judgment, which is exactly
what the experimenter role exists to supply at low cost.

**`claude -p` must never be invoked as the experimenter.** An earlier
version of this protocol shelled out to `claude -p --model haiku
--output-format=json --permission-mode bypassPermissions` as a detached
subprocess, one per cell, orchestrated by a plain bash script
(`full-pass.sh` + the since-removed `run-cell.sh`). A full pass under that
design (2026-10-19) lost roughly half its cells to a silent kill: each
failing log ends cleanly right after "launching experimenter", with zero
error text, no JSON, no shutdown trace — consistent with something external
killing the whole `claude -p` process tree outright. Rate limiting, OOM, and
cgroup limits were all checked and ruled out; nested headless `claude`
invocation is the remaining explanation and is now simply avoided rather
than chased further. The experimenter is instead a real Agent-tool subagent
of the *orchestrating* session (this document's launch sequence is
otherwise unchanged — same brief, same steps, same bundle shape), which
needs no subprocess invocation of `claude` at all.

## Launch sequence for one cell

1. **Setup** (orchestrator, `prepare-cell.sh`):
   - `run_id = uuid4()`.
   - `WORK` is created by `lib/common.sh`'s `wa_new_workspace`: `mktemp -d`
     under a disk-backed base (`${WA_WORK_BASE:-$HOME/.cache/ai-skills-benchmark/skill-adoption}`),
     **never `/tmp`** -- on a typical Linux desktop (including this harness's
     own dev host) `/tmp` is a tmpfs ramdisk, and a concurrent full pass holds
     many complete `claude`/`opencode` HOME trees at once; that is RAM, not
     disk. `mkdir "$WORK/project" "$WORK/home"`. Unlike the old single-script
     design, `WORK` now outlives `prepare-cell.sh` itself -- the experimenter
     subagent still has to read and write inside it -- so cleanup is
     `finish-cell.sh`'s job (`rm -rf "$WORK"` once the bundle is recorded), not
     a `trap ... EXIT` here. A crashed or abandoned cell (the experimenter
     subagent never finishes, or the orchestrating session is interrupted)
     still leaves a stale `cell.*` dir behind, exactly as before; that is
     what `wa_reap_stale_workspaces` sweeps before a fresh pass starts.
   - Run the scenario's `scenario.sh setup "$WORK/project"` if present.
   - Build (or confirm already built) the installer binary from this
     checkout, then, with `HOME="$WORK/home"`:
     ```
     installer install --source "$AI_SKILLS_REPO_ROOT" --package dev \
       --agent <claude|opencode> --all --yes \
       --integration decisions=<mode> --integration chat=<mode> \
       --integration tailpipe=<mode> --integration ai-text-editor=<mode> \
       --integration interactive-shell=skill
     ```
     `--all` installs the whole collection (per `SCENARIOS.md`'s harness
     contract, point 2 — real usage has every skill available, not just the
     target). Only the four dual-surface skills take an explicit
     `--integration`; every other skill has one mode and needs none.
     `interactive-shell` is pinned to `skill` mode regardless of the cell —
     it is the harness, not a target (`SCENARIOS.md` Exclusions), and the
     subject itself never drives it.
   - Capture `environment` (schema above): `git rev-parse HEAD` and `git
     status --porcelain` (for `worktree_dirty`) in `$AI_SKILLS_REPO_ROOT`,
     `claude --version`/`opencode --version`, `uname -a`, `hostname`.
   - Write a `registry.jsonl` line with `outcome: "abandoned"` placeholder —
     overwritten (append a corrected final line; `registry.jsonl` is
     append-only, so "overwrite" means the *last* matching `run_id` line
     wins when read back, not a literal in-place edit) with the real
     outcome once the cell finishes, so a crash leaves a trace either way.
2. **Brief**: `make-experimenter-brief.py <scenario.json> <cli> <mode>
   <WORK/project> <WORK/home> <run_id>` renders the experimenter's full task
   text: the fixed procedure below, the scenario's `prompt`, its
   `forbidden_vocabulary`, its `adoption_signals`, and the exact shell
   command to launch the subject (step 3).
3. **Launch** (experimenter, told to run this verbatim): an interactive
   `bash -i` under the disposable `$WORK/home`, never the subject CLI
   directly -- two problems solved by the one choice:
   - **PATH for this cell's own skill binaries** (`ai-text-editor`,
     `chat-client-rs`, `tailpipe-client-rs`, ...). The installer
     (`wa_install_skills`) already writes a PATH-prepend guard at the top of
     this disposable HOME's own `.bashrc`/`.zshenv`/`.profile`
     (`src/installer/src/path_env.rs`'s `ensure_shared_bin_on_path`,
     answered "yes" by the install's own `--yes`) -- the same mechanism a
     real `~/.bashrc` uses on a real machine. `bash -i` is what actually
     sources it; the subject CLI launched directly (an earlier version of
     this protocol) never did, and had no PATH-independent way to find
     those binaries.
   - **Working directory.** `interactive-shell` has no `--cwd` of its own --
     the command after `--` is forked directly (never through a shell), so
     it inherits whatever cwd `interactive-shell` itself happened to start
     in (the experimenter's own Bash-tool cwd), never `$WORK/project`, with
     nothing else asking for it otherwise. Measured live (2026-10-19): a
     subject launched directly this way ran entirely inside the
     orchestrator's own repo checkout and left real files there (a
     fabricated `client.py`, a real entry in the repository's own
     `DECISIONS.json`) -- see `git log`/this file's own history around that
     date for the cleanup. `cd` as the shell's own first command, below,
     fixes this the same way a human operator would.
   ```bash
   # claude subject -- env stripping per LAB-PROTOCOL.md's confirmed fix
   # (run 4 of that protocol's validation; see its "Known limitations").
   interactive-shell --session <run_id> --cols 120 --rows 40 --idle-timeout 180 -- \
     env -u CLAUDE_CODE_CHILD_SESSION -u CLAUDE_CODE_SESSION_ID \
         -u CLAUDE_CODE_SESSION_ATTENDED -u CLAUDE_CODE_BRIDGE_SESSION_ID \
         -u CLAUDE_CODE_MESSAGING_SOCKET -u CLAUDE_CODE_MESSAGING_TOKEN \
         HOME="$WORK/home" bash -i
   ```
   (`opencode` is identical, just without the `claude`-specific `-u` flags.)
   A scenario that also names `requires_path_without` (S09's jq) still
   overrides `PATH` ahead of `bash -i` with the symlink farm that stands in
   for it with that one binary excluded -- see `make-experimenter-brief.py`'s
   `path_without` -- since `.bashrc`'s own guard only ever *prepends* the
   shared bin dir, never removes anything.

   Wait for a plain bash prompt, then -- typing into that SAME
   interactive-shell session -- run, in order, waiting for the prompt to
   return after each:
   ```bash
   cd "$WORK/project"
   claude --session-id <subject-uuid> --permission-mode acceptEdits   # or: opencode
   ```
   Clear a one-time trust/onboarding dialog if shown (plain-text grep,
   arrow/ENTER), then wait for a genuinely idle, focused input box before
   proceeding.
4. **Briefing**: paste the scenario's `prompt` verbatim (not typed
   character-by-character) and submit. This is the only task instruction
   the subject ever receives; the experimenter adds nothing and never
   coaches tool choice. If the subject's own first turn asks a clarifying
   question, answer it under rule 1 below.
5. **Observation**: watch the screen. Handle any permission-prompt dialog
   (Bash approval, MCP-tool approval) by selecting the affirmative option —
   recorded in `permission_prompts_handled`. Do not nudge, hint, or
   otherwise intervene beyond answering a direct question from the subject.
6. **Completion detection**: the subject goes idle with no pending tool call
   and (where the scenario defines one) states it is done. If 10 minutes
   wall-clock pass with no idle state, treat it as `timed_out` and proceed
   to step 7 anyway — a timeout is itself a data point (`adopted` and
   `task_correct` are both still computable from whatever state exists).
7. **Verification**: before running the scenario's own `scenario.sh verify`,
   always write two files to `"$WORK/project"`, unconditionally — cheap, and
   it means a scenario's own `verify` never has to coordinate with the
   procedure about which one it will need:
   - `.subject-conclusion.txt` — the subject's own final summary message,
     **verbatim**. Used by a scenario whose correctness depends on whether
     the subject's *stated conclusion* matches independently-observed fact
     (S03, S07, S09).
   - `.transcript.txt` — the full saved scrollback observed so far,
     verbatim. Used by a `category: convention` scenario (S10-S20), whose
     adoption/correctness signal is a behavioral marker in the conversation
     itself (a numbered-question format, a short reply, a review offered
     unprompted) rather than anything a filesystem check alone could see.

   Then run `scenario.sh verify "$WORK/project"` and capture its one-line
   JSON (`{"task_correct":bool,"notes":str}`) and exit code verbatim. A
   scenario's `verify` reads whichever file(s) its own check needs and
   ignores the other; a scenario needing only world-state checks (S01,
   S04-S06, S08) is unaffected by either file's presence. **Never**
   otherwise substitute the subject's own closing summary for the
   verification step, and never let `verify` accept a claim of success
   found in either file at face value without checking it against
   something independent (the stream content, the actual file state, the
   literal regex a scenario names) — that is exactly the distinction
   `SCENARIOS.md`'s harness contract step 9 exists to enforce. The files
   exist so a scenario can check *content* against independently-observed
   fact, never so it can trust a verdict written in them.
8. **Scoring adoption**: grep the saved transcript against the scenario's
   `adoption_signals` for the cell's mode (`skill`/`mcp`/`behavioral`).
   Record the first match as `evidence`/`signal_matched`; `adopted:false`
   if none match.
9. **Bundle**: write one JSON object matching
   `schema/run-record.schema.json` to `$WORK/bundle.json` (the graded
   artifact) and also send it as the experimenter's final message.
10. **Wrap-up** (orchestrator, `finish-cell.sh WORK_DIR`): read
    `$WORK/bundle.json`, call `record-run.py` (validates against the schema,
    merges in the orchestrator-known `environment` fields captured at setup,
    appends to `results/runs.jsonl`, copies the transcript into
    `results/transcripts/<run_id>.txt`, updates the `registry.jsonl` line's
    outcome to `completed`), then `rm -rf "$WORK"`.

## Orchestrating a full pass

`full-pass.sh` (plain bash, `wait -n`-gated concurrency) is what a
`claude -p`-subprocess experimenter could run under unattended. It no longer
applies to the claude-subject cells, because there is no subprocess to
launch — the experimenter is a subagent of the *orchestrating session
itself*, and only that session can start one (the Agent tool, not a shell
command). A full pass is therefore driven interactively: the orchestrating
session partitions the cell matrix into N lanes, launches one Agent-tool
subagent per lane (`model: haiku`, a fresh general-purpose subagent — it
needs only Bash, Read and Write), and each lane subagent works through its
own cells strictly one at a time (`prepare-cell.sh` → follow the brief →
`finish-cell.sh`), never two cells concurrently within a lane. Lane count is
the real concurrency knob now, in place of `full-pass.sh`'s own
`CONCURRENCY` argument. `aggregate.py` is unchanged and still the last step
once every lane reports back.

## Rule 1 (forbidden vocabulary) — restated from `SCENARIOS.md`

Neither the pasted prompt nor any reply the experimenter sends afterward may
contain a scenario's `forbidden_vocabulary` entry (case-insensitive
substring), even in answer to a subject's own question that used the word
first. If an honest answer requires a forbidden word, answer "use your own
judgment" instead. `make-experimenter-brief.py` statically checks the
rendered `prompt` against `forbidden_vocabulary` and refuses to build a
brief that already fails this — a prompt-authoring bug caught before any
model time is spent on it.

## Known limitations and unverified assumptions

Stated as what has and has not actually been checked, in
`LAB-PROTOCOL.md`'s own style — a hypothesis is marked as one until a real
run confirms or refutes it, never asserted as settled in advance.

- **The `claude` subject env-stripping fix is confirmed** (not re-derived
  here): `LAB-PROTOCOL.md`'s run 4 showed `env -u CLAUDE_CODE_CHILD_SESSION`
  (plus the four related vars) makes a nested nested `claude` subject's
  `--session-id` resolve to a real, resumable transcript. This protocol
  reuses that fix verbatim rather than re-deriving it, on the assumption
  that whatever caused it (the orchestrator's own shell already carrying
  those vars because the whole benchmark typically runs from inside a
  Claude Code session) applies identically here. Re-verify this holds if
  this benchmark is ever launched from a context that does *not* already
  carry those vars (e.g. a bare CI shell) — stripping variables that were
  never set is harmless, but the premise that they need stripping at all
  should be re-checked rather than assumed to transfer.
- **The analogous fix for `opencode` is NOT confirmed.** Nothing in this
  repository has yet measured whether `opencode` has an equivalent
  nested-session identity problem, let alone which environment variable(s)
  would need stripping if it does. The first live `opencode` cell run under
  this protocol should include an explicit resume/session-identity check
  (mirroring `LAB-PROTOCOL.md`'s `resume-probe.json`) and the result
  (confirmed-fine, confirmed-needs-a-fix-and-here-it-is, or
  confirmed-unresolvable) belongs back in this section, replacing this
  paragraph, not appended alongside it.
- **The installer invocation in step 1 is unverified end-to-end against a
  disposable `HOME`.** It follows directly from `installer --help`'s
  documented `(--target DIR | --agent NAME)` + `--all` +
  `--integration SKILL=MODE` surface and T72's "binaries land under `$HOME`
  regardless of `--target`" comment in `src/installer/src/main.rs`, but no
  run of this benchmark has yet executed it for real. The first live cell
  for *each* `(cli, mode)` combination should confirm the install actually
  produces a working skill set (the subject's own first turn reaching for
  *anything* skill-shaped is itself weak evidence of this; a cell where the
  subject never adopts anything across its first several reps is worth
  checking for an install failure before it is scored as a true negative).
- **The experimenter's final-message-is-the-bundle convention** mirrors
  `LAB-PROTOCOL.md`'s own working pattern, but `record-run.py` still
  defensively looks for the last well-formed JSON object in the
  experimenter's `result` field rather than assuming the whole field parses
  cleanly, since a Haiku-tier model occasionally wraps JSON in prose despite
  being told not to — log (not silently drop) any bundle that needed this
  fallback, as a signal the brief's own instruction wording may need
  tightening.
- **10-minute timeout** (step 6) is a starting guess, not a measured
  figure. Scenarios with a background process on a fixed delay (S02's
  45-second chat message, S03's ~30-second tailpipe fill) are comfortably
  inside it; scenarios where the subject itself runs a real test suite
  (S08) may not be — widen per-scenario once real durations are observed,
  rather than widening the global default defensively in advance.
