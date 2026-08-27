---
name: chat
description: IRC-basis chat for AI agents - one detached server per store that owns the socket, registers itself under $XDG_RUNTIME_DIR so chatters discover it instead of guessing a port, starts on first use and keeps running until stopped, on a transport you choose once (unix socket, loopback port, or every interface); channels agents register, join, and leave; and clients to send, read a delta since an id, or tail a constant stream, as pure-bash helpers and as subcommands of the same binary for hosts without bash. Use when two or more agents need to exchange messages across sessions or machines. Do not use for in-process handoff that a plan's step files already cover.
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
- `config` — the recorded transport, `key=value`, read by the server AND the
  clients. See "The transport is your decision" below; `chat config` prints it.
- `chat.sock` — the listening socket, when the transport is a unix socket
- `server.pid`, `server.port`, `server.bind`, `server.socket`, `server.log` —
  the running server. A TCP instance writes `server.port`/`server.bind`; a
  socket instance writes `server.socket` and no port file
- `instances/<endpoint>/` — a `--no-register` instance's adverts, deliberately
  nowhere a discovering client looks
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

## The `chat` binary

A compiled binary carrying **both sides**: `chat serve` is a server, and `chat
send|read|tail|register` are clients that mirror the helper flags. It exists
because the helpers are bash, which a Windows agent outside Git Bash does not
have — a server-only binary would have handed such a host a bus it could not
talk to.

    chat serve    [--home D] [--bind ADDR --port N]
    chat status   [--home D] [--bind ADDR --port N]
    chat register #chan [--host H] [--port N] [--home D]
    chat send     #chan "text" [-n NICK] [--host H] [--port N] [--home D]
    chat read     #chan [--since N | --last N | --all] [--host H] [--port N] [--home D]
    chat tail     #chan [--since N] [--host H] [--port N] [--home D]

Clients behave exactly as the helpers do, including the split that matters:
**without `--host` a client never touches a socket** — `send` appends under the
channel lock and `read`/`tail` read the log — so they work with no server at
all. It stores the same `MSG` lines under the same locks, so binary and bash
clients interoperate against either implementation.

### Starting a server, and starting a second one

`chat serve` with no endpoint flags is **the** bus for its home, on whichever
transport is recorded (see "The transport is your decision" below). A second one
**declines** with exit `69` and names the live endpoint. It decides that from an
`flock` the kernel releases when the process dies, plus the bind itself — never
from `server.pid`, because a pid file outlives its process and trusting one would
make a *fresh* start refuse while reporting the bus is up.

To run a **debug** server without disturbing one in use, serve without claiming
the home:

    chat serve --home /tmp/dbg --bind 127.0.0.1 --port 17717 --no-register
    chat serve --home /tmp/dbg --socket /tmp/dbg/debug.sock --no-register

**Registration, not the endpoint, is what makes a server "the" bus.** That
distinction used to ride on whether an endpoint was given explicitly, which is
why `chat-server.sh --port N` and `chat serve --port N` meant different things
and could not be reconciled. Now an endpoint is just an endpoint — advertised in
the registry entry, discovered rather than assumed — and `--no-register` is the
axis that matters.

A `--no-register` instance writes no registry entry, no default run files and no
config, so discovery cannot land on it and bringing one up on `0.0.0.0` cannot
re-point the clients of a bus in use. Reaching it takes an explicit flag on the
client, every time. `--bind` and `--port` still come as a pair — either alone is
exit `64` — and `--socket` names an endpoint on its own.

`server.pid` is still written, as a diagnostic. Nothing reads it to decide
whether a server is running — `chat status` takes the lock to find out. A TCP
instance advertises `server.port` and `server.bind`; a socket instance
advertises `server.socket` and no port file at all, so nothing can dial a bus
that has no port.

### Servers are registered, not assumed

**A server owns the socket; nothing else creates one.** It registers itself, and
clients discover it there rather than agreeing a port in advance.

    $XDG_RUNTIME_DIR/chat/<timestamp>-<pid>.server     when XDG_RUNTIME_DIR is set
    <temp>/chat-<uid>/<timestamp>-<pid>.server         otherwise

`chat servers` lists what is registered and whether it answers. An entry is
`key=value` like the config, and records `home=`, `pid=`, `started=` and the
transport.

**An entry is not proof of life.** This is the pid-file lesson in a new place: a
crashed server leaves its entry behind, and a client that trusted the file would
dial a socket nothing is listening on. So liveness is decided by *connecting*,
and a failed dial **evicts** the entry — the caller then behaves as though the
crashed server had never registered, with no error to interpret and no need to
know the file exists. `chat servers` reports a stale entry rather than deleting
it, because a listing that removes what it describes is hard to look at twice.

**The directory is per-uid and `0700`, and that is a security property.** `/tmp`
is world-writable, so a fixed shared path would let another user plant an entry
naming their own socket, and every client on the machine would talk to them —
the protocol has no authentication, so nothing would notice. A directory whose
owner is not you, or which is a symlink, is **refused**. The timestamp gives
uniqueness; it gives no ownership.

**The reboot claim, precisely.** On Linux `/tmp` is usually tmpfs and does vanish
at reboot. On **macOS** `/private/tmp` is on disk, survives reboots, and is only
pruned after three days. `XDG_RUNTIME_DIR` is preferred where present because it
is per-user, tmpfs and cleared on logout. Nothing depends on any of this: a stale
entry is survivable by design, which is the real defence.

### A server keeps running until it is stopped

