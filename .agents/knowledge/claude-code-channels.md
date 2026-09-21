<!-- MODE: DEV -->
# Claude Code channels: what a server can push into a running session

A Claude Code session started with channels on accepts `notifications/claude/channel`
from an MCP server and shows it to the model, including while the session is idle
and while a tool is running. It has to be started with a hidden flag, it asks for
confirmation on every start, it does nothing under `claude -p`, and it fails
silently when it is not on. `chat-mcp` uses it for interrupts and timers
(`chat/docs/interrupts.md`); this is what was measured, on what.

Measured 2026-09-21 on Claude Code 2.1.278 (Claude Max login), in an interactive
session driven through tmux, model haiku 4.5. Each behaviour was seen once or
twice, not statistically.

## What works

| behaviour | measured |
|---|---|
| a push to an **idle** session | woke it with no key pressed, 1-4 s after the send; the model replied |
| a push during a **foreground tool** | the `← chat:` line appeared while the tool still showed "Running ... 12s" |
| a push while the turn is waiting on a **background shell** | started a turn |
| a repeating timer, `count: 2` | exactly two pushes, nothing more in the next 30 s |
| the TUI | shows `← <server>: <content>`; the `meta` attributes are not shown, the model sees them |
| the model | acted on pushed content every time in the bridge run; with a stand-in server, haiku ignored an instruction inside the content on 2 of 5 pushes, so content is treated as untrusted and a notice cannot be relied on to make the model do something |

The docs say queued events are delivered "on the next turn". What was seen is
finer: the next step inside a running turn. One sample, so do not build on it.

## What has to be true

- The server declares `capabilities.experimental["claude/channel"] = {}` at
  `initialize`; presence is what registers the listener. `instructions` is shown
  to the model as context. A stub that echoed the client's protocol version
  (2025-11-25) and `chat-mcp` (which answers 2025-06-18) were both accepted.
- The session is started with `--dangerously-load-development-channels server:<name>`.
  The flag is not in `claude --help`. It shows a blocking confirmation on every
  start, and `<name>` must be a server in the project's `.mcp.json`: with
  `--mcp-config` the TUI said "no MCP server configured with that name".
- `meta` keys are `[A-Za-z0-9_]+`; others are dropped silently. Each becomes an
  attribute of `<channel source="<server>" ...>`.
- Claude Code needs a claude.ai login or Console key; not Bedrock, Vertex or
  Foundry. Team and Enterprise accounts are blocked until an admin sets
  `channelsEnabled: true`, and that blocks the dev flag too. Documented, not
  measured here.
- Nothing tells the server whether anyone received a push. `notification()`
  resolving means only "written to the transport".

## What does not work

- `claude -p` with the dev flag: the server connected and the notification was
  written, but the model never saw it (two runs); the log said
  `nonInteractive=true` and registered no channel.
- Subagents: not tested.
- With `MCP_PROTOCOL_NEGOTIATION=auto` and a server that negotiates revision
  2026-07-28, the docs say it is not registered as a channel. Not tested.

## Side effects of testing this

A run writes folder-trust and MCP-server-approval entries for its working
directory into `~/.claude.json`. Point a test at a throwaway directory and remove
its `projects` key afterwards (`jq 'del(.projects[$dir])'` into a new file, check
the rest is unchanged, then rename over the original).

## What it means here

`wait` and `read` stay the contract; a push is an optimisation for an agent that
is busy or idle. Keep it additive: with no rule and no timer `chat-mcp` pushes
nothing, so a client that has channels off, or never registered the listener, sees
no difference. Re-measure after a Claude Code upgrade; the contract is a research
preview.
