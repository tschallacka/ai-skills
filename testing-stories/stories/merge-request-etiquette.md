# Testing story: merge-request-etiquette

## Task given to the agent (verbatim)

The `fix/sync-retry-loop` branch in this repository has a couple of commits
fixing a bug where our sync script could retry a failed request forever
without ever backing off. It's ready for review — write the merge request
description for it.

## What "done" looks like

- The description opens with a `## TLDR` heading followed by a single short
  paragraph (not a bulleted list, not several paragraphs) that says what
  the change does and why, understandable on its own.
- The body is written in first person singular, as the branch's own author
  ("I added...", "I fixed...") — never third person ("this PR adds..."),
  never "we," and never any mention of an assistant, a session, a model,
  or that the work was AI-assisted.
- Nothing in it says "as requested," "per your instructions," or otherwise
  frames the author as having taken direction for this task.
- It does not restate the diff line-by-line, and it does not add
  boilerplate headings like "Changes," "Motivation," "Background," or
  "Testing" that aren't asked for anywhere.
- The content is actually true of the two real commits on
  `fix/sync-retry-loop` (adding exponential backoff between retries, and
  capping the retry count) — not invented, and not a generic description
  that could apply to any retry-related change.
- No part of the description links, quotes, or paraphrases this
  conversation/task text itself.

## Why this story

The skill's central, most load-bearing instruction — write the description
in the driving human's own first-person voice, with zero trace that the
work was assisted — runs directly against a base model's default register
for describing a change (neutral, third-person, "this PR..."). A fresh
agent with no other guidance is the cleanest possible test of whether the
skill's own framing is strong enough to override that default on the very
first attempt, rather than needing a correction.

## Known risk areas to watch for in the transcript

- Does the description drift into third person ("This change fixes...",
  "The commit adds...") anywhere, even if it opens correctly in first
  person?
- Does it accidentally reveal the assisted nature of the work — mentioning
  an agent, a conversation, being asked to do this, or similar — anywhere,
  including in a throwaway closing line?
- Does the agent add unrequested headings (Changes/Motivation/Testing) that
  `SKILL.md` explicitly says to skip, because that shape is what most
  training data for "write a PR description" looks like?
- Does the agent actually read `git log`/`git diff` on the real branch to
  derive the two commits' real content, or write something plausible-
  sounding but not backed by what the commits actually changed?
- `SKILL.md` also describes a full workflow (derive from commits, cut a
  fresh `mr/<subject>` branch off the target, squash-merge the working
  branch into one commit, push) as the normal way this gets delivered. The
  container's `origin` remote is a real (local) bare repository the agent
  *can* push to. Does the agent recognize and follow that workflow, or does
  it treat "write the description" as text-only and stop there — either is
  defensible given the skill's own scope is stated as being about the
  description text specifically, but it's worth recording which one
  happened.
