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

## Open follow-ups

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
