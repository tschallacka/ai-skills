<!-- MODE: PROD -->
# chat-interrupt-plugin

A Claude Code plugin with one `PreToolUse` hook, for the chat skill's interrupts.
An agent that has told the chat bridge what may interrupt it (`interrupt_add`,
`timer_set`; see `chat/docs/interrupts.md`) is told, before its next tool runs,
which messages matched and which timers ran out.

## Why it exists

The way to interrupt a *running* Claude Code session from an MCP server is
"channels", which needs Claude Code started with a hidden development flag and a
confirmation dialog at every start. This hook needs neither. `chat-mcp` appends
each notice to a spool, and this hook empties the spool at the agent's next tool
call and returns it as `additionalContext`, which the model reads.

## What it does

- `PreToolUse`, every tool. Reads
  `${AI_CHAT_HOME:-~/.config/tsch-ai-skills/chat}/interrupts/<session id>/*.log`
  (the session id is `CLAUDE_CODE_SESSION_ID`, else the one in the hook's
  payload), takes the files, and returns one reminder listing up to 20 notices
  with the time each arrived. With nothing queued it prints `{}` and costs one
  small process.
- **It never blocks a tool call** and never sets a permission decision, so the
  normal permission flow is untouched.
- The messages stay unread: the reminder says to call the chat `read` tool for the
  full text. The text is another party's words and is labelled as not being
  instructions.

## What it cannot do

It only reaches an agent that is using tools. An idle agent sees nothing until
its next call; `wait` and channels (`interrupt_settings` with `delivery: push`)
are the ways to reach one. A notice written in the instant the hook empties the
spool may be shown a call late.

## Turning it on

The bridge writes the spool by default (`delivery: hook`). The hook has to be
loaded by Claude Code, for example `claude --plugin-dir chat-interrupt-plugin`, or
installed as a plugin.
