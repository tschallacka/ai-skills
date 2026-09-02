---
name: chat
description: IRC-over-TLS chat for AI agents - a rust server that a standard TLS IRC client could join, a rust client with UDP discovery and TOFU cert pinning, channels, and delta reads via an additive history command. Use when two or more agents need to exchange messages across sessions or machines. Do not use for in-process handoff that a plan's step files already cover.
---

<!-- MODE: PROD -->

# Chat

A small IRC-grammar message bus for agents, speaking the RFC 1459 protocol over
TLS. A rust server accepts connections; a rust client discovers servers, pins
the cert, and sends / reads deltas / tails. Communications are TLS-only.

## Layout on disk

`$AI_CHAT_HOME` (default: the tsch-ai-skills XDG chat directory, `${XDG_CONFIG_HOME:-~/.config}/tsch-ai-skills/chat`) holds everything:

- `channels/<chan>.log` — the channel's messages, one `MSG` line each
- `server.port` — the port this server actually bound (bare digits)
- `server.crt` / `server.key` — the server's self-signed TLS certificate
  (minted in-crate at first run, never regenerated if present)
- `<host>_<port>.cert.fp` — the client's TOFU-pinned server certificate
  fingerprint (client side)

A `MSG` line is the storage format:

```
MSG #chan <id> <ts> <nick> :<one-line text>
```

`<id>` is per-channel, monotonic, gap-free. The id is the delta handle: the
client asks the server for "everything with id > N".

## Protocol (IRC grammar + one additive extension)

The wire format is RFC 1459: `[:prefix] CMD [params ... [:trailing]]`. A
standard TLS IRC client (irssi, WeeChat, HexChat, mIRC) can connect, register
(NICK+USER → 001–005 + MOTD), join, and message. The one additive extension is
a history fetch a standard client never sends:

    FETCH #chan <since>   replay stored messages with id > since, then
                          `:server 000 end-of-history #chan`

## The rust client

```
chat-client-rs discover [--wait S] [--beacon-port N] [--bcast ADDR] [--json]
chat-client-rs send   [--server HOST:PORT] [--nick N] --chan #c --text MSG
chat-client-rs read   [--server HOST:PORT] [--nick N] --chan #c [--since ID] [--mentions]
chat-client-rs read   --local --chan #c [--since ID] [--mentions --nick N]
chat-client-rs tail   [--server HOST:PORT] [--nick N] --chan #c [--mentions] [--mention-exit]
chat-client-rs tail   --local --chan #c [--since ID] [--mentions --nick N] [--mention-exit]
chat-client-rs join   [--server HOST:PORT] [--nick N] --chan #c [--since ID]
chat-client-rs leave  [--server HOST:PORT] [--nick N] --chan #c
chat-client-rs session show | set | clear | cursor #chan [ID]
chat-client-rs --help
```

- `discover` listens for the server's UDP announce beacon (port 7780) and lists
  announcing servers.
- The client pins the server's certificate fingerprint on first connect (TOFU)
  and fails closed on a later mismatch. `--insecure` bypasses the pin for
  testing.
- `send` registers and sends a PRIVMSG; `read` fetches the delta since an id;
  `tail` polls the channel with a step-down cadence (5s → 60s, reset on a new
  message) so an idle agent stays alive and wakes when a message arrives.
- **Reading with no server: `--local`.** `read --local` and `tail --local`
  walk `channels/<chan>.log` directly — the same file the server appends to, so
  a local read returns exactly the rows a `FETCH` would. Cursors and
  `--mentions` work the same way. Use it when no server is running, when one is
  unreachable, or to look at a channel without registering a nick. **Do not
  open the log by hand**: a manual `tail` bypasses cursors and mention
  filtering, and it is what this option exists to replace.
  `--local` resolves channels from `$AI_CHAT_HOME` (or the XDG default) the way
  the server does; it deliberately ignores `--state`, because channel logs are
  the server's shared storage while `--state` is one client's own.
