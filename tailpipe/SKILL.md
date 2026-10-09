---
name: tailpipe
description: Use when an agent needs to pipe a running command's output into a named, server-held stream that another agent, session, or process can list/read/search/tail/save independently -- a less/tail for agents, with message ids like the chat skill. An idle stream (no read or write for 15 minutes) is evicted automatically, snapshotted to a gzip file first so the data is not lost. Not for durable, cross-session message history (use the chat skill) or for editing a file in place (use ai-text-editor).
---
<!-- MODE: PROD -->

# tailpipe

A server that accepts piped stdin from any running command, splits it into
lines, and holds each one under a monotonic per-stream id -- the same
message-id shape the chat skill uses for its own channels. A second binary
(the reader) and an MCP adapter (the consumer) let an agent address a
specific stream and query it by exact line id, an inclusive id range, or a
search, without needing to have been the process that started the stream. A
read-only board mod lets a human watch a stream's live output from inside
the host. Four parts: server, reader, consumer (MCP), mod.

## The server

```
tailpipe-server-rs <endpoint-path> [--idle-timeout-ms N] [--snapshot-dir PATH]
```

Binds its endpoint (a Unix socket on Unix, a nonce-authenticated loopback
TCP discovery file on Windows -- the same shape `planning-server`'s own
transport uses) and serves every connection on its own thread. A stream is
created on its first ingested line and lives in memory until it is
explicitly saved or evicted.

**Idle eviction.** A stream with no read or write for `--idle-timeout-ms`
(default 900000, 15 minutes) is snapshotted -- gzip-compressed, one line per
line, to `--snapshot-dir/<stream>-<unix-timestamp>.gz` -- and removed from
the live registry. `--snapshot-dir` defaults to
`${TAILPIPE_HOME:-${XDG_CONFIG_HOME:-$HOME/.config}/tsch-ai-skills/tailpipe}/snapshots`.
The data is never silently lost, only no longer queryable live: `gunzip` or
`zcat` reads a snapshot with ordinary tooling, the same as any other `.gz`
file.

## The reader (`tailpipe-client-rs`)

```
tailpipe-client-rs ingest --endpoint PATH --stream NAME     # reads stdin
tailpipe-client-rs list   --endpoint PATH
tailpipe-client-rs read   --endpoint PATH --stream NAME --from N --to N
tailpipe-client-rs search --endpoint PATH --stream NAME --mode exact|regex --query Q
tailpipe-client-rs tail   --endpoint PATH --stream NAME [--since N]
tailpipe-client-rs save   --endpoint PATH --stream NAME [--out PATH]
```

`ingest` pipes stdin into the named stream, printing each assigned id to
stderr as it arrives:

```bash
some-long-running-command 2>&1 | tailpipe-client-rs ingest --endpoint "$ENDPOINT" --stream build-log
```

`read`/`search` are each one request-response round trip. `tail` is a
genuine long-poll: the connection stays open and the server writes a new
line to it as soon as one is ingested, the same live-delivery shape the
chat skill's own `tail` has -- it does not reconnect or poll. With no
`--since`, it starts from the stream's current end (like `chat-client-rs
tail`'s own default); an explicit `--since ID` replays from an earlier id.
`save` triggers an explicit snapshot without evicting the stream, printing
the written path (or copying it to `--out PATH` first, when given).

## The consumer (`tailpipe-mcp`)

The MCP adapter for an agent that prefers typed tool calls over shelling
out. It resolves its own endpoint (the same `$TAILPIPE_HOME`-rooted default
the server and reader use, or `$TAILPIPE_ENDPOINT` to override) -- no tool
argument carries a socket path, matching the chat skill's own adapter
philosophy that transport details are the adapter's business, not a
model's to choose.

| tool | takes | answers |
|---|---|---|
| `list_streams` | -- | the server's currently active stream names |
| `read` | `stream`, `from`, `to` | the lines in that inclusive id range |
| `search` | `stream`, `mode` (exact or regex), `query` | the matching lines |
| `wait` | `stream`, `since`, `timeout_seconds` (default 30) | the next line past `since`, or `timed_out: true` |
| `save` | `stream` | the written gzip snapshot path |

`wait` genuinely blocks (up to `timeout_seconds`); the adapter answers a
`tools/call` on its own thread, mirroring chat-mcp's own fix for the same
problem (a long-lived wait must never hold every other request behind it).
Ingest has no MCP tool: piping stdin into a stream is inherently a shell
operation, not something a typed tool call can do.

## The mod (`tailpipe-board`)

A read-only pane, installed alongside the skill, showing a chosen stream's
recent lines with a streams-picker button to switch which one is shown. It
polls the reader's `read` subcommand on an interval rather than holding
`tail`'s own long-poll connection open -- the same choice the chat skill's
own board mod makes against chat's live tail. Unlike that board, there is
no on-disk fallback: a tailpipe stream lives only in server memory while
active, so the pane always needs a reachable server.

## Install mode

`tailpipe-client-rs` always ships: ingest is inherently a shell-piping
operation no MCP tool call can perform. `tailpipe-mcp` installs
additionally only in `mcp` mode, exactly mirroring the chat skill's own
dual-mode split:

```
installer install --integration tailpipe=skill --skill tailpipe --target DIR --yes   # reader only
installer install --integration tailpipe=mcp   --skill tailpipe --target DIR --yes   # reader + consumer
```

With neither flag, a fresh install defaults to `skill`.

## When not to use

Not for durable, cross-session message history between agents -- the chat
skill's channels are built for that, with no idle-eviction model at all. Not
for editing a file in place -- ai-text-editor owns that. A tailpipe stream
is a firehose: append-only, no revision concept, no undo, and gone from
memory 15 minutes after the last read or write unless explicitly saved.
