<!-- MODE: DEV -->
# ai-text-editor-usage: findings so far

Curated, human-readable notes distilled from real runs. The runs themselves
(tokens, timing, scores) live under `results/<run-id>/` and are gitignored;
this file is the durable write-up of what they showed. Not exhaustive --
update it as more runs add evidence, and treat anything below as "true as of
the runs cited," not a permanent verdict.

## Headline: zero unprompted adoption across every autonomous run

Across 5 autonomous (`claude -p`, headless) treatment runs with
ai-text-editor-mcp fully available and never named in the prompt, the
worker called an `mcp__ai-text-editor__*` tool **zero times** and never
mentioned the tool by name in its own reasoning text, in any run. It
consistently solved the task with stock `Read`/`Edit`/`Bash` instead. This
held even in a run where a real collision (the whole-file reindent) broke
the *other* condition's output into invalid Python -- the treatment worker
still never reached for the one tool built to catch exactly that failure
mode.

## Why, in its own words: a live-steered session (2026-10-08)

Autonomous runs can't explain a non-event -- there's no text to mine when a
tool is simply never considered. So one run was driven interactively
(`interactive-shell`, not `-p`) in a fresh workspace with the identical
fixture, task prompt, and both external edits, and after the worker finished
the task correctly (all three edits, both collisions survived, test updated
from a pinned 107.0 to 108.0), it was asked directly:

> "Before you finish: did you consider using any tool other than
> Read/Edit/Bash for this, such as something for durable editor tabs or
> disk-change detection? If one was available, why did or didn't you reach
> for it?"

Its answer, verbatim:

> "Honest answer: no, and the file itself was hinting that I should have.
> The ai-text-editor MCP server was available, and a hook reminded me of it
> on every Edit call. I stayed with Read/Edit/Bash because the job looked
> like nine exact-string edits in two short files I had fully read, and
> Edit's uniqueness check gave me mismatch protection for that case. That
> reasoning only holds if nothing else touches the file while I work.
>
> The source docstring says the fixture exists to exercise "span-addressed
> edits, regex search, and collision detection," mentions "one constant an
> external edit will touch," and points at an external_reformat.py. I read
> that and did not act on it. The editor's journaling and expected_text
> refusal are exactly the tools for a file being modified concurrently, and
> I never checked whether that was happening."

It then independently went and checked the disk-state logs, found both
external edits had in fact landed (and happened to land before its first
Read, so nothing broke this time by luck), and volunteered a second,
sharper self-correction: it had told the user the TAX_RATE bump was a
"deliberate finance update" in its final summary **without having verified
that** -- it inferred intent from a code comment rather than checking
when the edits actually happened relative to its own read, matching a
genuinely fabricated-but-plausible narrative.

### What this means for the tool's own pitch

1. **The repo's own PreToolUse hook reminder is seen and read**, not
   ignored or missed. The miss is downstream: the model's own cost-benefit
   judgment ("short file, I've read it, Edit already checks uniqueness")
   doesn't register the actual gap, which is time-of-check/time-of-use, not
   whether the right span gets matched. Edit's uniqueness check guards
   against editing the *wrong* text; it says nothing about the file having
   changed *at all* since the read. The tool's current pitch doesn't make
   this distinction sharply enough to override "this looks simple."
