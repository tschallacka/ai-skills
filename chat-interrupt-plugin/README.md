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
  with the time each arrived.
- **It also reminds you to (re-)arm `chat-spool-watch`.** `chat-mcp` marks its
  own spool directory `.active` whenever a rule or a timer would still fire
  under `delivery: hook`. When that marker is present, the watcher's `.watcher`
  heartbeat is missing or more than 90 seconds old, and this has not already
  said so in the last 30 minutes, it adds a second reminder naming the command.
  This is the answer to a watcher that expired (a Monitor lives 30 minutes at
  most) or was never armed: nothing else notices that on its own.
- With neither a queued notice nor a re-arm reminder due, it prints `{}` and
  costs one small process.
- **It never blocks a tool call** and never sets a permission decision, so the
  normal permission flow is untouched.
- The messages stay unread: the reminder says to call the chat `read` tool for the
  full text. The text is another party's words and is labelled as not being
  instructions.

## What it cannot do

It only reaches an agent that is using tools. An idle agent sees nothing until
its next call; the re-arm reminder above only ever fires at that next call too,
so it cannot itself wake anyone. To actually wake an idle agent, arm
`chat-spool-watch` as a Claude Code Monitor: it prints a line when notices have
sat in the spool unread for five minutes, and that line starts a turn
(`chat/docs/interrupts.md`). Channels (`interrupt_settings` with
`delivery: push`) are the other way and need a start-up flag. A notice written
in the instant the hook empties the spool may be shown a call late.

## Turning it on

The bridge writes the spool by default (`delivery: hook`). The hook has to be
loaded by Claude Code, for example `claude --plugin-dir chat-interrupt-plugin`, or
installed as a plugin.
