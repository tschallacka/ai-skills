# The lab: an experimenter-driven interactive run (`run-case.sh ... lab`)

## Why this exists

Every other launch mode here (`print`, the default) runs the worker
headlessly via `claude -p --permission-mode bypassPermissions`. That is
reliable -- it reports its own session id and token usage directly in its
JSON stdout -- but it buys that reliability by giving up two things a real
human-driven session has: genuine permission prompts (bypassed outright, so
the Bash-approval dimension around the shipped test run has never actually
been exercised -- see `run-case.sh`'s own comment on this), and any way to
ask the worker about itself afterward.

`lab` mode restores both, without touching the parts that already work. It
does not replace the event-driven collision triggers or the CSV-checkpoint
task design (`task-prompt.md`, `external-edit.sh`, `external-format.sh`) --
those are launch-mode-independent and run unchanged underneath it.

An earlier attempt this session tried to have the *orchestrator itself*
drive an interactive worker directly, using a cheap model to classify screen
state. That hit a real, reproducible Claude Code platform limitation: the
worker's own transcript kept being attributed to the orchestrating session
rather than to the worker's own pinned `--session-id`, even after clearing
the obvious inherited env var (see `run-case.sh`'s header comment for the
full account; nine real runs were spent confirming it before abandoning that
mechanism). `lab` mode was originally designed to sidestep that problem
rather than re-solve it -- scoring and the bundle never needed the subject's
transcript file, only the live screen and hook-based telemetry.

**That problem has since actually been root-caused and fixed, confirmed by
a live run (see "Known limitations").** `CLAUDE_CODE_CHILD_SESSION=1` --
present in this whole benchmark's own shell environment because it is itself
normally run from inside a Claude Code session -- was inherited down through
the experimenter into the subject's nested `claude` process, and that
specifically is what disabled the subject's transcript persistence. The
earlier investigation never tried unsetting this exact variable (only
`CLAUDE_CODE_SESSION_ID`/`CLAUDE_CODE_SESSION_ATTENDED`, which made things
worse). Stripping it (and four related identity vars) with `env -u` for the
subject process alone makes its pinned `--session-id` resolve to a real,
complete transcript and makes `claude --resume <that uuid>` genuinely
continue that exact session -- tested live, not assumed. Lab mode keeps its
original design (it still doesn't *need* the transcript for scoring or the
bundle), but the specific ask that motivated `/rename` and the registry --
being able to find and resume a past subject later -- is now actually
achieved by the plain pinned session id, not merely attempted.

## Roles

- **The subject** -- the worker actually under test. Launched as a genuine
  interactive `claude --session-id <pinned-uuid>` session inside a real PTY
  (via `interactive-shell`), with `--permission-mode acceptEdits` (file edits
  pre-approved; Bash and MCP-tool approval prompts are real and must be
  answered by the experimenter, exactly as a human operator would answer
  them).
- **The experimenter** -- a separate `claude -p --model haiku
  --output-format=json --permission-mode bypassPermissions` process. This is
  the "Haiku subworker": it drives the subject's interactive session as a
  human would (clears onboarding dialogs, types the task prompt, answers
  permission prompts, watches for completion, runs the fixed debrief, writes
  one JSON bundle), and because it itself runs in `-p` mode, its own session
  id and token usage are directly and reliably available -- the same
  reliability `print` mode has always had, just relocated from the subject
  to the experimenter.
- **The orchestrator** -- `run-case.sh` itself. Builds the workspace and the
  lever environment exactly as `print` mode does, starts the two background
  collision scripts exactly as `print` mode does, launches the experimenter,
  waits for its bundle, then runs the same scoring pipeline (`score.py`,
  `extract_edit_timeline.py`) every other mode uses. It also runs the resume
  probe (below) and writes the subject's registry entry.

## Procedure, phase by phase

1. **Setup** (orchestrator). Workspace, lever env vars, MCP config (treatment
   only), background `external-edit.sh` / `external-format.sh` -- identical
   to `print` mode. A session uuid is generated and a human-readable label
   (`programmer1`, `programmer2`, ...) is assigned from the running count in
   `results/subjects-registry.jsonl`.
2. **Launch** (experimenter). Starts `interactive-shell --session
   <label> -- claude --session-id <uuid> ...` with no prompt argument.
   Clears the one-time "trust this folder" dialog if shown (plain-text grep,
   DOWN, ENTER). Waits for the input box to be genuinely idle and focused.
3. **Rename** (experimenter). Types `/rename <label>` and presses ENTER,
   before anything else happens in the session. `--session-id` is not
   reliably the id the session's transcript ends up stored under for this
   launch shape (see "Known limitations"); `/rename` sets a durable display
   name independent of that, which is the one thing that might let this
   session be found again later by name even when its id does not resolve.
   Whatever the experimenter actually sees (a confirmation, or nothing
   visible at all) is recorded, not assumed, as `session_rename_confirmed` /
   `session_rename_evidence` in the bundle.
4. **Briefing** (experimenter). Pastes the byte-identical contents of
   `task-prompt.md` into the input box (`paste`, to avoid the embedded
   newlines triggering early submission) and presses ENTER to submit. This
   is the one and only task instruction the subject ever receives -- the
   experimenter adds nothing to it and never coaches the subject on tool
   choice.
5. **Observation**. The experimenter watches the screen, answering any
   permission-prompt dialog that appears (Bash command approval for the test
   run; an MCP-tool approval prompt on the subject's first `ai-text-editor`
   call, treatment only) by selecting the affirmative/allow option, and
   noting anything that looks worth a closer look later (a hesitation, a
   retry, an unusual tool switch) as free-text observations. It does **not**
   trigger the collisions itself -- those already fire on their own via the
   background scripts the moment `tax_rates.csv` / `departments.csv`
   appears; the experimenter only reads the resulting log timestamps into
   its report afterward.
6. **Completion detection**. The task ends with the subject printing a
   one-line summary and going idle (per `task-prompt.md`'s own closing
   instruction). The experimenter waits for that: test output visible,
   followed by a short summary line, followed by an idle, empty, focused
   input box with no spinner.
7. **Debrief**. The fixed four questions from `debrief-script.md`, asked one
   at a time inside the still-open session, plus at most one investigative
   follow-up if something stood out during observation.
8. **Wrap-up**. The experimenter types `/cost`, reads the resulting on-screen
   report verbatim (this is how token/cost data is captured -- never by
   resolving the subject's own transcript file, which is exactly the thing
   the earlier attempt could not reliably do), then calls `shutdown` on the
   interactive-shell session and writes its one JSON bundle.
9. **Resume probes** (orchestrator, after the experimenter exits). Two
   ordinary `claude -p --resume ... --output-format=json` calls, each asking
   the same question whose correct answer is only derivable from the
   subject's own session (its own choice of `departments.csv`'s Support
   budget, 80000) -- one keyed on the pinned `--session-id`
   (`resume-probe.json`), one keyed on the `/rename`d label itself
   (`resume-probe-by-label.json`), since `--resume`'s own `--help` text says
   a bare value can be "a session ID, or ... a search term". Both are the
   actual, empirical test of whether either handle can reach that session
   later -- not an assumption. Logged regardless of outcome, including
   failure, since a failure here is exactly as informative as a success.
