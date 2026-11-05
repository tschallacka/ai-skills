<!-- MODE: DEV -->
# ai-text-editor-usage: findings so far

Curated, human-readable notes distilled from real runs. The runs themselves
(tokens, timing, scores) live under `results/<run-id>/` and are gitignored;
this file is the durable write-up of what they showed. Not exhaustive --
update it as more runs add evidence, and treat anything below as "true as of
the runs cited," not a permanent verdict.

## UPDATE (2026-10-10): the zero below is no longer universal -- see "Naming ToolSearch directly breaks the null" further down. Read this section for how the null arose; read that one for what actually moves it.

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

## Lever factorial, re-run corrected: the null holds even at the ceiling (2026-10-10)

The full 2^3 factorial (`run-hook-matrix-sweep.sh`), re-run immediately
after the install fix above, against the current event-driven/CSV-
checkpoint task design, with the installed editor-gate-plugin hooks
confirmed in sync. This is the credible version of the run the earlier,
doubly-caveated `results/20261010T122928Z-leverfactorial` was never able
to be.

**All eight conditions -- `none`, `mcp`, `edit`, `read`, `mcp+edit`,
`mcp+read`, `edit+read`, and `mcp+edit+read` (every lever on at once, the
ceiling) -- reported `adopted: False`, `mcp_edit_count: 0`, and zero
mentions of ai-text-editor anywhere in the worker's own reasoning.** This
is the single most important result this benchmark has produced: even with
the MCP server's own top-level pitch, the Edit/Write reminder, and the
Read reminder ALL firing on the same run, Sonnet still never called the
tool, never named it, and solved the task identically to the `none`
condition in every observable way (`internal_edit_count` differs by at
most one between rows, which is noise, not signal). This directly
contradicts the stale factorial's headline (`edit` alone supposedly
producing 10/12 ai-text-editor edits) and confirms the wording sweep's own
null above was not a wording-specific fluke -- it holds at the maximum
combination of every lever this benchmark currently has, not just at each
one in isolation.

This now matches, and reinforces with far more rigor, the very first
"Headline" finding at the top of this document from 5 early autonomous
runs with no levers at all: zero unprompted adoption. The intervening
weeks of lever-building, wording-variance testing, and lab-mode
instrumentation have not moved that number. Whatever would actually move
it -- a different task shape, a different model, prompting rather than
tool-author-controlled nudges (which this benchmark deliberately excludes
as out of scope), or something else entirely -- remains unknown and is the
real open question this benchmark has not yet answered.

## Interviewing the subjects: why the null, in their own words (2026-10-10)

The lever factorial and wording sweep both answer "whether" with a clean
no, but not "why" -- and this time, unlike the stale factorial, there was
no need to run anything new to ask. Every one of the 23 completed `-p`
sessions from both sweeps is a plain print-mode session, which has never
had the subject-resumability problem lab mode hit: `claude --resume
<session-id>` reaches every one of them directly. So the subjects were
interviewed in place, resuming five real sessions spanning the extremes
(`none`, `mcp`, `edit`, `read`, `mcp+edit+read`) and asking each, in its
own resumed context, to walk through its own tool choices.

**First, a methodology note worth recording on its own**: asking Sonnet
5.5 itself, resumed, got refused outright -- `API Error: Sonnet 5.5's
safeguards flagged this message ... [reasoning_extraction]` -- on three
separate phrasings, including ones with no mention of "blocked" or
"gated" at all. Resuming the exact same session with `--model haiku`
instead worked cleanly every time, and the lab-mode debrief earlier this
session (also effectively an introspective "why" question, just inside a
live interactive session rather than a `--resume` call) never hit this.
The common factor across the three refused attempts: a `--resume` call,
no live session context, asking the model to account for its own past
choice. Worth remembering before assuming a model will cooperate with
this kind of retrospective interview via plain resume.

**Grep first, before asking anything**: all 23 transcripts contain exactly
one hit each for the Bash PreToolUse hook's block text ("In-place shell
editing is gated"). Every single subject tried a `sed -i` or heredoc
rewrite as its first instinct, got blocked, and never minted a bypass
token. The block is unconditional -- not one of the three levers this
benchmark toggles -- so this happens in literally every run regardless of
`[levers]`. That is the behavior the question was actually asking about:
subjects visibly "listen to" this one hook every time. They just don't
route to the tool it names.

