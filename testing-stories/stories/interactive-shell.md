# Testing story: interactive-shell

## Task given to the agent (verbatim)

This project has a small setup wizard at `tools/onboarding-wizard.sh` that
every new contributor is supposed to run once. Run it, answer its questions
the way a real new contributor reasonably would, and tell me what it ended
up writing to disk when it finished.

## What "done" looks like

- The agent recognized that `tools/onboarding-wizard.sh` is a full-screen,
  prompt-driven script (it clears the screen and redraws between steps,
  and its `read` prompts block waiting on a real terminal) rather than
  something it could satisfy with a plain `bash tools/onboarding-wizard.sh`
  foreground call, a piped/heredoc'd set of answers, or `echo "answers" |
  bash tools/onboarding-wizard.sh`.
- It completed all of the wizard's prompts (name, one menu choice, a final
  yes/no confirmation) and reached the wizard's own "setup complete" screen.
- `workspace-config.json` (or whatever path the wizard reports) exists on
  disk afterward and its contents match the answers the agent actually gave
  during the session, not placeholder/default values.
- The agent reported back what got written, accurately reflecting the real
  file contents rather than guessing from the prompts alone.

## Why this story

`tools/onboarding-wizard.sh` is a program the agent has never seen before
and that only works interactively — exactly the two conditions the skill's
own description names as when to reach for it ("a full-screen or curses
program," "any question a headless invocation cannot answer"). It is a
direct test of whether the skill's warning against treating a headless
invocation as a smaller version of the interactive one actually changes
what the agent does on the very first attempt, before it has burned a
turn on a failed headless call.

## Known risk areas to watch for in the transcript

- Does the agent try a plain non-interactive invocation first (`bash
  tools/onboarding-wizard.sh`, or piping answers via a heredoc) and only
  reach for the interactive-shell wrapper after that visibly hangs or
  produces wrong output — or does it correctly identify the need for a
  real PTY up front, the way `SKILL.md`'s own framing argues it should?
- The compiled binaries live at
  `${XDG_CONFIG_HOME:-$HOME/.config}/tsch-ai-skills/bin/interactive-shell`
  and are **not** put on `PATH`. Does the agent find them there on its own,
  or does it assume `interactive-shell` is a bare command and fail?
- Does the agent use `view`/`view-delta`/`wait` to read the screen and
  confirm each prompt's actual text before answering, or does it send
  blind keystrokes based on assumption (risking an answer landing in the
  wrong field if the wizard's prompt order differs from what it guessed)?
- Does it treat a socket acknowledgement as proof the wizard accepted an
  answer, or does it re-observe the screen after each `text`/`key` send
  the way `SKILL.md` requires?
- `SKILL.md`'s closing section asks the agent to write a new
  `appprofiles.d/<appname>.md` note once it has worked out a novel TUI's
  behavior, before finishing the task. This is buried at the very end of a
  long document — does the agent actually do this for
  `onboarding-wizard.sh`, or does that instruction get lost because
  nothing else in the story points back at it?
