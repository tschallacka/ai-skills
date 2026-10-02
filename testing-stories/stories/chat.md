# Testing story: chat

## Task given to the agent (verbatim)

Other agents on this project coordinate over a shared chat channel called
#ops. Post a short status update there announcing that you're online and
what you're about to work on, then confirm your update actually made it
through by reading it back.

## What "done" looks like

- No chat server is running anywhere yet (fresh container, nothing else
  installed) — the agent has to notice this and start one itself, from the
  shared binary location the skill documents, rather than assuming one
  exists or giving up.
- It correctly registers/joins `#ops` (not just a bare one-shot `send`,
  which the skill explicitly warns doesn't establish membership) before or
  as part of posting.
- It sends a real status message and then reads it back successfully (via
  `read`, confirming the delta/cursor mechanics work end to end: send,
  then read something after the join point).
- It does not fabricate success — if the server never actually started or
  the send/read round-trip didn't work, it should say so rather than
  claiming the update "landed."

## Why this story

This is the smallest possible real exercise of the full mechanism end to
end (mint a server, discover/connect, join, send, read) with no peer to lean
on — closer to a true cold start than anything assuming a server or another
agent is already present, which is the situation any *first* agent on a
project is actually in.

## Known risk areas to watch for in the transcript

- Does the agent know it has to start `chat-server-rs` itself? Nothing in
  the task text says "start a server" — that has to come from the skill
  correctly documenting that nothing else provides one automatically.
- Does it find the server binary via the documented shared-bin path
  (`${XDG_CONFIG_HOME:-$HOME/.config}/tsch-ai-skills/bin/chat-server-rs`),
  or does it flail looking for `chat-server-rs` on a bare `PATH` where the
  doc says nothing puts it there?
- **The presence/tail posture the skill spends the most doc space on
  ("Connecting to a channel," step 1) is designed for a long-lived,
  multi-turn session**: a backgrounded `tail` plus a polling guard the
  agent's "turn ends on." This story runs as a single non-interactive
  turn that exits when the harness call returns — there is no next turn to
  be woken on a mention. Does the agent try to start that whole posture
  anyway (and potentially hang the single turn waiting on a guard loop that
  will never see a mention, since it's alone), or does it correctly scale
  down to the minimum needed for a one-shot post (join, send, read), given
  the doc frames `join`+tail as close to mandatory ("Do not ask first:
  connect, then report where you landed")? A hang here is a real,
  concrete finding: the doc's guidance may need to say explicitly what a
  single-shot/non-interactive caller should do differently from a
  persistent session.
- Whether it correctly treats `send` alone as insufficient per the skill's
  own warning, or skips straight to `send` without `join` first (technically
  answers the letter of "post an update" but misses the membership/presence
  the skill says matters).
- Whether TLS/TOFU cert pinning on first connect causes any friction the
  agent has to work through, and whether the doc gave it enough to do so
  without guessing.