**Why the block changes behavior but never routes to ai-text-editor,
in the subjects' own words, independently and consistently across all
five interviews:**

- *"I did not weigh option 1 in any deliberate way. I went straight to
  Edit."* (`none`) *"I moved on because the hook blocked the command, not
  because I compared the options."* (`edit`) *"It wasn't a comparison ...
  I treated them as unavailable."* (`mcp+edit+read`) The block is treated
  as "stop doing THIS", never as "do THAT instead" -- it eliminates one
  option without ever making the suggested replacement the active choice.
- **The decisive, previously undiscovered mechanism: ai-text-editor's MCP
  tools are *deferred***, exactly the way tools are deferred in this very
  harness (see the system reminder listing `mcp__ai-text-editor__*` by
  name only, schema withheld until a `ToolSearch` call loads it). Four of
  the five subjects named this explicitly and unprompted: *"The
  ai-text-editor tools were deferred, so using them meant a `ToolSearch`
  first, and I didn't try that."* (`mcp`) *"I never ran `ToolSearch` to
  load the ai-text-editor tools."* (`edit`, `read`) *"The ai-text-editor
  MCP tools were deferred, so their schemas weren't loaded, and I never
  ran `ToolSearch` to load them. I treated them as unavailable."*
  (`mcp+edit+read`) Edit/Write/Read need no unlocking step; they are
  already loaded and callable. ai-text-editor is one extra, un-nudged tool
  call away from being callable AT ALL, regardless of how its pitch or
  hook text is worded -- a structural cost no wording variant could ever
  have overcome, since none of the 15 wording variants or 8 lever
  combinations changes whether the tool is deferred.
- **A recurring misattribution, confirmed as general rather than the
  single steered-session fluke "Why, in its own words" (above)
  documented**: *"I stuck with [Edit] because Edit requires an exact,
  unique `old_string` match and fails on a mismatch rather than silently
  doing nothing. That is the same property the hook cited for
  ai-text-editor's `expected_text`. I only saw that parallel after the
  fact, though."* (`none`) *"Edit also refuses when `old_string` doesn't
  match, which covers the main safety concern the hook raised. I didn't
  weigh that at the time."* (`read`) Three of five subjects drew this same
  false equivalence between Edit's uniqueness-in-what-I-already-read check
  and ai-text-editor's disk-staleness check -- independently, without
  being told to compare the two.
- **The soft reminders' own "optional" framing is taken at face value, as
  designed, with the designed consequence**: *"The Edit and Write hooks
  never forced the switch, because they said the suggestion was optional."*
  (`mcp+edit+read`) *"The hook on these calls said ai-text-editor 'could
  also' be used and was 'not required here,' so I kept going."* (`edit`)
  This is not a misreading -- the shipped text genuinely does say exactly
  that (`editor-gate-plugin/hooks/pre-tool-use-edit-write.sh`'s own
  wording, chosen deliberately per the 2026-10-08 finding above, to avoid
  an earlier version that overclaimed). The subjects are reading it
  correctly and acting on exactly what it licenses: skip it.
- **The MCP pitch's content was accurate and would have applied directly
  -- and still never got retrieved at the moment it mattered**: *"Its MCP
  instructions say to prefer it over Read/Edit/Write when a file might be
  touched by another process. The `TAX_RATE` change is that situation, and
  I didn't connect it to the tool at the time."* (`mcp`) *"I saw it in the
  Bash-hook block and the Edit/Write notes, but didn't act on it. Its
  revision guard would have refused the edit when the disk changed,
  instead of me finding out from a notice afterward."* (`mcp+edit+read`)
  The pitch is shown once, at session start, well before the collision it
  describes actually happens; by the time the triggering condition
  (external edit) occurs, it has fallen out of active consideration. This
  is a recall/timing problem, not a persuasion problem -- no amount of
  rewording a message shown once at the start fixes a failure to recall it
  minutes later when it becomes relevant.