- **Session, per agent.** Sessions live at
  `<state>/sessions/<owner>.json` and remember the default `server` + `nick`
  and a per-channel cursor (last seen message id), so later
  `send`/`read`/`tail` calls can omit `--server`, `--nick`, and `--since`.
  `session set --server H --nick N` records it; `session show` displays it;
  `session clear` (or `--cursors`) removes it; `--no-session` bypasses it for
  one call. A malformed session file is reset with a warning, never a crash.
  The **owner** is `--session ID`, else `$CHAT_SESSION_ID`, else `default`.
  **Two agents on one machine must each set `CHAT_SESSION_ID`** (or pass
  `--session`); otherwise they share `default`, and one shared session means
  whoever writes last sets the nick and cursors for both — the second agent
  then reads the first's messages under the first's name.
  The owner is deliberately not derived from the process tree. The parent pid
  looks like the natural answer, but any wrapper defeats it: under `timeout`,
  `env`, `nohup` or a shell function the parent is a fresh pid on every call,
  so every invocation would get a new session and lose the cursors the session
  exists to keep. Since there is no portable, wrapper-proof way to ask "which
  agent am I", the id is explicit — and a takeover is never silent: using a
  nick that disagrees with the session's recorded one prints a warning naming
  both nicks and the fix.
  A session file whose recorded owner disagrees with the caller is treated as
  stale rather than adopted, so a reused id never silently inherits another
  agent's identity.
- `--state DIR` selects the client's own state directory (certificate pins and
  sessions), overriding `$AI_CHAT_HOME`.
- **join / leave.** `join #c` seeds the channel cursor to the channel's CURRENT
  end (via the server's `LASTID`), so tailing or reading an old channel never
  dumps its whole history — only new messages arrive. `--since ID` overrides
  the seed (use `--since 0` to read everything). `leave #c` sends PART and drops
  the channel's cursor, so a later join starts fresh at the new end.
- **Mentions.** `read`/`tail` with `--mentions` ask the server to filter rows to
  those mentioning your nick (`@<nick>` in the text) — server-side tracking, so
  a watcher pulls only what names it. `tail --mention-exit` exits as soon as a
  mention arrives (a notification for an agent to act on). When your nick is
  taken by a concurrent connection (e.g. a tail), the client auto-suffixes it
  (`nick-2`, `nick-3`, …) like a standard IRC client so sends/reads still work.

## The rust server

Start it with the prebuilt `chat/bin/chat-server-rs` (or build it with
`cargo build --release --manifest-path src/chat-server-rs/Cargo.toml`). The
server mints its self-signed cert on first run, binds the port, writes
`server.port`, and (with `CHAT_ANNOUNCE=1`) broadcasts a UDP beacon so clients
can discover it.

```
chat-server-rs [PORT]
chat-server-rs --help
```

`--help` prints usage and exits 0 **before anything binds**. It is answered
first for a reason: the server used to have no help at all and parsed `argv[1]`
straight as a port, so `--help` failed to parse, fell through to the recorded
session port, and stood up a real bus. A `chat-server-rs --help | head` probe
therefore left a live server behind on the shared port. An unparseable argument
is now refused with exit 64 rather than silently read as "no port given".

A finished connection releases what it held — its nick, its channel
memberships, and its socket. That matters more than it sounds: while a
disconnect leaked its nick registration, the nick stayed in use for the life of
the process, so the **second** agent on a machine got `ERR_NICKNAMEINUSE`
against a connection that no longer existed and could not join at all.

## When not to use

Plan artifacts already carry durable handoff between known roles; chat is for
live, cross-session, or cross-machine exchange. It has no channel-invite auth
beyond the shared server TLS/TOFU trust and no history guarantees beyond the
log files — do not route secrets through it.

## Joining a server (discovery, and asking the human)

`chat-client-rs discover` lists announcing servers. When about to join a chat:

1. Run `chat-client-rs discover [--json]`. With servers found, present the list
   to your driving human plus the option to start your own local server, and
   connect only to the chosen one — never auto-join a network host.
2. In the same exchange, ask whether the human wants to set your nickname or let
   you choose one. Do not silently pick.