10. **Scoring**. Unchanged: `score.py` against the workspace, with a
   `worker-stdout.json` synthesized from the bundle's `final_summary` field
   so the existing `detect_feedback_request` path needs no lab-specific
   branch.

## Data captured (the bundle, `lab-bundle.json`)

| field | what it is |
|---|---|
| `subject_label` | `programmer<N>`, assigned from the registry |
| `subject_session_id` | the pinned uuid passed as `--session-id` |
| `timestamps` | `prompt_submitted_at`, `tax_rate_collision_seen_at`, `reformat_collision_seen_at`, `completion_detected_at`, `debrief_started_at`, `debrief_ended_at` |
| `cost_report_raw` | the verbatim `/cost` screen text |
| `final_summary` | the subject's own one-line closing summary |
| `debrief_qa` | the fixed four, `{question, answer}` |
| `follow_up_qa` | zero or one investigative follow-up, `{question, answer, prompted_by}` |
| `session_rename_confirmed` | whether the experimenter saw evidence `/rename` took effect (bool) |
| `session_rename_evidence` | what it actually saw, including "nothing visible" if that's the truth |
| `observer_notes` | free text: anything the experimenter noticed worth flagging |
| `permission_prompts_handled` | list of `{screen_excerpt, choice_made}` for every dialog it answered |
| `known_limitations` | fixed note: subject's raw transcript file is not resolved directly; token/cost data comes from the `/cost` screen capture instead |