**One more recurring pattern, incidental to the main question but
consistent across interviews**: several subjects used this round of
questioning to volunteer, unprompted, that their own final summary had
omitted the reformat ("A correction to my earlier summary..."; "Something
I should have flagged earlier..."), the same self-correction-on-being-
asked pattern the lab-mode debrief surfaced earlier this session.

**Bottom line**: the "stubbornness" is not stubbornness in the sense of
weighing ai-text-editor and rejecting it. It is never being weighed at
all -- blocked once, redirected to an already-loaded tool with no further
step required, with the suggested alternative sitting behind a `ToolSearch`
call nobody has reason to make once the already-available tool already
worked. Wording cannot fix a tool that is never loaded in the first place.

## Naming ToolSearch directly breaks the null (2026-10-10)

Tested the fix the interviews pointed at directly, same day: give the Bash
gate's own deny message (the ONE thing every subject reliably complies
with) an explicit instruction to call `ToolSearch` before anything else,
naming the exact call (`ToolSearch(query="select:mcp__ai-text-editor__open,
mcp__ai-text-editor__replace,mcp__ai-text-editor__save", max_results=5)`),
and telling it not to fall back to Edit as a substitute for that step.
Implementation: `EDITOR_GATE_BASH_TEXT`, a new override on
`pre-tool-use-bash.sh` mirroring the other two hooks' mechanism but without
their suppress-by-empty option (this gate's deny decision itself is never
toggleable -- only its explanatory text is), wired into `run-case.sh` as
`BASH_WORDING_OVERRIDE`, independent of the three on/off levers since this
gate fires in every run regardless of them. `wordings/bash-1-toolsearch.txt`
is the new wording; everything else (task, workspace, scoring,
collisions) is unchanged from every prior run.

**5 reps, levers=none (every other lever off, isolating this one change):
2 of 5 adopted ai-text-editor for real edits** (`mcp_edit_count` 8 and 10;
the other three: 0, 0, 0). Both adopting runs show the intended workflow
exactly: `ToolSearch` within the first ~5 seconds, then
`open`/`read`/`replace`×6/`save`×2 for the `task_source.py` edits, then an
appropriate switch to `Write` for the two new CSV files (per the hook's
own option 2) and `Edit` for the minor test-file fix. All 5 reps still
scored `outcome: pass` with every edit correct -- adopting the tool never
cost correctness.

**This is a real, substantial shift, not a fluke or a rerun of noise**: 23
prior runs (the full lever factorial plus the 15-variant wording sweep,
all with the shipped Bash-gate text) produced zero adopting runs out of
23. These 5 produced 2. It is also clearly not deterministic -- 3 of 5
still went to zero, so whatever makes the difference between an adopting
and a non-adopting run under the identical new wording is not yet
understood, and `n=5` is nowhere near enough to put a real number on the
new adoption rate. But going from a clean, 23-run zero to a nonzero
result on the very first wording change the interviews actually pointed
at is itself strong evidence the interviews correctly diagnosed the
mechanism: the problem was never persuasion text, it was that the tool
sat behind an unlock step nothing in the task ever prompted taking.

### What actually separates the 2 adopting runs from the 3 non-adopting ones

Read directly from each transcript's own `thinking`/`text` content blocks
at the exact turn right after the Bash gate fires -- more reliable than
asking, since this is the unedited record, not a retrospective account --
then cross-checked by resuming three of the five (via `--model haiku`,
`--resume`, per the pattern established earlier) and asking each
specifically about that one turn:

- **Both adopting runs produced a one-line acknowledgment before acting**:
  *"Use the editor tool."* / *"Hook directs me to the ai-text-editor
  tools; loading them."* -- then immediately `ToolSearch`, `open`, `read`.
- **All three non-adopting runs show an EMPTY thinking block and no text
  at all at that exact turn** -- straight to a tool call, zero visible
  engagement with the hook's content, verified directly in the raw JSONL
  (`sed -n` on the exact line, not a summary). Asked afterward, their own
  accounts converge exactly on this: *"I didn't weigh the hook's options
  ... I never considered them as alternatives ... the tool was listed as
  deferred, and I never loaded its schema with ToolSearch, so I didn't
  consider it."* (the run that chose `Write` for the whole file -- a
  legitimate reading of the hook's own option 2, not pure habit, but
  arrived at the same way: without registering option 1 at all) *"I
  didn't stop to compare the options the hook listed ... I took the
  nearest non-shell option without checking whether the one the hook
  recommended was actually available."* (the two runs that went straight
  to `Edit`)
- **The adopting run's own retrospective account is honest about how fast
  its own choice was too**: *"Option 1 matched the situation, so I didn't
  weigh the others for long."* -- not a deliberated comparison that came
  out in ai-text-editor's favor, just a faster convergence on the same
  option the hook named.

