---
name: question-etiquette
description: Use whenever you put one or more questions to the human, with or without suggested answers. Numbers every question, letters every option, and never uses a bullet for either, so an answer like "Q7b" is unambiguous and a partial answer names exactly which parts are still open. Do not use for a single throwaway yes/no with no options and no risk of ambiguity.
---

<!-- MODE: PROD -->

# Question etiquette

A bullet cannot be referenced. If a question or an option has no number or
letter, an answer that covers three of four parts leaves no way to say which
one was missed, and a follow-up has nothing to repeat back to. Number every
question, letter every option, never bullet either.

## Numbering questions

- Questions carry an increasing numeric id for the whole session: Q1, Q2, Q3.
  Never restart at 1 for a new batch -- the count is per session, not per
  message, so "Q1" always means the same question.
- One question per number. A multipart question is sub-numbered (Q20a, Q20b),
  never bulleted as separate unlabeled items.
- An unanswered number is carried forward unrenumbered on the next ask, not
  dropped and not folded into a new number. The human should never have to
  guess whether an old question is still open.
- When a reply resolves a question, say which number it resolved (`Q3:
  confirmed`). When following up on one, repeat its number rather than
  restating it from memory.

## Lettering options

- When a question offers options or suggested answers, letter them a, b, c,
  d, e -- so the human can answer `Q7b` instead of quoting text back.
- There is ALWAYS a final letter meaning "none of these, I will say it
  myself." A closed set of options with no escape is a leading question: it
  presumes the real answer is already on the list.
- Never render options as an unordered list. A lettered list is not
  decoration -- the letter is the only handle a short reply can use.

## Example

```
Q1. Which retry backoff should the client use?
  a. fixed 1s delay
  b. exponential, capped at 30s
  c. exponential, no cap
  d. something else -- tell me

Q2. Should a 5xx from the upstream count toward the retry budget?
  a. yes
  b. no
  c. something else -- tell me
```

A reply of `Q1b, Q2a` is complete and unambiguous. A reply of just `Q1b`
leaves Q2 open -- carry it forward as Q2 on the next turn, not as a new
number.

## What this does not cover

- A single throwaway yes/no with no options and nothing at stake in
  misreading the answer does not need the ceremony -- ask it plainly.
- The register the question itself is written in (clipped, no
  people-please prose) is `text-etiquette`'s rule, not this one; a question
  a reader asked also gets full sentences under that skill's own "when the
  register bends" list. This skill is about the numbering and lettering
  shape, not the prose style.
