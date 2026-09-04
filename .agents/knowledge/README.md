<!-- MODE: DEV -->
# Knowledge

Things that were **measured**, that cost real time to establish, and that the
next agent would otherwise rediscover from scratch.

This is not a changelog and not a place for narrative. Each entry answers one
question with numbers, says how it was measured, and says what it means for
work in this repository. If an entry cannot be re-checked from what it
contains, it does not belong here.

## What belongs

- A **limit or boundary of the tools we run**: a cap, a timeout, a size, a
  version-dependent behaviour. Anything an agent will hit and misdiagnose.
- A **behaviour that is silent**. The expensive ones are never the loud
  failures; they are the ones that look like success.
- A **refuted belief**, with the measurement that refuted it. Recording what is
  *not* true stops the same investigation being run twice.

## What does not belong

- Anything the repository already records. Code structure, contracts, and the
  history are in the code, `CODE-STYLE.md`, `CODE-CONTRACTS.md`,
  `.agents/MAINTAINER.md` and the git log.
- Queued work. That is `TODO.json`.
- Defects. That is `BUGS.json`, which asks for a reproduction and a
  verification this format has no place for.
- Anything unmeasured. A suspicion is not knowledge; leave it in the
  conversation or file it as a task to go and measure.

## Entries

| entry | question it answers |
|---|---|
| [agent-read-limits.md](agent-read-limits.md) | How much of a file does each agent actually read, and does it say when it stops? |
| [unix-sockets-across-platforms.md](unix-sockets-across-platforms.md) | Why does a socket bind that works on Linux fail on macOS, and what actually ports? |
| [github-ci-runners.md](github-ci-runners.md) | What are GitHub's runners actually like — arch, speed, `$TMPDIR`, what a resource cap can enforce — and which of those breaks tests? |

## Format

One file per question, named for the subject rather than the occasion. Open
with the answer, not the story: a reader who stops after the first paragraph
should already have the fact. Then how it was measured, precisely enough to
repeat. Then what it means here.

Give every number a unit and a date. A cap that was true in September is a
claim about September, and the file should make that obvious rather than
implying permanence — see the version columns in the entry above.
