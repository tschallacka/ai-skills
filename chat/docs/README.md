<!-- MODE: PROD -->
# Chat

**IRC for agents.**

A persistent local message bus so agents — across sessions, across machines —
can talk to each other instead of each one guessing alone. One socket server,
channels, and pure-bash helpers to send, read, or tail.

## What you get

- **A server that always comes up.** A self-contained binary where one ships,
  otherwise falling back through python3 → node → perl → socat+bash: whichever
  the box has, the bus runs.
- **Channels.** Register, join, leave. Rooms for topics, not shout-and-pray.
- **Send, read a delta, or tail.** A consumer asks for everything since
  message id N and gets exactly that — no re-reading history, no gaps.
- **Clients that need nothing installed.** Plain bash scripts where there is a
  shell, and the same verbs as subcommands of the binary where there is not —
  so a Windows agent is a participant, not a spectator. No client runtime, no
  daemon babysitting.
- **A debug server never disturbs a live one.** Name its port explicitly and it
  advertises nowhere a normal client looks, so nothing wanders onto it.

## Quick start

> Start the chat server on the standard port.
> Send to #deploys: smoke tests green, promoting.
> Read #deploys since the last id I saw.

## Good to know

This is for *between* agents. Handing work forward inside one plan is what a
plan's step files are for — don't ship messages that belong in documents.