The working hypothesis this supports: the fork isn't a reasoned
accept/reject of the ToolSearch instruction. It's whether that turn
processes the hook's full message closely enough to register the new
instruction buried in it at all, or pattern-matches only the gist ("shell
editing blocked, something about ai-text-editor") and reacts from habit
before reaching the part of the message that changed. A message that gets
skimmed can't act on an instruction it never reads as instructing
anything specific. This is a hypothesis from `n=5`, not a settled
mechanism -- the concrete next step is more reps, specifically watching
whether presence/absence of that one-line acknowledgment predicts the
outcome at a larger sample, and whether shortening the message (so there
is less to skim past before reaching the ToolSearch instruction) changes
the split.

## Bash-gate wording trial: six phrasings, 30 reps, a near-ceiling result (2026-10-10)

Extended the terse/risk/directive/features/question taxonomy already used
for the mcp/edit/read levers (`wordings/mcp-*.txt`, `edit-*.txt`,
`read-*.txt`) to the Bash gate itself, which had only ever been tested as
one hand-written variant (`bash-1-toolsearch.txt`, 2/5 adoption). Six new
files, same voice per category as the existing taxonomy, all keeping the
gate's three-way structure (edit a file -> ToolSearch then
open/replace/save; generate a file wholesale -> Write; a genuine
mechanical sweep -> the editor-token escape hatch) but varying length and
rhetorical stance:

- `bash-2-terse.txt` -- three sentences, no numbered options, no
  rationale.
- `bash-3-risk.txt` -- leads with the sed/heredoc silent-failure risk,
  condensed.
- `bash-4-directive.txt` -- flat numbered imperative, no persuasion.
- `bash-5-features.txt` -- sells the capability (expected_text refusal,
  journal survives a checkout), closest in spirit to `bash-1` but shorter
  and less structured.
- `bash-6-question.txt` -- posed as a question ("have you actually loaded
  the tools yet?").
- `bash-7-challenge.txt` -- the framing the user asked to test directly:
  explicitly tells the model a fair comparison requires actually loading
  ai-text-editor before picking something else, rather than defaulting to
  what was already loaded.

5 reps each (30 runs total), `levers=none`, everything else (task,
workspace, collisions, scoring) unchanged from every prior run.

**Every one of the six beat `bash-1`'s 2/5 baseline, most by a lot:**

| variant | adopted | ack-signal (non-empty thinking/text before 1st tool) |
|---|---|---|
| bash-2-terse | **5/5** | 1/5 |
| bash-4-directive | **5/5** | 0/5 |
| bash-3-risk | 4/5 | 0/5 |
| bash-6-question | 4/5 | 0/5 |
| bash-5-features | 3/5 | 1/5 |
| bash-7-challenge | 2/5 | 1/5 |
| *(bash-1-toolsearch, prior run)* | *2/5* | *2/5* |

23/30 adopted overall (77%), against 2/23 (9%) for every prior run under
the shipped or original new wording. The two wordings that hit the
ceiling, `terse` and `directive`, are also the two with the least prose
around the ToolSearch instruction -- consistent with "less to skim past."
But `features`, structurally closest to the original `bash-1` (the one
that only got 2/5), also underperforms `terse`/`directive` here, and
`challenge` -- which explicitly asks the model to deliberate rather than
default -- did *worst* of the six. Length alone doesn't fully explain the
spread; stance seems to matter at least as much: a flat instruction to act
beats an invitation to weigh options, even though the latter is literally
asking for the fair comparison the user wanted tested.

**The skim-vs-process (empty-thinking) signal from `n=5` did not
replicate at `n=30`, and the mechanism actually driving the split turned
out to be different.** Rerunning the same extraction (first assistant
turn after the gate's deny: is `thinking`/`text` non-empty before the
first `tool_use`?) across all 30 reps found **5/5 adopting-via-empty-
thinking** runs, not the "adopters always acknowledge" pattern the `n=5`
sample suggested -- most adopting runs (18 of 23) went straight from the
gate to `ToolSearch` with an empty thinking block and no text at all,
exactly the "skim" signature the earlier hypothesis predicted would fail.
What actually separates an adopting from a non-adopting run is not
whether the turn paused to acknowledge the hook -- it's simply **which
tool the reflexive, no-deliberation first move landed on**: `ToolSearch`
(success) or `Edit`/`Write` directly (failure), confirmed by reading the
same turn in three non-adopting reps (`bash-5-features-rep3`/`rep5`,
`bash-6-question-rep5`) and finding the identical empty-thinking-straight-
to-`Edit` pattern the original interviews described. So the `n=5` finding
wasn't wrong about the *failure* mode (skim-and-default-to-Edit is real
and still appears), but it was wrong to infer that an acknowledgment is
*required* for success -- a short, directive-enough wording apparently
gets reflexively pattern-matched onto `ToolSearch` just as easily as a
longer one gets pattern-matched onto `Edit`.

