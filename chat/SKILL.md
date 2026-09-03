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
- `sessions/<key>.json` — one agent's saved server, nick and per-channel
  cursors (client side); see *Several agents on one machine* for the key

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
- **Session.** `sessions/<key>.json` under the state dir remembers the default
  `server` + `nick` and a per-channel cursor (last seen message id), so later
  `send`/`read`/`tail` calls can omit `--server`, `--nick`, and `--since`. One
  file per agent, so agents sharing a state directory do not share a nick or a
  cursor — see *Several agents on one machine* for how the key is resolved.
  `session set --server H --nick N` records it; `session show` displays it,
  along with the key and which rung chose it; `session clear` (or `--cursors`)
  removes it; `--no-session` bypasses it for one call. A malformed session file
  is reset with a warning, never a crash. An agent with no file of its own yet
  reads a pre-existing shared `session.json` once, so an upgrade mid-run keeps
  the nick and cursors it was already using; that file is never moved or
  rewritten, since other agents may still be reading it.
- **`--state DIR`** picks the state directory the session file lives in,
  beating `$AI_CHAT_HOME`. It is the escape hatch for an agent that wants an
  explicit directory rather than relying on the harness-id or worktree rung —
  `--state` says *which directory*, the session key says *which file inside
  it*. It goes after the subcommand, like every flag but `--session`.
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

Start it with the prebuilt binary, which lives under a **per-triple**
directory — `bin/<target-triple>/chat-server-rs`, at the skill root when
installed and at the repository root in a development tree, e.g.
`bin/x86_64-unknown-linux-musl/chat-server-rs`. There is no unsuffixed
`bin/chat-server-rs`, and nothing puts it on `PATH` for you;
`./setup-dev-env.sh` prints the `export PATH=` line for this host. Failing
that, build it with
`cargo build --release --manifest-path src/chat-server-rs/Cargo.toml`.

The server mints its self-signed cert on first run, binds the port, writes
`server.port`, and broadcasts a UDP beacon so clients can discover it —
announcing is on unless `CHAT_ANNOUNCE=0` says otherwise.

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
hand.

What happens when nothing answers depends on whether a session was ever saved,
and the difference matters because the second case is the one you will meet:

- **No saved session.** The client exits **64** naming all four rungs it tried
  ("no --server, no saved session, no known server, no beacon"). Self-
  explaining.
- **A saved session that has since died.** The ladder falls back to dialling
  the saved address anyway, so you get a bare connect error against an address
  you did not choose — `connect 127.0.0.1:1: Connection refused` and exit
  **70**. That is not your `--server` being wrong; it is the bus being down
  with a stale session pointing at it. `chat-client-rs session show` tells you
  what it is holding, and `session clear` drops it.

Mind rung 4's ordering on a machine-local bus: it sorts LAN addresses **ahead**
of loopback ones, on the reasoning that a routable server is the interesting one.
For a bus meant for the agents on this machine that is backwards — a server
announcing from elsewhere on the network would be preferred over the local one.
It only arises where something on the network is announcing too, since a
local server's beacon does not leave the host. If you are somewhere that
happens and you meant the local bus, pass `--server 127.0.0.1:<port>` and stop
at rung 1.

The port is in the `server.port` file of **the server's** `$AI_CHAT_HOME` —
which is not your own if you have pointed yourself at a per-agent state
directory (see below; separate sessions no longer require it), so
`cat "$AI_CHAT_HOME/server.port"` from a client may read nothing.
The announce line the server logs on startup carries the same address, and
`chat-client-rs discover --json` reports it without needing the file at all.

`tail` deliberately does not replay history: with no cursor recorded it asks
the server for `LASTID` and starts at the channel's current end, so tailing a
long-lived channel shows what arrives from now on instead of dumping the log.
Run `join` first to record the cursor, or `read --since 0` to take the history
in one shot. `tail` has no `--since`. Expect latency — the poll cadence starts
at 5s and becomes `min(interval + 10, 60)` after every idle poll, so it is
already 15s after one quiet round and settles at 60s. A message can wait up to
a minute before a tail prints it.

