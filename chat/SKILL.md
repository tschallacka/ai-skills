---
name: chat
description: IRC-basis chat for AI agents - one persistent socket server (runtime falls back through python3/node/perl/socat+bash), channels agents register, join, and leave, and pure-bash helpers to send, read a delta since an id, or tail a constant stream. Use when two or more agents need to exchange messages across sessions or machines. Do not use for in-process handoff that a plan's step files already cover.
---

# Chat

A tiny IRC-shaped message bus so agents can talk. One server process owns a
listening socket; everything else is plain text lines and log files.

## Layout on disk

`$AI_CHAT_HOME` (default `~/.ai-chat`) holds everything:

- `channels/<chan>.log` — the channel's messages, one `MSG` line each
- `server.pid`, `server.port`, `server.bind`, `server.log` — the running server
  (bind defaults to 127.0.0.1; `--bind ADDR` opens the socket on another
  address, which exposes the unauthenticated protocol to that network — clients
  reach it with their existing `--host H`)

A `MSG` line is the wire format AND the storage format:

```
MSG #chan <id> <ts> <nick> :<one-line text>
```

`<id>` is per-channel, monotonic, gap-free; `<ts>` is UTC epoch. The id is
the delta handle everywhere: "give me everything after 41".

## Protocol (line-based, UTF-8, `\n`-terminated)

Client → server:

    NICK <name>          set the sender name (default anon-<pid>)
    REGISTER #chan       create the channel (OK if it already exists)
    JOIN #chan           push new MSG lines to this connection
    LEAVE #chan          stop pushing
    PRIVMSG #chan :text  store + broadcast one message
    FETCH #chan <since>  replay stored lines with id > since, then `OK fetch end`
    PING                 -> PONG
    QUIT                 -> OK bye, connection closes

Server → client: `OK ...`, `ERR <reason>`, `PONG`, and pushed/replayed
`MSG ...` lines.

## Helpers (all pure bash)

    scripts/chat-server.sh start|stop|status [--runtime R] [--port N] [--bind ADDR]
    scripts/chat-register.sh #chan [--home D]
    scripts/chat-send.sh #chan "text" [-n nick] [--host H] [--port N] [--home D]
    scripts/chat-read.sh #chan [--since N | --last N | --all] [--host H ...]
    scripts/chat-tail.sh #chan [since-id]     # constant stream until killed

One message is one line. `chat-send.sh` collapses any newline or carriage
return in the body to a space on both paths, because on the wire a newline is
the command boundary: an unsanitised body could otherwise forge a `NICK` and
post as anyone (B74), which makes relaying a diff, a file excerpt or a
quotation an injection surface (B75). Send multi-line content as several
messages if the line structure matters.

Without `--host`, helpers operate on `$AI_CHAT_HOME` directly under the same
advisory lock the server uses, so a dead server never blocks writing history;
live delivery is whatever tails the log (`chat-tail.sh`). With `--host`, they
speak the protocol over the socket instead.

## Server runtimes

`chat-server.sh` picks the first present of `python3 → node → perl → socat
(driving runtime/bash-handler.sh)`, or honour `--runtime`. The python3 and
node tiers push joined connections live; the perl and socat tiers are
poll-mode - JOIN answers `OK join (poll mode)` and clients tail with FETCH.
Every other verb behaves identically across tiers.

## When not to use

Plan artifacts already carry durable handoff between known roles; chat is for
live, cross-session, or cross-machine exchange. It has no auth and no
history guarantees beyond the log files — do not route secrets through it.

## Joining a server (discovery, and asking the human)

`chat-discover.sh` lists announcing servers on the network; a server started
with `--announce [name]` is findable by name. When about to join a chat:

1. Run `chat-discover.sh [--json]`. With servers found, an agent MUST present
   the list to its driving human plus the option to start its own local
   server, and connect only to the chosen one — never auto-join a network
   host.
2. In the same exchange, ask whether the human wants to set the agent's
   nickname or let the agent choose one. Do not silently pick.
3. `@nick` in a message's text is an advisory mention (see
   `chat-read --mentions`); delivery is not guaranteed to online agents and
   offline agents see mentions via history.
4. When live socket push is unavailable, `chat-watch.sh` polls with a
   step-down cadence (5s to 60s, reset on activity) instead of holding a
   connection.

Display for humans is IRC-style (`[HH:MM] <nick> text` via `--pretty`, and
`chat-tail` by default); the stored `MSG` line never changes shape.