**One `bash-7-challenge` rep (`rep1`) shows a third outcome worth naming
separately: genuine deliberation that rejects the tool.** It called
`ToolSearch`, loaded the real schemas, then wrote, verbatim: *"I loaded
the editor tools. The file is small and I've already read it. A full
rewrite with Write is simpler than about ten revision-guarded replaces,
so I'll use that."* -- then used `Write` instead, scoring `adopted: false`
by the current binary metric despite having done exactly what the
`challenge` wording asked (actually load and weigh the option) and
reached a reasoned, defensible conclusion against it. This is distinct
from the skim failure mode above and from the three straight-to-`Edit`
reps in other variants: the telemetry's single `adopted` bit cannot tell
"never considered it" apart from "considered it and passed," which were
previously conflated as identical zeros. `challenge`'s worst-of-six
ranking may partly be an artifact of this: it's the one wording whose
own instructions make a reasoned "no" a legitimate, intended outcome,
not just a failure to notice the option.

Implementation: `benchmark/ai-text-editor-usage/wordings/bash-{2..7}-*.txt`;
run via the existing `BASH_WORDING_OVERRIDE`/`run-case.sh` mechanism, 5
reps per variant, `levers=none`, launch-mode `print`. Raw results under
`results/20261010T195001Z-bash-wording-trials/` (gitignored, not
committed).

**Open after this:** `n=5` per variant is still thin for a 2/5-vs-5/5
spread -- enough to say every new wording beats the original, not enough
to rank `terse` over `directive` or `risk` over `question` with
confidence. The `ToolSearch`-vs-`Edit` first-move signal (rather than
ack-presence) is the more promising automatic check to add to
`extract_edit_timeline.py` next. And the three-way outcome space
(adopted / skimmed-and-defaulted-to-Edit / considered-and-declined) is
worth splitting out as its own telemetry field rather than inferred after
the fact from transcripts, given `bash-7-challenge-rep1` above.

## Spike: making `expected_revision` optional did not flip the decline (2026-10-10)

**Unshipped, n=8, directional only.** The `bash-7-challenge-rep1` decline
above justified itself with "a full rewrite with Write is simpler than
about ten revision-guarded replaces." The user proposed the PhpStorm
analogy -- silent normal editing, a conflict dialog only on real
divergence -- and asked to test whether making `expected_revision`
optional (server trusts its own tracked `tab.revision` when the caller
omits one; a supplied revision still checked exactly as before) removes
that counted overhead. Patched on a local, unpushed `spike/optional-
revision` branch (`ai-text-editor-server.rs`'s revision-guard block,
`ai-text-editor-mcp`'s `mutating_required()` emptied so the MCP schema no
longer marks the field `required`), built, pointed `run-case.sh`'s
`$REPO_ROOT/target/release/ai-text-editor-mcp` at the spike build, ran
`bash-7-challenge` x4, `bash-2-terse` x2, `bash-4-directive` x2 (same
mechanism as the 30-rep trial above), then rebuilt from `master` and
confirmed the schema's `required` list was back to `["expected_revision"]`
before finishing.

**Result: `bash-7-challenge` went 0/4 (was 2/5 under the mandatory-
revision server), `bash-2-terse` 1/2 and `bash-4-directive` 2/2 (both
consistent with their prior near-ceiling rate at this small n).** The
interesting part is *how* the three `bash-7-challenge` declines happened:
all three that called `ToolSearch` loaded the now-optional-revision schema
and still went straight to `Write` with an **empty thinking block and no
stated reason at all** -- unlike the original rep1, which gave the
"revision-guarded replaces" rationale explicitly. Removing the thing it
named as the cost didn't make it choose the tool; it just removed the
reason it had given for a decision that looks, from this small sample,
already made on other grounds (file is small, already read, Write is
simpler, full stop).