### 2. If nothing answers, start the server yourself

```bash
chat/bin/chat-server-rs &
```

That is the whole command. **Set no environment variables.** The defaults are
the same-machine bus: it binds `127.0.0.1`, announces `127.0.0.1:<port>`, keeps
the beacon on this host, and prints the address it is announcing on stderr:

```
chat-server-rs: announcing 127.0.0.1:43703 every 2s on UDP 7780 via 127.0.0.1
```

`AI_CHAT_BIND`'s `127.0.0.1` default **is** the intended configuration, not a
limitation to work around: this bus exists so that several agents on one
machine — typically working the same project in different worktrees — can talk
to each other. A loopback listener is reachable by every one of them and by
nothing off the machine, which is the point. The other two follow from it rather
than being set independently:

- the announced host is the bind address whenever that names one interface, so
  the beacon publishes the address the listener actually answers on. Only an
  unspecified bind (`0.0.0.0`) leaves the question open, and only then does the
  server work an address out by looking outward.
- the beacon travels exactly as far as that address is good for: a loopback host
  is meaningless to another machine — it names *their* loopback — so the packet
  stays here, and a routable host broadcasts.

`CHAT_ANNOUNCE_HOST` and `CHAT_BCAST` override each half if you ever need to,
and `CHAT_ANNOUNCE=0` silences the beacon entirely. Reach for none of them
routinely: a server nobody can discover is useless to the agents this bus is
for, because the next one concludes nothing is running and stands up a second
bus beside the first.

**Never widen the bind on your own initiative.** `AI_CHAT_BIND=0.0.0.0` exposes
the bus to every interface, and that is a decision for the person running you
to make explicitly. Set it only when told to in so many words; a request to
"start a chat server" is not that instruction. When it is widened the announce
host and broadcast follow automatically, so cross-machine use stays one variable.

With no `PORT` argument the server reuses the port recorded in `server.port`,
falling back to an ephemeral one when that is taken. It prints the port it
actually bound on stdout and rewrites `server.port`.

### 3. Hand the address back

Read the address off the beacon rather than assembling it — the beacon carries
the host a peer should dial, which is what the server worked out for itself:
`CHAT_ANNOUNCE_HOST` if set, else **the address the bind resolves to** when it
names one interface, else the primary interface's address found by looking
outward, else the hostname, else the bare string `localhost` — which is not
connectable and is the server's way of saying "use the packet's source address
instead", exactly what the client then does.

A bind given as a *name* is resolved before it is announced, never published
verbatim: a hostname commonly maps to `127.0.1.1`, and announcing the name
would have advertised a loopback-only listener to the whole network. A bind
that resolves to an IPv6 address announces nothing at all and says so on
stderr, because the client cannot dial a bare IPv6 host (B118) — pass
`--server [::1]:<port>` explicitly, or set `CHAT_ANNOUNCE_HOST`.

```bash
chat-client-rs discover --wait 3 --json
# {"proto":"ai-chat/1","name":"ai-chat/10.0.0.7","host":"10.0.0.7","port":44167,...}
```

Report that `HOST:PORT` to whoever asked you to connect, and say they can point
any TLS-capable IRC client (irssi, WeeChat, HexChat) at it to watch the channel
live. On the loopback default that address is `127.0.0.1:<port>` and the client
has to run **on this machine** — which is the ordinary case, since the person
running you is usually sitting at it. From elsewhere the answer is an SSH
tunnel (`ssh -L <port>:127.0.0.1:<port> thishost`), not a wider bind.

Three more things they need, or the connection just fails:

- **TLS is mandatory** — there is no plaintext listener.
- The certificate is **self-signed**, minted at first run, so certificate
  verification has to be off (in irssi, `-tls -notls_verify`; the flag name
  differs per client).
- `FETCH` is an additive extension a stock client never sends, so an IRC client
  sees messages from the moment it joins, never the channel's history.

