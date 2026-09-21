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

## What the binary says about the gate

Read out of the 2.1.278 binary (`strings` on the executable, then reading the
minified code around each key), 2026-09-21. **Read, not run**: none of this was
exercised except the `--dangerously-load-development-channels` path.

- A `server:<name>` entry given to `--channels` is **never** admitted: the code
  answers "server X is not on the approved channels allowlist (use
  --dangerously-load-development-channels for local dev)". The dev flag is the only
  route for a bare MCP server.
- A `plugin:<name>@<marketplace>` entry is admitted when that pair is on the
  allowlist. The allowlist is the managed setting `allowedChannelPlugins` when
  one is set (it **replaces** the default, so it must list the approved ones too),
  else the default, which is the remote feature flag `tengu_harbor_ledger`: a list
  Anthropic controls, with nothing local that can add to it. The plugin must also
  be installed from that marketplace.
- `allowedChannelPlugins` and `channelsEnabled` are managed-settings keys
  (Linux: `/etc/claude-code/managed-settings.json` or `managed-settings.d/`; the
  directory does not exist on this machine). `channelsEnabled` blocks channels only
  for a claude.ai Team or Enterprise login (and, on a Console key, when managed
  settings exist without it), so a personal Pro or Max login is not blocked.
- The confirmation dialog belongs to the dev flag alone and has no memory: it is
  shown at every start whenever channels are on, and nothing in the code stores an
  acceptance. `--help` text for the flag says "Shows a confirmation dialog at
  startup".
- There is **no** setting or environment variable that turns channels on without
  the command-line flag: no key or variable with "channel" in it other than the
  two managed ones above and `CLAUDE_CODE_REMOTE_TOOLS_SESSION_CHANNEL`, which is
  unrelated.
- Channels are also gated by the remote flag `tengu_harbor` and by a first-party
  provider; a third-party provider, or the flag off, drops them without a message.

So the only ways to receive a push without `--dangerously-...`: be on Anthropic's
list, or package the server as a Claude Code plugin in a marketplace, install it,
put `plugin@marketplace` in `allowedChannelPlugins` in a managed-settings file
(root-owned, system-wide) and still pass `--channels plugin:...` at every start.
Neither has been tried.

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
