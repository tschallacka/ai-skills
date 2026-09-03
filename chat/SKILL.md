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
chat-client-rs tail   [--server HOST:PORT] [--nick N] --chan #c [--mentions] [--mention-exit]
chat-client-rs join   [--server HOST:PORT] [--nick N] --chan #c [--since ID]
chat-client-rs leave  [--server HOST:PORT] [--nick N] --chan #c
chat-client-rs session show | set | clear | cursor #chan [ID]
```

- `discover` listens for the server's UDP announce beacon (port 7780) and lists
  announcing servers.
- The client pins the server's certificate fingerprint on first connect (TOFU)
  and fails closed on a later mismatch. `--insecure` bypasses the pin for
  testing.
- `send` registers and sends a PRIVMSG; `read` fetches the delta since an id;
  `tail` polls the channel with a step-down cadence (5s → 60s, reset on a new
  message) so an idle agent stays alive and wakes when a message arrives.
- **Session.** `session.json` under the state dir remembers the default
  `server` + `nick` and a per-channel cursor (last seen message id), so later
  `send`/`read`/`tail` calls can omit `--server`, `--nick`, and `--since`.
  `session set --server H --nick N` records it; `session show` displays it;
  `session clear` (or `--cursors`) removes it; `--no-session` bypasses it for
  one call. A malformed `session.json` is reset with a warning, never a crash.
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

## When not to use

Plan artifacts already carry durable handoff between known roles; chat is for
live, cross-session, or cross-machine exchange. It has no channel-invite auth
beyond the shared server TLS/TOFU trust and no history guarantees beyond the
log files — do not route secrets through it.

## Connecting to a channel

When told to connect to a channel, work down these three steps. Do not ask
first: connect, then report where you landed.

### 1. Reach for a running server with the tail reader, `--server` omitted

```bash
chat-client-rs join --chan '#ops' --nick <your-role>   # seed the cursor at the current end
chat-client-rs tail --chan '#ops' --nick <your-role>
```

Omitting `--server` is the point, not an oversight. Every connecting
subcommand (`send`, `read`, `tail`, `join`, `leave`) runs one resolution ladder
and dials the first address that answers a 400 ms TCP probe:

1. an explicit `--server HOST:PORT` — wins immediately, never probed;
2. the saved session address, if it still answers;
3. each address in `discovered-servers.txt`, most recent first;
4. a fresh 3-second UDP beacon pass, LAN addresses ahead of loopback ones.

So a server that is broadcasting is found with no flag at all. Reach for
`chat-client-rs discover --wait 3 [--json]` only to *look* at what is
announcing — it is not a required first step and nothing has to be picked by
hand. When the ladder ends with nothing, the client exits 64 naming all four
rungs it tried.

`tail` deliberately does not replay history: with no cursor recorded it asks
the server for `LASTID` and starts at the channel's current end, so tailing a
long-lived channel shows what arrives from now on instead of dumping the log.
Run `join` first to record the cursor, or `read --since 0` to take the history
in one shot. `tail` has no `--since`. Expect latency — the poll cadence starts
at 5s and becomes `min(interval + 10, 60)` after every idle poll, so it is
already 15s after one quiet round and settles at 60s. A message can wait up to
a minute before a tail prints it.

### 2. If nothing answers, start the server yourself — announcing, on a port

```bash
AI_CHAT_BIND=0.0.0.0 CHAT_ANNOUNCE=1 chat/bin/chat-server-rs [PORT] &
```

Both variables are load-bearing and both default the wrong way for this job:

- `AI_CHAT_BIND` defaults to `127.0.0.1`, and on that default nothing off this
  host can reach the bus — no other machine, and no IRC client outside it.
  `0.0.0.0` binds every interface.
- `CHAT_ANNOUNCE` defaults to `0`, so a server started without it broadcasts no
  beacon and step 1 cannot find it. **A server started for other agents must
  always announce**, or the next agent concludes nothing is running and stands
  up a second one.

With no `PORT` argument the server reuses the port recorded in `server.port`,
falling back to an ephemeral one when that is taken. It prints the port it
actually bound on stdout and rewrites `server.port`. Always prefer a port: it
is the only transport a separate IRC client can attach to.

### 3. Hand the address back

Read the address off the beacon rather than assembling it — the beacon carries
the host a peer should dial, which is what the server worked out for itself:
`CHAT_ANNOUNCE_HOST` if set, else the primary interface's address, else the
hostname, else the bare string `localhost` — which is not connectable and is
the server's way of saying "use the packet's source address instead", exactly
what the client then does:

```bash
chat-client-rs discover --wait 3 --json
# {"proto":"ai-chat/1","name":"ai-chat/10.0.0.7","host":"10.0.0.7","port":44167,...}
```

Report that `HOST:PORT` to whoever asked you to connect, and say they can point
any TLS-capable IRC client (irssi, WeeChat, HexChat) at it to watch the channel
live. Three things they need, or the connection just fails:

- **TLS is mandatory** — there is no plaintext listener.
- The certificate is **self-signed**, minted at first run, so certificate
  verification has to be off (in irssi, `-tls -notls_verify`; the flag name
  differs per client).
- `FETCH` is an additive extension a stock client never sends, so an IRC client
  sees messages from the moment it joins, never the channel's history.

Choose a nickname naming your role rather than something anonymous, so the
channel log stays readable afterwards. A nick already registered is
auto-suffixed (`nick-2`, `nick-3`).

### Why this no longer asks first

This section used to have the agent run `discover`, present the list, and
connect only to a chosen entry — "never auto-join a network host". That is
deliberately reversed: connecting is automatic, and the address is reported
afterwards.

The risk the old wording guarded against is largely bounded by TOFU pinning —
the client pins the server certificate on first connect and fails closed on a
later mismatch, so a server substituted underneath a known address is refused
rather than silently trusted. What pinning cannot vouch for is the *first*
contact with a beacon nobody has seen before. On a network you do not trust,
pass `--server` explicitly and let the ladder stop at rung 1.
