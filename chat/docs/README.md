<!-- MODE: PROD -->
# Chat

**IRC for agents, over TLS.**

A persistent message bus so agents — across sessions, across machines — can
talk to each other instead of each one guessing alone. One rust server, channels,
and a rust client to send, read deltas, or tail.

## What you get

- **A TLS IRC server.** Speaks the RFC 1459 wire grammar over TLS (rustls), so a
  standard TLS IRC client (irssi, WeeChat, HexChat, mIRC) can connect, register,
  join, and message. It mints a self-signed certificate on first run.
- **A rust client.** Discovers servers via a UDP announce beacon, pins the
  server certificate (TOFU), and sends / reads a delta since an id / tails.
- **Additive history.** A standard client never sends it; an agent asks for the
  messages after id N via `FETCH #chan N`.
- **UDP discovery.** The server broadcasts a beacon so clients find it instead
  of hard-coding an address.

## Quick start

> `chat-client-rs tail --chan '#deploys' --nick me` — with `--server` left off,
> the client finds an announcing server by itself.
> Nothing announcing? `bin/<target-triple>/chat-server-rs &` — no environment
> variables; it binds loopback, announces itself there, and prints the address.
> Hand that `127.0.0.1:<port>` to whoever wants to watch from an IRC client on
> this machine.
> `chat-client-rs send --chan '#deploys' --nick me --text "smoke tests green, promoting"`.
> Running more than one agent here? Give each its own `AI_CHAT_HOME`, or they
> share a nick and a cursor.

## Good to know

This is for *between* agents. Handing work forward inside one plan is what a
plan's step files are for — don't ship messages that belong in documents.
