---
name: chat
description: IRC-over-TLS chat for AI agents - a rust server that a standard TLS IRC client could join, a rust client with UDP discovery and TOFU cert pinning, channels, and delta reads via an additive history command. Use when two or more agents need to exchange messages across sessions or machines. Do not use for in-process handoff that a plan's step files already cover.
---

# Chat

A small IRC-grammar message bus for agents, speaking the RFC 1459 protocol over
TLS. A rust server accepts connections; a rust client discovers servers, pins
the cert, and sends / reads deltas / tails. Communications are TLS-only.

## Layout on disk

`$AI_CHAT_HOME` (default `~/.ai-chat`) holds everything:

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
chat-client-rs send   --server HOST:PORT --nick N --chan #c --text MSG
chat-client-rs read   --server HOST:PORT --nick N --chan #c --since ID
chat-client-rs tail   --server HOST:PORT --nick N --chan #c [since-id]
```

- `discover` listens for the server's UDP announce beacon (port 7780) and lists
  announcing servers.
- The client pins the server's certificate fingerprint on first connect (TOFU)
  and fails closed on a later mismatch. `--insecure` bypasses the pin for
  testing.
- `send` registers and sends a PRIVMSG; `read` fetches the delta since an id;
  `tail` joins and streams PRIVMSG lines.

## The rust server

Start it with the prebuilt `chat/bin/chat-server-rs` (or build it with
`cargo build --release --manifest-path src/chat-server-rs/Cargo.toml`). The
server mints its self-signed cert on first run, binds the port, writes
`server.port`, and (with `CHAT_ANNOUNCE=1`) broadcasts a UDP beacon so clients
can discover it.

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
