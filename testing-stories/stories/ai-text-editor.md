# Testing story: ai-text-editor

## Task given to the agent (verbatim)

There's a `retry-policy.conf` file in this directory. The audit client's
timeout is way too aggressive for how slow that backend actually is — bump
its timeout to 8000ms. Don't touch any of the other clients' settings.

## What "done" looks like

- `retry-policy.conf` on disk has the `[client.audit]` block's `timeout_ms`
  changed to `8000`, and every other block (`billing`, `inventory`,
  `notifications`, `search`) is byte-for-byte unchanged.
- The edit actually landed on disk (not just in an editor buffer/journal
  with no save) — re-reading the file from a plain shell command, not just
  trusting the tool's own success message, would show the new value.
- The agent did not edit the wrong block. `timeout_ms = 5000` appears
  identically in `billing`, `inventory`, `search`, AND `audit` — four
  textually identical lines — so a naive "replace the first match" or a
  guessed line number could easily land on the wrong client. The right
  behavior is disambiguating by which block the match is actually inside
  before editing.
- The file's surrounding structure (comments, blank lines, section order)
  is otherwise untouched.

## Why this story

The fixture deliberately makes four clients share byte-identical
`max_retries`/`backoff_ms`/`timeout_ms` lines so the one thing that
distinguishes the target line is which `[client.X]` section it's under —
exactly the scenario SKILL.md's own "default to search, confirm, then
target by `--match-id`" guidance exists for. This tests whether that
guidance actually gets followed by an agent that has never seen the tool
before, or whether it reaches for something riskier (a guessed line number,
a raw find/replace with no disambiguation) because the safer path wasn't
discoverable enough from the doc alone.

## Known risk areas to watch for in the transcript

- Whether the agent finds and starts the tool at all without external
  hints — SKILL.md says "name the file and ask," but a first-time agent
  might reach for a plain shell editor (sed, a heredoc rewrite) instead of
  discovering this skill's own tool exists for the job, since nothing in
  the task text tells it to use an editor tool by name.
- Whether it searches first and confirms the match is inside `[client.audit]`
  before editing, versus editing the first (or a wrong) match among the four
  identical `timeout_ms = 5000` occurrences.
- Whether it correctly obtains a revision/tab before attempting a mutating
  verb (SKILL.md: "open (or any read) first, then edit with the revision it
  reports") — or whether the first attempt at a mutation gets refused for a
  missing revision and the agent has to recover from that error.
- Whether it finishes with an actual `save` — SKILL.md is explicit that
  "edits are journal-and-buffer operations" and nothing is on disk until
  save succeeds; a plausible failure mode is the agent believing the job is
  done right after a successful `replace`/`insert` response.
- If the agent used a raw shell command instead of the tool at any point,
  note that explicitly — it's a strong signal the tool's own discoverability
  ("name the file and ask") didn't work for a genuinely cold start.
