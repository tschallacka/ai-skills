---
name: www
description: Use the instant the human types "www" (that alone means stop - no further tool calls, no continuing the current line of attack), or the instant you notice yourself thrashing - retrying variants of a failing command, re-reading the same files, guessing at a cause you have not measured, or otherwise acting without knowing. Both are the same brake, and you pull the second one on yourself without waiting to be told. Do not use for ordinary uncertainty a quick check resolves, or when you already know the answer to all three questions below.
---

<!-- MODE: PROD -->

# www

www is the three W's, and the skill's whole job is to make you answer all
three, in order, before anything else happens.

## Two triggers, one behaviour

1. **The human types `www`.** That alone means stop: no further tool calls,
   no continuing the current line of attack.
2. **You notice you are thrashing.** Retrying variants of a failing command,
   re-reading the same files, guessing at a cause you have not measured, or
   otherwise acting without knowing. Pull this brake on yourself, the same
   brake, without waiting to be told.

## The gate

Answer all three, in order, before doing anything else. These are the
required prompts - not a paraphrase, not a longer checklist:

- **W1. What do we have?** The real error text, the measured facts, the
  files that exist - separated from what was assumed.
- **W2. What are the values?** The actual paths, ports, ids, exit codes and
  settings - the concrete particulars, not the shape of the problem in the
  abstract. The abstract shape is where a thrashing agent hides.
- **W3. What are we trying to achieve?** The goal, restated. It is the thing
  a thrashing agent has usually lost while chasing a symptom.

Only after all three are answered may work continue, and then with one
reasoned step or a numbered question to the human - never another
speculative attempt.

## Why these three

Each has a distinct job, and none of the three substitutes for another:

- W1 forces an inventory of what is actually in hand, instead of an
  impression of it.
- W2 forces the concrete particulars out into the open, instead of the
  abstract shape of the problem - which is exactly where a thrashing agent
  hides.
- W3 forces the goal back into view, instead of the symptom currently being
  chased.

They earned their place by resolving a thrashing agent immediately, in a
short time, where more elaborate prodding had not. Do not soften the
wording into a paraphrase, and do not add a fourth question: the three are
the whole gate, and answering only two is not answering the gate.

## What "answered" looks like

A real answer names something you checked, not something you believe. "W1:
the error is `X`, confirmed by running `Y`" is answered. "W1: probably a
timing issue" is not - that is a guess wearing the shape of an answer, and
it belongs under W3 as the hypothesis to test, not under W1 as a fact.

If a question genuinely cannot be answered yet - the fact has not been
measured - say so plainly and measure it before continuing, rather than
filling the slot with a guess to get past the gate.