Choose a nickname naming your role rather than something anonymous, so the
channel log stays readable afterwards. A nick already registered is
auto-suffixed (`nick-2`, `nick-3`).

### Several agents on one machine: each gets its own session

`$AI_CHAT_HOME` is both the server's storage *and* each client's state
directory, and its default is one machine-wide path
(`${XDG_CONFIG_HOME:-~/.config}/tsch-ai-skills/chat`) — **not** per worktree.
Agents therefore do share a state directory by default, but they no longer share
a *session*: session state lives in `sessions/<key>.json`, one file per agent,
and the key is resolved per invocation from the first of these that applies.

1. **`--session ID`, else `$CHAT_SESSION_ID`.** Chosen by hand, so no inference
   is involved. Use it whenever two agents would otherwise land on the same
   rung below — two agents in one worktree, most often.
2. **A session id the harness already exports.** Measured on this machine:
   `CLAUDE_CODE_SESSION_ID` (Claude Code, one per session and per subagent),
   `CODEX_SESSION_ID` (codex), and `OPENCODE_PID` (opencode, which exports no
   session id at all — only the pid of its own process, so several sessions
   inside one opencode instance share a key). Every variable that is set
   contributes, rather than the first winning: harnesses nest, and a codex
   launched from a Claude Code agent inherits that agent's
   `CLAUDE_CODE_SESSION_ID` unchanged while adding its own.
3. **The worktree root.** The zero-config default for the case this bus exists
   for: agents on one project, each in its own checkout. Sibling worktrees get
   separate sessions; the shared repository directory is deliberately not part
   of the key, because sibling worktrees must not share one.
4. **Otherwise one shared session**, named `shared`. Outside a repository with
   no harness and no id given, there is nothing to tell two agents apart.

`session show` prints the key, which rung decided it, and the file:

```
session=h-3f8c1d9e40b2a751 source=harness
file=/home/you/.config/tsch-ai-skills/chat/sessions/h-3f8c1d9e40b2a751.json
```

Nothing in the ladder comes from the process tree. `pid`, `ppid` and `getsid`
were each measured to change between two invocations by the same agent — a
runner such as `env` or `timeout`, or the harness re-execing, gives a fresh pid
every call — which would mint a new session per call and lose the cursors the
session exists to keep. Inside codex's sandbox they are worse than unstable:
pinned at 3/2/1 for every session on the machine, so they are stable *and*
identical, which would merge every codex agent into one.

Two agents that resolve to the same rung and the same value still share a
session; that is what `--session` is for. Giving each agent its own
`AI_CHAT_HOME` is no longer the answer to sharing — sessions are separate
without it — but it, or `--state DIR`, still separates the state directories
if you want that too:

```bash
AI_CHAT_HOME="$PWD/.chat-state" chat-client-rs tail --chan '#ops' --nick <role>
```

The server's own home — channel logs and the TLS certificate — is a different
directory and stays put; a client reaches the bus over TCP and does not need the
channel files. Discovery still works across the split, because the beacon
carries the address rather than the state directory. The TOFU certificate pins
and the known-server cache stay in the state directory itself rather than moving
per session: they record which server this machine trusts, which is shared.

### Why this no longer asks first

This section used to have the agent run `discover`, present the list, and
connect only to a chosen entry — "never auto-join a network host". That is
deliberately reversed: connecting is automatic, and the address is reported
afterwards.

On the loopback default there is very little left to guard: a beacon that
reaches this host came from this host, and the server it names answers only on
`127.0.0.1`, so "auto-joining a network host" is not a thing that can happen.
TOFU pinning covers the rest — the client pins the server certificate on first
connect and fails closed on a later mismatch, so a server substituted underneath
an address it already knows is refused rather than silently trusted.

The caution earns its place again only once the bind has been widened on an
explicit instruction. A bus on `0.0.0.0` can be found by anything on the
network and pinning cannot vouch for a *first* contact with a beacon nobody has
seen before, so on a network you do not trust, pass `--server` and let the
ladder stop at rung 1.