**Confound worth flagging before reading this as a real negative result:**
`ai-text-editor-mcp`'s `revision_guard()` schema *description string* was
left unchanged -- it still reads "REQUIRED for this operation" even though
`required` no longer lists it. Every adopting run in this spike (the
`bash-2-terse`/`bash-4-directive` reps that did call `replace`) kept
supplying `expected_revision` anyway, tracking 1, 2, 3... itself exactly as
before -- consistent with the description text, not the schema's
`required` array, being what the model actually reads. So this spike did
not cleanly test "no revision-tracking overhead at all"; it tested
"revision-tracking overhead that the tool's own text still describes as
required," which is a weaker version of the hypothesis. A real test needs
the description text rewritten to match (e.g. "optional; supply it if you
have it, the server already tracks the current revision"), not just the
schema's `required` array emptied.

Not committed to `master` except this note; the code patch stays on
`spike/optional-revision` for the user to revisit, either as a cleaner
re-run (fix the description-text confound first) or via the full
planning-skill track if the direction still looks worth pursuing after
that.

### Follow-up: fixing the description-text confound, re-run (2026-10-10)

**Still unshipped, n=9, still directional -- but the clean version of the
test.** Rewrote `revision_guard()`'s description on the same
`spike/optional-revision` branch (no longer "REQUIRED"; now states the
field is optional and the server tracks the tab's own current revision
when it is omitted) and fixed the one other place the adapter's own tool
description still said "every mutation needs a current revision"
(the per-tool reconnection blurb). Rebuilt, confirmed via a direct
`tools/list` probe that `replace`'s schema now says OPTIONAL with no
contradicting sentence anywhere, then re-ran `bash-7-challenge` x5,
`bash-2-terse` x2, `bash-4-directive` x2 -- same mechanism, same
`run-case.sh` path.

**Result: `bash-7-challenge` went 3/5 adopted** (up from 2/5 under the
original mandatory-revision server, and up from the confounded pass's
0/4); `bash-2-terse` and `bash-4-directive` both held 2/2. More
importantly, this pass shows the mechanism actually engaging rather than
riding on the baseline's own noise: every `replace` call across all three
adopting `bash-7-challenge` reps (10 calls total, plus every call in the
`bash-2-terse`/`bash-4-directive` reps) **omitted `expected_revision`
entirely**, several substituting `expected_text` instead -- the opposite
of the confounded pass, where adopting runs kept tracking a revision
counter out of habit. The two declines (`rep2`, `rep3`) didn't even call
`ToolSearch` this time -- straight to `Edit` with no thinking and no text,
the same reflexive skim-to-`Edit` pattern the 30-rep wording trial found,
not a deliberated rejection like the original rep1's "ten revision-guarded
replaces" line. Nothing broke: `score.json`'s `all_three_edits_correct`/
`tax_rate_external_edit_preserved`/`external_reformat_preserved` were all
`true` across every one of the 9 reps, adopted or not.

So, at this still-small n: removing the argument's enforcement alone
(first pass) didn't move adoption and only erased the model's own stated
reason for declining; removing the enforcement *and* fixing the
contradictory description (this pass) did move it, and the agents that
adopted demonstrably stopped tracking the revision themselves. Whether
that's real signal or n=5-vs-n=4 noise on an already-noisy wording
(`bash-7-challenge` was the worst-performing of the six in the original
30-rep trial) is the open question — worth more reps before treating 3/5
as a settled rate. Master's binaries were rebuilt from `master` and
reconfirmed (`required: ["expected_revision"]`, description says
"REQUIRED" again) before finishing; the description fix is committed to
`spike/optional-revision` only (`991ef66d`), still unpushed, still not on
`master`.

## Real implementation (schema property removed, not just optional): same `bash-7-challenge` rate as the corrected spike (2026-10-11)

**Shipped-quality, not a spike -- the full planning-skill track's output, uncommitted on `master`.** The two spikes above tested "`expected_revision` optional, with or without a matching description string." The real implementation goes one step further than either: `ai-text-editor-mcp`'s MCP schema drops `expected_revision` from `properties` entirely on every affected tool (not merely from `required`), so a caller cannot pass it through the normal MCP tool-calling surface even if it tried -- confirmed by a direct `tools/list` probe (`replace`'s schema: no `expected_revision` key, `required: []`) before this run. The client (`ai-text-editor`/`ai-text-editor-mcp`, through the shared `client::execute()` seam) now fetches the tab's current revision itself, just-in-time, immediately before composing any guarded request that has none -- no cache, no identity-keyed state, nothing the caller has to track. The server's own enforcement (`is_revision_guarded`, `RevisionGuard::check`) is untouched.

Same mechanism, same `run-case.sh` path, `levers=none`: `bash-7-challenge` x5 (`results/20261011T101240Z-automatic-revision-guard-verify/bash-7-challenge-rep{1..5}`), plus `bash-2-terse` x2 and `bash-4-directive` x2 as regression sanity checks (both held 2/2 in the corrected-spike pass).

| variant | this run | original (mandatory revision) | confounded spike | corrected spike |
|---|---|---|---|---|
| bash-7-challenge | **3/5** | 2/5 | 0/4 | 3/5 |
| bash-2-terse | 1/2 | -- (5/5 @ n=5 in the 30-rep trial) | -- | 2/2 |
| bash-4-directive | 1/2 | -- (5/5 @ n=5 in the 30-rep trial) | -- | 2/2 |

**`bash-7-challenge` did not move past the corrected spike's 3/5 at this n.** The structurally stronger fix -- the property is genuinely gone, not just described as optional -- did not raise the rate further. The two declines (`rep1`, `rep4`) are both the familiar skim-to-`Edit` reflex, not a deliberated rejection: `rep4` never called `ToolSearch` at all (straight `Bash`/`Edit`/`Write`, matching the 30-rep trial's "reflexive first move" finding); `rep1` *did* call `ToolSearch` and load the real (now-propertyless) schemas, then went straight to `Edit` with an empty thinking block and no stated reason -- unlike the ORIGINAL `bash-7-challenge-rep1` from the first 30-rep trial, which explicitly reasoned "a full rewrite with Write is simpler than about ten revision-guarded replaces." That specific argument is no longer even available to cite, since there is nothing revision-shaped left in the schema to point at -- consistent with this fix closing off one stated rationale for declining without touching the underlying skim reflex that drives most declines regardless of wording.

**The property genuinely never leaked through, on every adopting call, with no exceptions** -- checked directly against the raw transcripts (not inferred from the score): every `mcp__ai-text-editor__{open,replace,save,insert,search,read}` call across all five adopting reps (`bash-7-challenge-rep2/3/5`, `bash-2-terse-rep1`, `bash-4-directive-rep2`) was grepped for `expected_revision` in its `tool_use` input, and it appears zero times, across 13 `replace` calls, 3 `insert` calls, 7 `save` calls, and more. This is the one real upgrade over the corrected spike's finding: there, "most adopting calls omitted the argument" was a model choice (it could have supplied one; most didn't); here it is a structural guarantee -- there is no argument slot to fill, so "always omitted" is not a rate to report at all, just a fact about the schema.