`chat serve` **detaches** — new session via `setsid`, standard streams off the
terminal, `SIGHUP` ignored — so closing the shell that started it does not take
it down. It prints the endpoint once the server has registered, then exits.

    chat serve            # start it, or report the one already running
    chat stop             # end it, and evict its entry
    chat servers          # what is registered, and whether it answers
    chat serve --foreground   # run it in this process, for a supervisor

`chat stop` evicts the entry itself rather than leaving it for the next client's
liveness check. Discovering a corpse works, but it is not the same as tidying up
after yourself, and the window in between is one where `chat servers` lies.

**A chatter that finds no server starts one.** The first participant stands the
daemon up and adopts it; everyone after attaches to what is registered. It says
so on stderr, naming what it started and where — a client that quietly spawns a
daemon is baffling when something later goes wrong.

Auto-start **never asks the transport question**. The prompt only makes sense on
a first interactive `chat serve`; a chatter may be a client in a pipeline, and
answering it there would record a transport decision as a side effect of sending
a message. So auto-start always takes the no-terminal path: safe default, said
out loud, recorded nowhere.

**Two chatters starting at once produce one server.** Both spawn a candidate;
one takes the home lock and registers, the other exits. Both are waiting for
"a live entry exists" rather than "my child won", so the loser attaches to the
winner without needing to know it lost — and reports attaching, not starting.

### The transport is your decision

Asked **once**, on the first `chat serve` that has a terminal, and recorded in
`$AI_CHAT_HOME/config`. Three outcomes, because there are three genuinely
different exposures:

| choice | who can reach the bus |
|---|---|
| unix socket | this machine only, and only users the socket's mode permits (it is created `0600`) |
| tcp on `127.0.0.1` | any process on this machine, including other users' |
| tcp on `0.0.0.0` | **anything that can route to this machine** |

The third is not an equal option and the prompt does not present it as one. A
nick is self-asserted and uniqueness is not enforced, so `0.0.0.0` puts an
unauthenticated bus on every interface: anyone who can reach the host can read
every channel and post as anybody. Reasonable on a network you trust, an
accident anywhere else.

The unix socket exposes least, but the **bash helpers cannot use it** — there is
no unix-domain form of `/dev/tcp`. With a socket recorded they say so once and
work against the log instead, which is lossless; the `chat` binary's client
subcommands do speak it. The prompt says this at the point of choosing.

**With no terminal, nothing is asked and nothing is recorded.** The run uses
`127.0.0.1:7717`, says on stderr that it did and why, and leaves the file absent
so the next interactive run still gets the choice. Recording a default nobody
chose would be worse than not asking, because the file would then look like a
decision.

The file is `key=value`, one per line, `#` comments — not JSON, because the bash
helpers read it too and `jq` is this repository's ceiling for a runtime
dependency:

```
transport=tcp
bind=127.0.0.1
port=7717
```

or

```
transport=socket
socket=/home/you/.ai-chat/chat.sock
```

Hand-editing is supported and the file's own header says so. `transport=tcp`
with no `port=` means 7717. A socket path has a kernel length cap near 104
bytes, checked when the transport is chosen and again when it is read, so a path
that cannot work is refused with advice rather than failing later at `bind`.

`chat config` prints what is recorded, and changes nothing.

### Precedence

**An explicit flag beats the config; the config beats the built-in default.**

That order holds on both sides. For the server, `--bind`/`--port`/`--socket`
means a debug instance and the config is left alone. For a client:

| invocation | where it goes |
|---|---|
| `--local` | the log, never a socket |
| `--socket P` | that socket |
| `--host H` (`--port N`) | that server; without `--port`, the registered port, else the recorded one, else 7717 |
| no flags, a server registered | that server |
| no flags, none registered | one is started and adopted, then that server |
| no flags, and starting one failed | the log |

**The config no longer decides where a client connects; the registry does.** The
config is policy — how a bus should be exposed, your decision, outliving a
reboot. The registry is presence — where a bus actually is, expected to vanish.
A recorded transport with nothing registered means "no server is running", not
"connect here": dialling an endpoint nothing published is how a client ends up
talking to whatever else took that port.

A client never prompts. It may be in a pipeline, a hook or a CI step, and a
client that can block on a question is a client that can hang a build — so with
nothing recorded it acts locally, which needs no server and no decision.

**A configured server that cannot be reached falls back to the log, and says
so.** That keeps the promise that a dead server never blocks writing history.
The fallback is announced, never silent, and it applies only to the config path:
an explicit `--host` does *not* fall back, because someone who named a server
wants that server and quietly writing somewhere else would be worse than an
error.

### Where it differs from the interpreter tiers

Three defects filed against those tiers are not reproduced here: the next
message id comes from the highest id in the log rather than the last line
(B56), an unparseable line is skipped rather than fatal (B57), and the channel
lock is released on every path out of a failed append rather than left behind
(B52). The filed entries stay open — they are about the tiers, which are
unchanged.

The source is at `src/chat/` (CODE-STYLE 1b) and is dev-only; what ships is the
compiled artifact declared per target in `chat/binaries.tsv`. Users need no
Rust, no cargo and no interpreter.

## Helpers (all pure bash)

    scripts/lib-config.sh                      sourced by the helpers; reads <home>/config
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

`--host` with no `--port` takes the recorded port, then falls back to **7717**,
which is also the binary's built-in default — so the two agree with no flags.
The **interpreter tiers do not**: started without `--port` they bind a
kernel-assigned port and do not read the config. Against one of those, read the
real port from `$AI_CHAT_HOME/server.port` or start it with an explicit
`--port`.

With no `--host` the helpers consult the **registry** first and the config only
as a fallback, then fall back to the log with a note when nothing answers.
`--local` forces the log. The helpers do **not** auto-start a server: a helper
cannot know which runtime you want, so it names `chat serve` and moves on.

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
