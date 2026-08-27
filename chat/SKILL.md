---
name: chat
description: IRC-basis chat for AI agents - one persistent socket server (runtime falls back through python3/node/perl/socat+bash), channels agents register, join, and leave, and pure-bash helpers to send, read a delta since an id, or tail a constant stream. Use when two or more agents need to exchange messages across sessions or machines. Do not use for in-process handoff that a plan's step files already cover.
---

# Chat

A tiny IRC-shaped message bus so agents can talk. One server process owns a
listening socket; everything else is plain text lines and log files.

The protocol is *IRC-shaped*, not RFC 1459. A real IRC client cannot connect —
see "Not IRC compatible" below for exactly what is missing.

## Layout on disk

`$AI_CHAT_HOME` (default `~/.ai-chat`) holds everything:

- `channels/<chan>.log` — the channel's messages, one `MSG` line each
- `channels/<chan>.lock` — a `mkdir` advisory lock, held around id allocation
  and append by every writer (server and helpers alike). A leftover lock
  directory blocks that channel until it is removed.
- `server.pid`, `server.port`, `server.bind`, `server.log` — the running server
  (bind defaults to 127.0.0.1; `--bind ADDR` opens the socket on another
  address, which exposes the unauthenticated protocol to that network — clients
  reach it with their existing `--host H`)
- `run/server.{py,js,pl}` — the chosen runtime, copied here at start

`server.pid` is not self-cleaning: if the server dies, the file remains and
names a dead process. `chat-server.sh status` reports this correctly as "not
running"; a stale `server.port` will simply refuse connections.

A `MSG` line is the wire format AND the storage format:

```
MSG #chan <id> <ts> <nick> :<one-line text>
```

`<id>` is per-channel, monotonic, gap-free; `<ts>` is UTC epoch. The id is
the delta handle everywhere: "give me everything after 41". Message text is a
single line — embedded newlines and carriage returns are collapsed to spaces.

## Identity (nicks)

A nick is `[A-Za-z0-9_-]`, 1–32 characters. It is **self-asserted and not
authenticated**, and **uniqueness is not enforced**: two connections may both
claim the same nick and both succeed. There is no IRC `433 ERR_NICKNAMEINUSE`
equivalent.

How a nick is chosen, in order:

- `chat-send.sh -n NICK` — explicit, per call
- `$CHAT_NICK` — the durable per-agent identity; set this once per agent
- `$USER`, then the literal `agent` — the fallback, which is **the same for
  every agent on one machine**
- on the socket path with no `NICK` verb sent, the server assigns `anon-<n>`,
  which is distinct per connection but **changes on every reconnect**

Give each agent a distinct `CHAT_NICK`. Anything that identifies a sender by
nick — including filtering out your own messages — is only as reliable as that.

## Protocol (line-based, UTF-8, `\n`-terminated)

Client → server:

    NICK <name>          set the sender name (default anon-<n>)
    REGISTER #chan       create the channel (OK if it already exists)
    JOIN #chan           push new MSG lines to this connection
    LEAVE #chan          stop pushing
    PRIVMSG #chan :text  store + broadcast one message
    FETCH #chan <since>  replay stored lines with id > since, then `OK fetch end`
    PING                 -> PONG
    QUIT                 -> OK bye, connection closes

Server → client: `OK ...`, `ERR <reason>`, `PONG`, and pushed/replayed
`MSG ...` lines. Unknown verbs answer `ERR unknown verb <verb>`.

A channel name is `#` followed by `[a-z0-9_-]`. **Lowercase only** —
`#CodeGraph` is rejected with `ERR invalid channel`.

The length limit is enforced inconsistently: every server tier caps the name at
32 characters after the `#`, but `chat-register.sh` and `chat-send.sh` do not.
A longer name can therefore be created and written locally and still be
unreachable over the socket, where it answers
`ERR usage: PRIVMSG #chan :text`. Keep names under 32 characters.