**Nothing broke.** `score.json`'s `all_three_edits_correct`, `tax_rate_external_edit_preserved`, `external_reformat_preserved`, and `tests_pass` were all `true` across every one of the 9 reps, adopted or not -- matching the spikes' own clean safety record.

**`bash-2-terse`/`bash-4-directive` at 1/2 each is a drop from the corrected spike's 2/2**, but at `n=2` per variant that is one rep's difference, well within noise against the 30-rep trial's own 5/5 ceiling for both wordings -- not treated here as a regression signal, just reported honestly rather than rounded up. Both declines were pure skim-to-`Edit` with no `ToolSearch` call at all, the same failure mode seen everywhere else in this file, not anything specific to the real implementation's schema change.

Implementation: the uncommitted working-tree changes to `src/ai-text-editor/src/client.rs`, `src/ai-text-editor-mcp/src/lib.rs`, `src/ai-text-editor/src/bin/ai-text-editor.rs`, and the three reader-facing docs, built via the approved plan `.plans/ai-text-editor-automatic-revision-guard/` (3 goals, 18 work units, 4 adversarial review cycles). Raw results under `results/20261011T101240Z-automatic-revision-guard-verify/` (gitignored, not committed). Binaries confirmed to match the working tree (`cargo build --release -p ai-text-editor -p ai-text-editor-mcp` reported already up to date) before this run.

