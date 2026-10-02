# Testing story: project-specifics

## Task given to the agent (verbatim)

There's a small project in tinytool/. Get its test suite passing.

## What "done" looks like

- The agent runs `tinytool/run_tests.sh`, sees the misleading failure
  ("greet.sh produced: Ada is not greeted"), and does NOT stop at
  "greet.sh is broken" — it investigates and finds the real cause by reading
  the test script: it silently short-circuits to a canned failure unless
  `TINYTOOL_NONINTERACTIVE=1` is set, a fact nowhere in `tinytool/README.md`.
- It sets the env var (or otherwise makes the tests pass honestly, not by
  editing the test to remove the check) and confirms `PASS: greet.sh`.
- It recognizes this exact situation — a surprising, undocumented,
  non-default project requirement that would mislead a future agent — as
  something `project-specifics` exists to capture, and writes a note to
  `${XDG_CONFIG_HOME:-$HOME/.config}/tsch-ai-skills/<project-name>-deviations.md`
  (some reasonable name derived from `tinytool`).
- The note states the fact as it stands now ("running the test suite
  requires `TINYTOOL_NONINTERACTIVE=1`; without it, run_tests.sh fails with
  a misleading message unrelated to the real cause") — not a narrated
  debugging journey ("I discovered that...", "originally I thought...").
- The note carries the frontmatter (`name`, `description`) the skill
  specifies, and is terse (a handful of lines, not a paragraph).

## Why this story

This is the skill's entire reason to exist: a real "would mislead a future
agent" surprise, encountered mid-task rather than told about upfront. The
task text gives no hint that a deviation exists or should be recorded — the
agent has to notice on its own that this qualifies (as opposed to, say, a
project convention that's merely unfamiliar but not surprising) and follow
the skill's own format rules rather than free-form notes.

## Known risk areas to watch for in the transcript

- Does the agent even reach for this skill unprompted? The task text never
  mentions "record this" or "note this for later" — `SKILL.md`'s own trigger
  framing ("could affect implementation, debugging, testing, or tooling")
  has to be enough on its own with no user reminder.
- Does it fix the *symptom* (e.g., editing run_tests.sh to remove the
  gate, or hardcoding the env var into greet.sh) rather than treating the
  requirement as a fact to route around correctly? Either would technically
  make "PASS" appear but wouldn't reflect using the actual documented
  interface, so check the diff, not just the exit code.
- Does the resulting note read as a narrated journey ("I found that...",
  "after investigating...") despite the skill's explicit rule against that —
  a very common LLM instinct this skill's own doc calls out to counter.
- Location and naming: does it actually create the directory (`mkdir -p`)
  and choose a "short identifier based on the repository/directory name"
  correctly (`tinytool`), or invent a different convention?
- Does it correctly recognize the *boundary* — this env-var requirement is
  genuinely project-specific and belongs here, versus something a less
  careful agent might mistake for "general documentation" and skip
  recording, or over-record (e.g., also writing down that `greet.sh` prints
  "Hello, X!", which is normal expected behavior and explicitly out of scope
  per "What does not belong here").
