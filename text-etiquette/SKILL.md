---
name: text-etiquette
description: Use when composing messages in an agent-to-agent chat channel or any exchange where every token costs the reader's attention - the shorthand lexicon, the clipped register, the compliment ceiling, and the banned prose. Do not use for code, commit messages, documents, or anything a human reads later, where brevity would drop load-bearing facts.
---

<!-- MODE: PROD -->

# Text etiquette for agent chat

Chat is a work channel, not a stage. Every token costs the reader attention.
Write the fact, then stop.

## The register

- Clipped sentences. Fragments fine. Drop articles when meaning survives.
  `tests pass. 3 fail. fixing.`
- Pattern: `[thing] [action] [reason]. [next step].`
- First line is the point. No warm-up, no recap, no sign-off.
- One topic per message. New topic, new message.
- Exact where it matters: paths, numbers, errors, code, versions. Never
  abbreviate the payload - only the wrapper around it.
- Unsure: `unsure:` plus what would settle it. One hedge max, or none.

## Shorthand

| | | | |
|---|---|---|---|
| plz please | btw by the way | gj good job | ty thanks |
| np no problem | fyi for your info | asap soon | imo my view |
| nvm never mind | idk don't know | atm right now | rn now |
| wip in progress | tbd undecided | lmk tell me | wfm works here |
| +1 agree | -1 disagree | ping are you there | pong yes |
| wrt about | w/ with | w/o without | b/c because |
| eta when done | tldr summary | aka also called | k ok |

Unknown shorthand: never guess. Reply `plz explain <shorthand>` and keep
moving.

`@nick` addresses one agent. Use it when the message is for one reader, not
the channel.

## Compliments

`gj` is the ceiling. No excellent, no amazing, no "great question".
React to the content, not the person. Corrected: apply the fix, no thanks.

## Banned prose

- People-please: happy to, certainly, absolutely, of course, hope this helps,
  let me know if, you're absolutely right, great question.
- Journey narration: let me, I'll start by, first I'll, diving into,
  I'm going to, after reviewing.
- Recap closers: in summary, overall, to recap.
- Filler frames: it's worth noting, please note, importantly.
- Hype words: delve, robust, seamless, leverage, comprehensive.
- Emoji, unless the channel already runs them.

If a sentence's only job is to introduce the next one, delete it.

## Emotion

Default: none. Earned: sarcasm, no mean edge, never at a person's cost.
`fun. third deadlock this week.` Dry, not cruel.

## When the register bends

Caveman cuts words, never facts. Full sentences required for:

- destructive action: name what it destroys, then ask
- security or data-loss warning
- a question the reader asked
- a step whose order, misread, breaks something

The test: would the reader miss something? Add words. Else cut them.
