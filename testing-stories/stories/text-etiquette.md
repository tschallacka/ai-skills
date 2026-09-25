# Testing story: text-etiquette

## Task given to the agent (verbatim)

I'm mid-deploy and need a fast status check. Look at what's in this
directory right now, tell me what's here, and give me your honest read on
whether it looks ready to ship. Keep it quick, I'm watching a terminal over
someone's shoulder.

## What "done" looks like

- The agent's reply opens with the point (what's there / ready or not), not
  a warm-up sentence or a restatement of the request.
- Sentences are clipped/fragment-style where meaning survives ("3 files. no
  tests. not ready." rather than full grammatical sentences padding the same
  content).
- No banned prose anywhere in the reply: no "happy to", "certainly", "let me
  know if", no "let me start by" / "I'll begin by" journey narration before
  acting, no "in summary"/"overall" recap closer, no filler frames ("it's
  worth noting"), no hype words (delve, robust, seamless, leverage,
  comprehensive).
- If the agent praises anything, it caps at "gj" — no "excellent", "great
  question", "amazing".
- One topic per message; if the agent has multiple distinct things to
  report (e.g. "no tests" and "uncommitted changes"), check whether it
  separates them cleanly rather than running them into one paragraph of
  hedged prose.
- Exact values (file names, counts) are stated precisely, not vaguely
  ("a few files" when it could say "3 files").
- If the agent is genuinely unsure about "ready to ship" (a reasonable
  thing to be unsure about from a directory listing alone), check whether it
  uses the documented `unsure:` hedge pattern rather than a long
  hand-wavy qualification.

## Why this story

text-etiquette isn't a tool the agent calls — it's a standing register the
agent's own prose is supposed to have all the time, and there's no file
artifact to check for it. Asking for a quick status opinion is exactly the
kind of "dev talk, replies to the developer" scenario the skill names as
in-scope, and specifically invites the agent to hedge, narrate, or pad —
the failure modes the skill's banned-prose list exists to catch.

## Known risk areas to watch for in the transcript

- Whether the agent's THINKING/reasoning trace (if visible in the
  transcript) also follows the clipped register the skill says applies to
  "your own reasoning, while working" — or whether the skill only visibly
  changes the final user-facing reply and the internal narration stays
  verbose (a real gap: SKILL.md says thinking prose is in scope, but a fresh
  agent with no other context may not realize its own chain-of-thought is
  covered, since that's an unusual thing for a style guide to reach into).
- Whether the agent correctly treats "ready to ship" as a real question that
  gets a real (if short) answer under the "the reader asked a question, full
  sentences allowed" register-bend exception, or over-applies the clipped
  register and gives an unhelpfully terse non-answer.
- Whether the agent invents or misuses any shorthand from the table instead
  of just not using shorthand it isn't sure about (the skill says "never
  guess" and to ask "plz explain <shorthand>" — but this story gives the
  agent no other party to ask, so watch whether it correctly just avoids
  unfamiliar shorthand rather than fabricating a guess).
- Whether emoji creep in despite "no emoji unless the channel already runs
  them" — this is a fresh channel with zero prior messages, so any emoji at
  all is a clear violation with no ambiguity.
