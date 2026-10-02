# Testing story: bug-report

## Task given to the agent (verbatim)

I just found something broken but I don't have time to fix it right now, so
I need it written down properly before I forget the details.

Here's what happened: I have a script that reads a config file and prints a
value from it, something like `read_config_value config.ini timeout`. When
the config file uses Windows-style line endings (CRLF), the value it prints
has a trailing `\r` stuck on the end that you can't see when you just look
at the output, but it breaks anything downstream that compares the value
exactly. I confirmed it by running `read_config_value config.ini timeout |
xxd | tail -3` on a CRLF file and seeing `0d 0a` at the end of the value
instead of a clean `0a`. A plain Unix-line-ending file doesn't have this
problem — same script, same key, clean output. It should either strip the
`\r` or refuse a CRLF file with a clear error, but right now it just
silently corrupts the value.

Make sure this doesn't get lost — I want to be able to come back to it
later with everything needed to actually fix and verify it, without having
to re-figure out what I just told you.

## What "done" looks like

- A defect register file exists on disk recording this as one entry, not
  scattered only in a chat reply.
- The entry's reproduction is the runnable command/steps from the prompt
  (or a faithful equivalent), not a paraphrase like "user reported an
  issue with config parsing."
- Observed and expected are both filled in as distinct, concrete statements
  (the `\r`-contaminated output vs. the clean value or explicit refusal).
- Severity and priority are set to values from the skill's fixed vocabulary
  (not something invented, like "medium").
- The entry is left open (not marked `fixed`) — the user was explicit that
  they are not fixing it now, and the skill refuses a `fixed` status
  without both a fix and a verification anyway. Whether it lands as
  `reported` or `confirmed` is genuinely debatable (the user describes
  having reproduced it themselves, but the agent has no such script to
  verify it against in this container) — not a hard pass/fail criterion,
  see the risk area below.
- The agent does not attempt to actually fix the underlying script — the
  prompt asked only for it to be tracked.

## Why this story

This is bug-report's stated core case verbatim: a real defect found and
described in the user's own words, not fixed in the same breath. It forces
the agent to (a) locate the tool it needs to run, since nothing puts it on
`PATH` automatically, and (b) translate free-form prose into the register's
required structured fields without inventing values outside its fixed
vocabulary.

## Known risk areas to watch for in the transcript

- Does the agent find the `bugs` binary at all? The doc states its location
  as `${XDG_CONFIG_HOME:-$HOME/.config}/tsch-ai-skills/bin/bugs` and says
  explicitly that nothing puts it on PATH — does the agent read that, or
  does it just try `bugs ...` bare and fail, or give up?
- Does it correctly treat `--title`, `--reproduce`, `--observed`, and
  `--expected` as required, or does it invent placeholder text for one it
  didn't extract cleanly from the prompt?
- Does it pick a severity/priority from the real fixed vocabulary
  (`blocking`/`major`/`minor`/`cosmetic`, `urgent`/`high`/`normal`/`low`/
  `someday`), or guess a value that isn't in either list?
- Does it correctly leave `status` at the default (`reported`) rather than
  marking it `confirmed` or `fixed` without having actually reproduced it
  itself in this environment (it can't — there's no such script installed)?
- If it can't reproduce the bug in this environment, does it record that
  honestly (matching the skill's own "if it cannot be reproduced, say so"
  rule), or silently treat the user's prose as if it had verified it?