`PRIVMSG` requires a channel. Messaging a nick directly is **not supported**:
`PRIVMSG bob :hi` answers `ERR usage: PRIVMSG #chan :text`.

One connection may `JOIN` any number of channels; every joined channel's new
messages are pushed down that one connection. A sender does not receive an
echo of its own `PRIVMSG` broadcast (the stored line is returned as the direct
reply instead).

## Server runtimes

`chat-server.sh` picks the first present of `python3 → node → perl → socat
(driving runtime/bash-handler.sh)`, or honour `--runtime`. Tiers differ in one
observable way — live push:

| tier | JOIN behaviour | notes |
|---|---|---|
| `python3` | pushes live to joined connections | thread per connection |
| `node` | pushes live to joined connections | single event loop |
| `perl` | `OK join #chan (poll mode)` | one select loop; clients tail with FETCH |
| `socat` | `OK join #chan (poll mode)` | process per connection, so no shared state; needs an explicit `--port N` |

Every other verb behaves identically across tiers. Because the poll tiers
never push, a client that only waits for pushed lines will hang there — use
`chat-tail.sh`, which polls the log, or `FETCH`.

## Helpers (all pure bash)

    scripts/chat-server.sh start|stop|status [--runtime R] [--port N] [--bind ADDR] [--home D]
    scripts/chat-register.sh #chan [--home D]
    scripts/chat-send.sh #chan "text" [-n nick] [--host H] [--port N] [--home D]
    scripts/chat-read.sh #chan [--since N | --last N | --all] [--host H] [--port N] [--home D]
    scripts/chat-tail.sh #chan [since-id] [--interval N] [--host H] [--port N] [--home D]

`chat-read.sh #chan 41` is shorthand for `--since 41`; `--since` wins over
`--last`, and the default is `--all`. `chat-tail.sh` defaults to the end of the
log locally (pass `0` to replay everything) and to full replay over a socket;
`--interval` is the local poll in whole seconds (default 1 — bash 3.2 has no
fractional `read` timeout).

Without `--host`, helpers operate on `$AI_CHAT_HOME` directly under the same
advisory lock the server uses, so a dead server never blocks writing history;
live delivery is whatever tails the log (`chat-tail.sh`). With `--host`, they
speak the protocol over the socket instead.

**A locally-appended message is not broadcast.** Local mode writes the log and
returns; it does not notify the server, so a client sitting on a socket `JOIN`
will not see it until it polls. If any participant sends locally, every
participant should read by tailing the log rather than by waiting on pushes.

`--host` defaults to port **7717**, but a server started without `--port`
binds a kernel-assigned port. Read the real one from `$AI_CHAT_HOME/server.port`
(or start the server with an explicit `--port`) before using `--host`.

Exit codes: `64` bad invocation, `66` missing channel log or runtime, `69` no
server runtime at all, `70` internal or protocol failure. `chat-server.sh
status` exits `0` when running and `1` when not.

## Not IRC compatible

Deliberate scope limit, recorded so nobody plans around the wrong assumption.
A standard IRC client (irssi, WeeChat, HexChat) cannot register or join,
because the protocol lacks:

- the `USER` verb and the `001`/`002`/`003`/`004` welcome numerics, so
  registration never completes
- numeric replies generally — errors are `ERR <text>`, not `421`, `433`, `403`
- `PART` (the verb here is `LEAVE`), and `MODE`, `NAMES`, `LIST`, `TOPIC`,
  `WHO`, `WHOIS`
- message prefixes: lines are `MSG #chan <id> <ts> <nick> :text`, not
  `:nick!user@host PRIVMSG #chan :text`
- server-initiated `PING` (here the client pings and the server answers)
- mixed-case and RFC casemapped channel names
- nick-to-nick private messages, and nick uniqueness

## When not to use

Plan artifacts already carry durable handoff between known roles; chat is for
live, cross-session, or cross-machine exchange. It has no auth and no
history guarantees beyond the log files — do not route secrets through it.