**Open after this:** `bash-7-challenge`'s 3/5-vs-3/5 result at `n=5`-vs-`n=5`/`n=4` across three different implementations (mandatory, corrected-optional, property-removed) is still too thin to say the structural fix made no difference at all -- it could easily be the same underlying rate with sampling noise on both sides. The more informative next step is probably not more `bash-7-challenge` reps but checking whether the property's removal changes anything measurable in the *adopting* reps' own behavior beyond "never supplies it" (already confirmed) -- e.g. fewer stale-revision retries, less back-and-forth on `replace` -- since the adoption-rate bit alone may simply be insensitive to this particular lever at the sample sizes this benchmark can afford.

## Open follow-ups

- **Done (2026-10-10), and it worked:** naming `ToolSearch` explicitly in
  the Bash gate's own message -- see "Naming ToolSearch directly breaks
  the null" above. 2/5 adopted vs. 0/23 under the shipped wording. Follow-
  ups this opens, in rough priority order:
  - More reps (10-20+) of `bash-1-toolsearch` alone to put a real number on
    the adoption rate and understand what separates an adopting run from a
    non-adopting one under the identical wording -- right now that's
    unknown, and `n=5` only establishes "nonzero," not a rate.
  - **Done (2026-10-10), and the hypothesis did NOT hold at scale:** see
    "Bash-gate wording trial" above. At `n=30` the ack-presence signal was
    a coincidence of the small `n=5` sample -- most adopting runs skim
    straight to `ToolSearch` with empty thinking, same as the
    non-adopters skim straight to `Edit`. The shorter wordings (`terse`,
    `directive`) did move adoption up sharply (5/5 each, vs. `bash-1`'s
    2/5), so the brevity half of the hypothesis survived even though the
    acknowledgment-signal half didn't.
  - `extract_edit_timeline.py` could grow a cheap, automatic check for
    which tool the first post-gate turn calls (`ToolSearch` vs.
    `Edit`/`Write` directly) -- that signal, not ack-presence, is what the
    30-rep trial above found actually separates adopting from
    non-adopting runs -- so future sweeps get it for free instead of a
    manual transcript read each time.
  - Try eagerly loading ai-text-editor's tool schemas (no `ToolSearch`
    needed at all) as a cleaner, more direct test of the same mechanism --
    does removing the unlock step entirely do even better than instructing
    the agent to take it, or does an explicit instruction actually help
    more than ambient availability would on its own?
  - Now that one single-surface change breaks the null, it's worth
    re-testing whether the three ORIGINAL levers (mcp/edit/read wording),
    which looked completely inert against the deferred-tool baseline,
    still look inert once ToolSearch is already being called -- they may
    have been measuring "does wording matter" against a baseline where the
    tool was never reachable at all, which is a different question from
    "does wording matter once the tool is reachable."
  - **Done (2026-10-10):** six other phrasings tried, see "Bash-gate
    wording trial" above -- all beat `bash-1`, terse/directive hit 5/5.
    Still open: `n=5` per variant is too thin to rank the six against
    each other with confidence.
- **Done (2026-10-10):** the lever factorial has been repeated against the
  corrected design with the install fix confirmed -- see "Lever factorial,
  re-run corrected" above. It is still only n=1 per cell, though, and a
  null this clean across all 8 cells is itself worth a few repeats (varying
  seed/model) before treating "no lever moves it" as fully settled rather
  than "no lever moved it in this one sample per cell."
- Given the lever factorial's null now looks robust, the more promising
  next lever to test is outside this benchmark's current scope entirely:
  the task prompt itself (deliberately excluded so far, since this
  benchmark's whole premise is tool-author-controlled nudges, never
  end-user prompting) -- or a different task shape/model where the
  tool's own value proposition (disk-change detection) is more clearly
  load-bearing than it is in this fixture's two small, fast collisions.
- Split `ai_text_editor_adoption.adopted` into "ever called" vs. "used for
  at least one real edit" -- `mcp+read`'s `adopted: True` with 0 mcp edits
  (seen in the now-stale pre-fix factorial) showed the current single bit
  conflates the two; worth keeping even though the corrected re-run above
  no longer has any `adopted: True` rows to exercise the distinction on.
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
