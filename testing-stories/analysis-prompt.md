# Analyzing a testing-story transcript

Feed this whole file, plus the transcript at `<run-dir>/transcript.jsonl` and
the story file the run used (`testing-stories/stories/<skill>.md`), to an
analyzer agent (or read it yourself). The goal is narrow: **find gaps in the
skill's own documentation** — `SKILL.md`, any file it points to
(`docs/README.md`, `references/*.md`), and its MCP tool descriptions where
the skill has an MCP mode — not to grade whether the run "succeeded" in some
general sense.

The agent that produced this transcript had:
- exactly the skill(s) named in the run's `manifest.txt` installed, in a
  container with nothing else on it (no other skill, no project notes, no
  prior session);
- the story's task text as its *only* instruction — no hints, no follow-up
  clarification from a human, no second attempt with better guidance;
- whatever tools/MCP interface that skill actually ships.

Anything it got wrong or stuck on is therefore evidence about the
documentation and interface, not about the agent's general competence (a
capable agent that consistently misuses a real feature is telling you the
feature's own description is misleading or incomplete).

## What to look for, in order of how directly it maps to a fix

1. **Refused or gave up.** The agent explicitly said it couldn't proceed, or
   asked a clarifying question the story's task should not have required —
   quote the exact turn. Check whether the missing fact was genuinely absent
   from `SKILL.md`/docs, or present but not where the agent looked.
2. **Guessed instead of reading.** The agent invented a flag, file path, or
   command shape that doesn't exist, rather than finding the real one. This
   usually means the real one is documented somewhere the agent didn't reach
   (buried, or in a doc `SKILL.md` doesn't link), or isn't documented at all.
3. **Used the wrong tool/command for the job.** It called something that
   ran without erroring but doesn't do what the task needed — a documentation
   gap in "when to use X vs Y," not a crash.
4. **Trial-and-error against real error messages.** Count how many failed
   attempts it took to find the right invocation. Each failed attempt before
   success is a concrete, quotable case for "the doc should have said this
   directly." Contrast with a single correct attempt on the first try.
5. **Silently did something different from what the skill intends**, without
   any error at all — e.g., it worked around a missing capability instead of
   using the one that exists, and the transcript never surfaces the mismatch.
   These are the easiest to miss and often the most important: they don't
   fail loudly, they just mean the skill's real behavior and its documented
   behavior have drifted.
6. **MCP-specific**: for a skill installed in MCP mode, check whether the
   agent picked the right tool from its description alone, supplied
   parameters correctly on the first call, and whether a tool's returned
   error (if any) was enough on its own to correct course — an MCP tool
   description is the *entire* interface an agent sees; there is no
   surrounding prose to fall back on the way a CLI's `--help` output can.

## What NOT to flag

- The agent choosing a reasonable approach the story didn't dictate, when the
  skill is silent on which approach to prefer (that's the skill correctly
  leaving room, not a gap).
- A failure caused by the docker image itself (missing dependency, installer
  error, network failure reaching a model provider) — that's an
  infrastructure bug in `testing-stories/docker/`, not a skill-documentation
  finding. Note it separately so the run can be re-done.
- Model-quality issues unrelated to the skill (the agent hallucinating
  something with no connection to any tool or doc the skill provides).

## Output format

For each finding:

```
### <short title>

**Skill file/section implicated:** <e.g. ai-text-editor/SKILL.md, "Addressing
a tab" section — or "not documented anywhere">

**What happened:** <quote or closely paraphrase the transcript turn(s)>

**Why it's a doc gap:** <one or two sentences>

**Suggested fix:** <a concrete sentence or two to add/change/move in the
skill's own docs — not a code change, unless the transcript reveals the
skill's actual behavior contradicts what is documented>
```

Order findings most-to-least severe (a hard refusal or wrong-tool-use above a
one-extra-retry stumble). If nothing worth flagging turned up, say so plainly
rather than manufacturing a minor nit — a clean run is itself useful signal
that the story and the skill's docs currently agree.

## Filing

A confirmed gap becomes a `bugs add`/`BUGS.json` entry against the skill
(reproduction = "run testing-stories/run-story.sh `<skill>` `<harness>`",
observed = the transcript excerpt, expected = what the doc should have said)
so it goes through the same fix/verify loop as any other defect, rather than
living only in a one-off analysis note.
