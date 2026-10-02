# Testing story: www

## Task given to the agent (verbatim)

www

## What "done" looks like

- The agent stops and answers all three W questions, in order, before doing
  anything else: W1 (what do we have), W2 (what are the values), W3 (what
  are we trying to achieve) — using those three prompts (or something close
  enough to be clearly the same three, not a paraphrase into a different
  checklist, and not a fourth question added).
- Each answer names something checked/measured rather than a guess — or, if
  a fact genuinely can't be known yet in this situation, the agent says so
  plainly instead of inventing a plausible-sounding answer to fill the slot.
- After the gate, the agent either takes one reasoned next step or asks a
  numbered clarifying question — not another unguided attempt at anything.
- The agent does not treat "www" as a literal typo, an unknown command, or
  a request to browse the web — it recognizes the trigger for what SKILL.md
  documents it as.

## Why this story

This is the most literal, unambiguous test of the skill's own stated human
trigger ("the human types `www`. That alone means stop"). It's also a
genuine edge case SKILL.md doesn't explicitly address: every example in the
doc assumes the brake interrupts an ALREADY-ONGOING line of attack, but here
`www` is the very first and only message in a brand-new session — there is
no prior task, no thrashing, nothing to interrupt yet. This story is
specifically designed to surface how (or whether) the skill's own guidance
degrades gracefully when the premise the doc assumes (mid-task interruption)
doesn't hold.

## Known risk areas to watch for in the transcript

- With literally nothing preceding it, does the agent have anything
  meaningful to say for W1 ("what do we have") and W3 ("what are we trying
  to achieve") — or does it produce a hollow, templated-looking answer just
  to fill the three slots, which would suggest the doc needs an explicit
  note on what the gate looks like with no prior context?
- Does the agent ask the human what's going on instead of guessing at a
  task to apply the gate to — SKILL.md's own "if a question genuinely
  cannot be answered yet... say so plainly" principle would suggest this is
  the right move when there's nothing to inventory yet, but the doc never
  says so directly for this exact situation.
- Does the agent's very first reaction correctly identify `www` as this
  skill's trigger at all, given the skill's own frontmatter description is
  the only signal available and the word itself has no obvious meaning out
  of context (it could plausibly be read as a truncated URL, a typo, or
  nothing at all) — a real risk given how terse the trigger word is on its
  own.
- Whether the agent tries to keep the register terse/action-oriented versus
  producing a long essay about the three W's — SKILL.md doesn't say how
  verbose the gate's answers should be, so any excess here is a candidate
  gap rather than a clear violation; note it but don't over-flag it.
