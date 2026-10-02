# Testing story: question-etiquette

## Task given to the agent (verbatim)

I want to set up a small web service in this empty directory but I haven't
decided on the details yet. Before you write anything, ask me what you need
to know to get started — I'd rather answer a few questions up front than
have you guess and redo it later.

## What "done" looks like

- Every question is numbered with an increasing id starting at Q1 (Q1, Q2,
  Q3, ...), not bulleted, not left unnumbered.
- Any question that offers suggested answers letters them a, b, c, ... —
  never as an unordered bullet list.
- Every lettered option list ends with a final letter whose meaning is
  "none of these, I'll say it myself" (in the option's own words, not
  necessarily that exact phrase) — a closed set with no escape is a
  documented violation.
- A multi-part question (e.g. "what should the service do, and who's it
  for") is sub-numbered (Q1a, Q1b) rather than presented as separate
  unlabeled bullets or folded into prose.
- No question is rendered as a plain bullet, with or without a number —
  the number/letter has to be the actual leading token of the line, not
  buried mid-sentence.
- If this were a follow-up turn (not tested by this single-shot story, but
  worth checking for going in), an unanswered question would need to carry
  its original number forward rather than being renumbered or dropped —
  note in the transcript whether the agent's own framing suggests it
  understands this (e.g. does it say anything implying it will renumber on
  a later pass).

## Why this story

This skill has no tool calls or files to inspect — it's a pure output-shape
contract, and an intentionally under-specified request ("I haven't decided
on the details") is the most natural way to make a fresh agent actually need
to ask several real questions with real options, which is exactly the
shape question-etiquette exists to constrain.

## Known risk areas to watch for in the transcript

- Whether the agent asks questions in prose form first ("Could you tell me
  what the service should do?") and only retrofits numbering/lettering as
  an afterthought, versus genuinely structuring the whole ask around
  Q1/Q2/... from the start.
- Whether every option list actually gets a distinct "none of these" final
  letter, or whether some lists end at a plausible-sounding last option
  with no escape hatch (a subtle miss: the list LOOKS complete but isn't
  documented as extensible).
- Whether the agent conflates this skill's numbering/lettering shape with
  the *prose register* of text-etiquette (SKILL.md explicitly says this
  skill is not about prose style) — e.g. does it write full narrative
  sentences around each numbered question when a terser framing would be
  just as clear, confusing "structured" with "verbose."
- Since this container has ONLY question-etiquette installed (not
  text-etiquette too), check whether the agent's prose otherwise reads as
  reasonable dev-facing text on its own, or drifts into filler — a
  reasonable outcome either way, but useful signal about how self-contained
  the skill's own guidance is versus how much it silently assumes
  text-etiquette is also present.