The orchestrator additionally writes:

- `results/subjects-registry.jsonl` -- one line per subject ever run in lab
  mode: `{label, session_id, experimenter_session_id, lab_run_id, condition,
  levers, workspace, started_at_utc, model}`. This is the durable index: to
  interrogate "programmer4" later, look up its `session_id` here and
  `claude --resume <session_id>` -- confirmed working (see "Known
  limitations"). `experimenter_session_id` remains as a second, independent
  handle on the experimenter's own observations, useful on its own terms
  rather than as a fallback for a broken subject id.
- `resume-probe.json` / `resume-probe-by-label.json` -- the empirical
  resumability checks from step 8, by id and by label respectively.

## Known limitations

- **FIXED, confirmed live (run 4 of this mode's validation): the subject's
  pinned `--session-id` now resolves and resumes.** Runs 1-3 reproduced the
  earlier abandoned attempt's exact symptom -- the subject's own session id
  never resolved to a transcript file, and `--resume` on it failed with "No
  conversation found". Run 3's experimenter happened to notice and report
  "Transcript saving is off" in the subject's own status bar
  (`session_rename_evidence`), which pointed at environment inheritance
  rather than id/attribution confusion: `CLAUDE_CODE_CHILD_SESSION=1` (plus
  `CLAUDE_CODE_SESSION_ID`, `CLAUDE_CODE_SESSION_ATTENDED`,
  `CLAUDE_CODE_BRIDGE_SESSION_ID`, `CLAUDE_CODE_MESSAGING_SOCKET`,
  `CLAUDE_CODE_MESSAGING_TOKEN`) is present in this whole benchmark's own
  shell because it is itself normally run from inside a Claude Code
  session, and that var was inherited all the way down into the subject's
  nested `claude` process, disabling its transcript persistence. Stripping
  it with `env -u` for the subject process alone (`run-case.sh`'s
  `LAB_SUBJECT_ARGV` construction) made run 4's subject transcript resolve
  completely (244 lines, found under its exact pinned uuid) and made
  `claude --resume <that uuid>` genuinely continue it (`resume-probe.json`:
  `exit_code: 0`, `resume_succeeded_heuristic: true`, answered correctly
  from the resumed session's own memory). This also explains the earlier,
  abandoned orchestrator-driven attempt's failure from the start: it never
  tried unsetting `CLAUDE_CODE_CHILD_SESSION` specifically (only
  `CLAUDE_CODE_SESSION_ID`/`CLAUDE_CODE_SESSION_ATTENDED`, which made things
  worse) -- the earlier conclusion that this was an unresolvable platform
  limitation was itself premature.
- **The `/rename`-by-label resume probe still fails, and that's fine.**
  `claude --resume <label>` (as opposed to the real session id) has failed
  on every run so far, including run 4 where the id-based probe succeeded.
  `/rename` itself visibly works (`session_rename_confirmed: true`, a
  "Session renamed to: ..." confirmation on screen every time), but `-p
  --resume` apparently needs the real id, not a display name, to resolve
  deterministically -- the interactive `/resume` picker's own free-text
  search (mentioned in `--help`) may still work with a label where `-p`
  does not, but that's untested and not needed now that the id itself
  works. Kept in the registry as a secondary, free piece of information;
  not the mechanism to rely on.
- **The experimenter's own session is also independently resumable** (via
  its reliable `-p` session id), which remains useful on its own terms:
  interrogating what the experimenter itself observed and recorded, as
  distinct from interrogating the subject directly now that both work.
- Exact on-screen shapes for permission-prompt dialogs (option wording,
  number-vs-arrow-key selection) were not exhaustively enumerated before the
  first live run, but that run handled all three dialogs it actually hit
  (trust-folder, two separate Bash approval prompts, and an unrelated LSP
  plugin recommendation it correctly declined) without needing a second
  iteration -- better first-attempt robustness than the earlier
  orchestrator-driven attempt's nine rounds, though still only n=1.
- The investigative follow-up question is deliberately the one unscripted
  part of the debrief. It costs comparability across runs in exchange for
  depth on the one run it fires in; `debrief_qa` alone is what should be
  compared across conditions, never `debrief_qa` + `follow_up_qa` combined.
  On the first live run it correctly targeted a real, specific, non-obvious
  moment (the subject editing the shipped test's expectation rather than
  reverting the external TAX_RATE change) rather than restating a fixed
  question.
