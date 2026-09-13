# agent-identity-plugin

A Claude Code plugin that hands each subagent its own identity the moment
it starts, so tools that key state per agent (ai-text-editor's tabs,
interactive-shell's socket name) do not silently merge a parent and its
subagent into one.

## Why this exists

Measured 2026-09-08 (T122): a Claude Code subagent's environment is
byte-for-byte identical to its parent's -- the only variable that ever
differed was a model setting, not an identity. `CLAUDE_CODE_SESSION_ID` is
one per top-level session and shared by every subagent under it (B303), so
nothing downstream of the shell can tell a parent and its subagent apart on
its own. The one place the harness does hand over a real per-agent id is
the `SubagentStart` hook payload: `agent_id` and `agent_type`, matching
character for character the id the Agent tool returned to the spawner and
the id a subagent's own tool calls carry on the wire.

## What it ships

`SubagentStart`, unmatched (this event is not tool-scoped). Reads the
hook's own JSON payload, and when `agent_id` is present emits
`additionalContext` naming it and instructing the subagent to declare it on
every call to a tool that can actually use it today:

- **ai-text-editor**: pass `agent`/`session` (its existing optional
  argument) on every `mcp__ai-text-editor__*` call, so its tabs reconnect
  to this agent's own workspace rather than whichever one answered last.
- **interactive-shell**: pass `--agent <id>` (or `export AGENT_ID=<id>`
  once in its own shell -- interactive-shell's own doc already reads
  `AGENT_ID` as a fallback) so its session socket is named per agent.
- **chat**: states plainly that chat-mcp does not yet accept a per-call
  identity override, so a subagent's chat/wait calls still share its
  parent's nick for now (chat-mcp holds one connection for its whole
  process lifetime; multiplexing that by resolved identity is a separate,
  not-yet-built feature, tracked as its own follow-up TODO filed alongside
  T122's own closure).

Nothing here blocks or rewrites a tool call. This is context, not an
environment variable a child process inherits -- the hook cannot set env
for a subagent, because a subagent is not a new process. Being context, it
is also soft: the model has to act on what it was told, which is why this
pairs with each tool's own scoping (a wrong or absent id is still just an
argument, not a bypass) rather than being a security boundary on its own.

## Where this does not (yet) generalize

`SubagentStart` is a Claude Code hook. opencode and codex were not measured
to have an equivalent lifecycle hook at the time this plugin was written --
on those harnesses a session-dependent skill still resolves identity
through `src/agent-session-key/`'s own ladder (env vars each harness
exports, then the worktree root), which already separates agents there by
other means (see `.agents/knowledge/agent-identity-across-harnesses.md`).
This plugin adds nothing on those harnesses; it is Claude-Code-only, and
the installer says so plainly rather than pretending otherwise.