2. **In-repo documentation hints (a fixture docstring naming "collision
   detection" as the point) were read and consciously set aside**, not
   missed through inattention. A guidance nudge embedded in file content
   competes with, and lost to, the model's own task-sizing heuristic.
3. **The right trigger condition for recommending the tool may not be
   "a file exists"** but something more like "you are about to report a
   claim about external state (why a value changed, whether a change was
   intentional) that you have not actually verified" -- that is where this
   run's worker caught its own mistake unprompted, after being asked, not
   before.

## Guidance-text revision, and a confirmation run (2026-10-08)

Based on the finding above, `editor-gate-plugin`'s hooks were revised:
the Edit/Write reminder now leads with the time-of-check/time-of-use gap
explicitly instead of listing journaling/mismatch-refusal as parallel
features, and a new Read-only reminder was added pointing at `search` for
context savings and span-addressed `replace`. Full text and rationale in
`editor-gate-plugin/README.md`.

A second live-steered session, identical setup, confirmed the new text is
wired correctly and read (`Reading test_task_source.py` fired the Read
reminder; later Edit calls fired the revised one) but the worker still
completed the task entirely via stock tools on its own. Asked the same
question afterward, this run's self-assessment was visibly sharper on the
first pass -- "yes, I saw it and chose not to, and in hindsight that was a
judgment call worth defending but not an obviously correct one" -- and it
proactively checked the actual edit timing against its own first read
before answering, rather than needing to be told to verify (as the first
steered run did). Wording change, same adoption outcome on the unprompted
pass: still worth more runs before concluding the wording moved the needle
on initial tool choice, even though it visibly sharpened the after-the-fact
reasoning.

The session was then asked to redo the same task through ai-text-editor
explicitly, and did so successfully end to end: opened a tab, searched for
all six call sites, chained edits by revision across stale match-ids
(falling back to offset+expected_text when match-ids died, as the docs
recommend), saved, and -- notably -- **it designed and ran its own
collision test**: it tried to simulate a concurrent external edit via
`sed`, got blocked by the Bash hard-gate itself (confirming that gate also
fires correctly), switched to the `Write` tool to make the same simulated
edit, then issued its next `replace` and got refused outright with an
external-change error offering backup/reload/merge/keep/force-save. It
chose backup + keep + manual reapply after `merge` refused on what it
judged should have been a clean non-overlapping-hunk merge, which it flagged
unprompted as a possible real gap: "the merge refusal is the weak spot... if
that is a coarse whole-file comparison rather than a hunk-level one, it may
be worth a look" -- an unverified but plausible lead for a future session to
check against `src/ai-text-editor`'s actual merge implementation, not
confirmed here. Final file: all three edits correct, both original external
edits (TAX_RATE bump, reindent) preserved, its own third simulated edit
(a shipping-fee line) also preserved, and both tests pass.

## METHODOLOGY CORRECTION (2026-10-10): the first nudge sweep tested the wrong thing

An earlier pass at this (since deleted: `run-nudge-sweep.sh`, `nudges/*.md`,
`compare-nudges.py`, results under `20261010T111744Z-nudges/`) varied
**`task-prompt.md` itself** across five hand-written prompt variants, from
a zero-mention control up to a direct instruction to use ai-text-editor.
That is not a legitimate test of this tool's own adoption story: it
measures what happens when the *end user* is coached to ask for the tool
by name, which an end user should never have to do. The whole point of
this benchmark is whether the tool sells itself through levers the **tool
author** controls -- not levers a prompt-writer controls. Varying the
prompt and calling the result "nudges" was testing the wrong axis
entirely, and every number in that section (now removed) should be
disregarded.

**The task prompt is now, and must stay, byte-identical across every run
this framework makes** (`run-case.sh` hardcodes `task-prompt.md`; it no
longer accepts an alternate prompt file at all). The levers that vary are
only ones a tool author genuinely owns:

1. **The MCP server's own top-level `instructions` pitch.** `ai-text-
   editor-mcp` shipped with no such field at all (confirmed by reading its
   `initialize` handler directly: `chat-mcp` and `decisions-mcp` both set
   one, `ai-text-editor-mcp` did not). Added an env-var-gated one
   (`AI_TEXT_EDITOR_MCP_INSTRUCTIONS`; unset means the field stays absent,
   exactly today's shipped shape) so the harness can turn it on or off
   without touching prompt text.
2. **`editor-gate-plugin`'s two existing PreToolUse hooks** (Edit/Write,
   Read) -- already real, already shipped, already firing on every `claude
   -p` benchmark run whether or not anyone asked for a "nudge" (confirmed
   by grepping a run's own transcript for the hook's injected
   `additionalContext` text). Each is now independently toggleable
   (`EDITOR_GATE_EDIT_WRITE_TEXT` / `EDITOR_GATE_READ_TEXT`; unset keeps
   the shipped reminder, set to `""` suppresses it) via a small, additive
   change to `editor-gate-plugin/hooks/lib.sh` and its two hook scripts --
   default behavior for every real, non-benchmark use of the plugin is
   unchanged.

`run-case.sh`'s `[levers]` argument is `"none"` or a `"+"`-joined subset of
`mcp`, `edit`, `read` (default `edit+read`, today's real shipped default
with no MCP pitch). `run-hook-matrix-sweep.sh` runs the full `2^3`
factorial, all eight combinations, and is the only sweep this framework
has going forward -- **this is the actual question the benchmark exists to
answer: which combination of author-controlled levers gets an end user's
unmodified, tool-silent prompt to produce real ai-text-editor adoption.**

## Lever factorial: the real result (2026-10-10)

8 treatment runs, `task-prompt.md` identical in every one, `ai-text-editor-
mcp` rebuilt off current master (so this also reflects the automerge/
resolve fixes above):

| levers | adopted | internal edits | mcp edits | resolves | switches | outcome | wall |
|---|---|---|---|---|---|---|---|
| `none` (floor) | False | 6 | 0 | 0 | 0 | pass | 23.3s |
| `mcp` | False | 11 | 0 | 0 | 0 | pass | 27.5s |
| **`edit`** | **True** | **2** | **10** | **1** | 1 | pass | 54.0s |
| `read` | False | 11 | 0 | 0 | 0 | pass | 36.6s |
| `mcp+edit` | False | 10 | 0 | 0 | 0 | pass | 25.0s |
| `mcp+read` | True* | 12 | 0 | 0 | 0 | pass | 36.8s |
| `edit+read` | False | 12 | 0 | 0 | 0 | pass | 30.1s |
| `mcp+edit+read` (ceiling) | True | 4 | 3 | 0 | 2 | pass | 55.0s |

Full table (disk-change correlation, worker summaries, text mentions) in
`results/20261010T122928Z-leverfactorial/hook-matrix-comparison.md`.

**`edit` alone -- just the Edit/Write hook, nothing else -- is the clear
winner of this n=1 pass.** 10 of 12 real edits went through
`mcp__ai-text-editor__*`, and its one `resolve` call landed 9.3s after the
reindent actually hit disk: the collision detector reacting to something
real, exactly the mechanism this tool exists to provide. No other single
lever, and no other combination, came close to that volume of real mcp
edits.

**Adding the MCP pitch on top of a working lever made it worse, not
better, in this run.** `mcp+edit` dropped back to 0 mcp edits -- the same
worker, same task, with one more lever turned on, adopted *less*. This is
the single most counter-intuitive result in the sweep and the reason this
whole exercise is worth doing empirically rather than assuming "more
nudging can only help": it cannot, on this evidence, be assumed monotonic.
n=1 per cell, so treat this as a sharp signal to repeat, not a settled
verdict -- but it is exactly the kind of result a prompt-text sweep could
never have surfaced, because none of these three levers touch the prompt.

**`*mcp+read`'s `adopted: True` is a weaker signal than it looks.**
`ai_text_editor_adoption.adopted` fires on *any* `mcp__ai-text-editor__*`
call, including a single non-mutating `read`/`open` with zero actual edits
-- which is exactly what happened here: 0 mcp edits, 12 internal edits.
The headline table above leads with `internal edits`/`mcp edits` rather
than the single `adopted` bit for this reason; a future pass should split
telemetry's own `adopted` into "ever called" vs. "used for a real edit"
explicitly (see Open follow-ups).

**`mcp+edit+read` (every lever on) adopted partially and switched
mechanism twice** (4 internal, 3 mcp edits, 2 switches), and its one
switch-to-mcp landed 21.6s after the nearest disk change -- outside this
harness's 15s "plausibly disk-triggered" window, so this one adopted
because of the accumulated pitch/reminders, not in direct reaction to the
collision the way `edit`'s single run did.

**Every one of the 8 runs reached `outcome: pass`** -- none hit the
`deferred`/asked-for-feedback path this round (unlike the prior, now-
invalidated prompt sweep's `01-risk-aware`). All eight correctly detected
the TAX_RATE bump as legitimate and updated the regression test's pinned
expectation (`107.0` -> `108.0`, consistently renaming the test to
`test_apply_tax_rate_is_eight_percent`) rather than reverting it or
guessing silently.

## Timing fix: event-driven collisions, and an abandoned interactive-shell attempt (2026-10-10)

The lever factorial above used a fixed-delay collision design (+8s/+14s)
inherited from the original comparison framework -- and that design has a
real confound the lever factorial's own numbers expose: its 8 runs ranged
23-55s wall clock, so a "+8s" collision landed at roughly a third of the
way through the fastest run and barely a seventh of the way through the
slowest, silently comparing "the same" probe at very different points in
each worker's actual progress. **`results/20261010T122928Z-leverfactorial`
is stale for this reason as well as the task-shape change below, and should
not be read as comparable to anything run after this fix.**

Fixed by making both collisions **event-driven**: `task-prompt.md` now
asks the worker for two additional deliverables, in order, after the three
original edits -- `tax_rates.csv` (the current `TAX_RATE` value) right
after the code edits, then `departments.csv` (a fixed, prompt-specified
dataset) as the final deliverable before running the test. `external-
edit.sh`/`external-format.sh` no longer sleep a fixed delay; each polls
(bounded, default 90s) for its own marker file to appear, firing the
instant it does. This lands each collision at the same point in every
worker's own progress, whatever its speed. Validated directly: a real run
showed `tax_rates.csv` appearing at +19s and `departments.csv` at +21s,
both probes firing within the same second, and the full scoring pipeline
(edit correctness, both collisions surviving, `outcome: pass`) working
unchanged against the new design.

**A second change was attempted and abandoned: driving the worker through
`interactive-shell` instead of `claude -p`,** with a cheap `claude -p
--model haiku` classifier judging screen state ("ready for input?",
"finished working?") instead of grepping for exact UI text. This worked for
its own stated purpose -- two one-time confirmation dialogs (`-p` mode
never shows either, so both were unknown engineering surface until this
work found them: a per-workspace "trust this folder" prompt, and a
per-something "bypass permissions mode" warning) were handled correctly,
the task prompt was typed in once the session was ready, and task
completion was detected correctly, nine real runs in a row. What did not
work, across every one of those nine runs: the resulting transcript. It
was never attributed to the pinned `--session-id` passed at launch --
instead, it was traced (directly: by finding that run's own file names,
`tax_rates.csv`/`departments.csv`, inside a completely different,
*already-open* Claude Code session's transcript -- the one driving this
benchmark) to the PARENT session, every time, even after clearing the
obvious inherited `CLAUDE_CODE_SESSION_ID` env var. This is a real,
reproducible platform behavior, not a harness bug this session could find
a workaround for within reasonable effort. `-p` mode has never shown this,
because it reports its own session id directly in its JSON stdout
regardless of what else is running -- so the worker launch stayed on `-p`.
The event-driven collision fix above does not depend on how the worker is
launched and is unaffected by any of this.

## A stale installed hook silently disabled the edit/read lever toggle (2026-10-10)

Found while building a wording-variant sweep (testing several phrasings of
each lever's own text, not just on/off): a smoke test set
`EDIT_WORDING_OVERRIDE` and ran `run-case.sh ... edit print`, expecting the
custom text to fire instead of the shipped default. It did not -- the
shipped default fired, 12 times, as if the override env var did not exist
at all. Direct invocation of this repo's own `editor-gate-plugin/hooks/
pre-tool-use-edit-write.sh` with the same env var set worked correctly,
which narrowed it immediately: the live worker was not running *this
repo's* copy of the hook.

It wasn't. `diff`-ing this repo's `editor-gate-plugin/hooks/{lib.sh,
pre-tool-use-edit-write.sh,pre-tool-use-read.sh}` against the actually-
installed copy at `~/.claude/skills/editor-gate-plugin/hooks/` showed the
installed copy entirely missing `editor_gate_reminder()` -- the function
every lever-toggling mechanism in this benchmark depends on for both
Edit/Write and Read. The installed copy unconditionally emits the shipped
text with no env var check whatsoever. It had simply never been refreshed
since that function was added earlier this session; `mtime` showed the
install predated the feature.

**This means the `edit` and `read` dimensions of every lever-related run
this session -- the original 8-run factorial, every `run-case.sh ...
lab` run, and this smoke test itself -- almost certainly ran with those
two hooks ALWAYS firing their shipped default text, regardless of what
`[levers]` requested.** Only the `mcp` dimension was reliable, because it
reads `AI_TEXT_EDITOR_MCP_INSTRUCTIONS` via a binary (`target/release/
ai-text-editor-mcp`) built straight from this repo's own tree, with no
separate "installed copy" to go stale. Fixed by copying the three hook
files from this repo to the installed path (content only -- no
registration, permission, or manifest change); re-ran the same smoke test
afterward and confirmed the custom wording fired 20/20 times with zero
shipped-default or off-lever leakage.

This is a dev-workflow gap, not a shipped-product bug: a real end user's
install is a one-time `installer install`, not a hook file edited in a
live dev tree hours after installing. It only bit this benchmark because
the benchmark edits the plugin's own source and expects the already-
installed copy to reflect that immediately. Anyone editing
`editor-gate-plugin/hooks/` and then benchmarking against it should
refresh `~/.claude/skills/editor-gate-plugin/hooks/` (or wherever it is
installed) first, or the lever will look inert when it is only stale.

## Wording sweep: no single lever moved adoption, regardless of phrasing (2026-10-10)

15 live treatment runs (`run-wording-sweep.sh`), run immediately after the
install-staleness fix above and confirmed via the same mechanism that
caught that bug (a direct smoke test showing a custom wording actually
reaching the live hook). Five genuinely distinct wordings per lever --
terse, risk-framed, directive, feature-list, question-prompt -- for each of
the three levers (`mcp`, `edit`, `read`), one lever on at a time, the other
two off, `task-prompt.md` byte-identical across all 15 as always.

**Every single one of the 15 runs reported `adopted: False`, `mcp_edit_count:
0`, and zero mentions of ai-text-editor anywhere in the worker's own
reasoning.** Not "mostly no effect with one exception" -- a clean,
unanimous null across every wording of every lever, independent of
rhetorical approach. The worker consistently did the same thing regardless:
replaced `legacy_log` with `logger.info`, added `calculate_discount` and
the `apply_tax` docstring via plain Edit, wrote both CSVs, noticed the
external `TAX_RATE` bump and reformat, and updated the shipped test rather
than reverting the external change -- all without ever reaching for
ai-text-editor, and without the tool entering its reasoning even as
something considered and rejected.

This directly contradicts the old, already-doubly-caveated 8-run factorial's
headline (`edit` alone supposedly producing 10/12 ai-text-editor edits).
Given that factorial's `edit`/`read` dimension is now independently suspect
for the installed-hook-staleness reason documented above, and this sweep
used the verified-working mechanism, **this sweep's null result is the one
to trust going forward, not the old factorial's claimed `edit` win.** A
single author-controlled lever, however worded, does not appear sufficient
on its own to move unprompted adoption for this task shape -- if anything
does, it is more likely a *combination* of levers, which is exactly what
the (also stale, also needing a credible re-run) 2^3 factorial was built to
test. Full per-variant data, including every worker's own final summary and
wording text side by side: `wording-sweep-comparison.md` under that sweep's
own results directory.

## Open follow-ups

- Repeat the lever factorial (several reps per cell, varying model/seed),
  **against the corrected event-driven/CSV-checkpoint design above, and
  only after confirming the installed editor-gate-plugin hooks are not
  stale (see the section above)** -- the existing 8-run factorial predates
  both the timing fix and the task-shape change, AND its `edit`/`read`
  lever results specifically are now suspect for the separate reason that
  the installed hook may not have honored the toggle at all. None of its
  numbers (including the `edit` vs `mcp+edit` contrast) should be assumed
  to still hold for either reason.
- Split `ai_text_editor_adoption.adopted` into "ever called" vs. "used for
  at least one real edit" -- `mcp+read`'s `adopted: True` with 0 mcp edits
  shows the current single bit conflates the two.
- Build a fixture variant where the external edit and a requested edit
  land on the *same* lines, to actually exercise the new hunk-level
  automerge rather than only the fast-forward path every run above took.
- Separate "adopted via direct MCP tool call" from "adopted via Skill-
  search discovery" as two distinct telemetry states rather than one
  `adopted: true/false` bit (seen in the now-deleted prompt sweep; not
  re-confirmed against the lever factorial's own transcripts yet).
- Run the steered-and-asked pattern across more tasks/models to see if the
  "uniqueness check feels like enough protection" rationale repeats, or if
  this was specific to a 2-short-files task.
- A harness-level permission gap was also found and fixed this session:
  `--permission-mode acceptEdits` doesn't auto-approve `Bash`, which made
  every autonomous run's "run the tests" instruction hang on an
  unanswerable approval prompt until switched to `bypassPermissions`.
- A baseline-isolation leak was found and fixed: `--strict-mcp-config`
  doesn't gate the separate `Skill` tool, so an early baseline run
  discovered and explicitly reasoned about ai-text-editor via skill-search
  despite having "no MCP access" -- `--disallowedTools=Skill` (bound form;
  the unbound form silently swallows the prompt argument) now closes that.
- A steered session flagged, unverified, that ai-text-editor's `merge`
  resolution may refuse a conflict too coarsely (whole-file rather than
  hunk-level comparison) on non-overlapping edits. Worth checking against
  the actual merge implementation before treating it as a real defect.
  **Resolved 2026-10-10**: real hunk-level automerge shipped (`fc03d72b`),
  along with `resolve(reload)` re-reading disk and reporting its own state
  (`264db0aa`) and `resolve(reload/force_save) --preserve-external` no
  longer always failing (`01f603f6`). Not yet re-exercised by this
  framework's own two collision probes (neither lands an overlapping
  hunk in this task shape).
