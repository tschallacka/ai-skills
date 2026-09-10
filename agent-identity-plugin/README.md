# agent-identity-plugin

A Claude Code plugin that answers "which agent is calling?" for two
session-dependent tools in this repository (`ai-text-editor`, `chat`) without
ever asking the model to say who it is.

## Why this exists

Measured 2026-09-08/09 (`src/agent-session-key/HARNESS-IDENTITY.md`,
`.agents/knowledge/agent-identity-across-harnesses.md`): a subagent's
environment on Claude Code is byte-identical to its parent's, and Claude
Code's MCP wire carries only a per-call id (`claudecode/toolUseId`), never a
per-agent one. Nothing at either surface tells an MCP server which agent is
actually asking, so a server that shares one process across a session's
parent and every subagent — which this measurement confirmed happens — has
no way to keep their state apart on its own.

`PreToolUse` fires before every tool call and carries both that call's id
*and* the calling agent's id in the same payload. This plugin's `PreToolUse`
hook writes the pairing to a small per-session file; an MCP server (see
`agent_session_key::lookup_hook_register` and
`ai-text-editor-mcp`'s `resolve_caller_identity`) reads it back keyed by the
same call id the wire already gave it. Neither side ever asks the model for
an identity, and the September 2026 decision (T122's own logged note) is
explicit about why that matters: a value the model *could* supply is
advisory at best, and a mismatch would be something to refuse, not trust —
better not to offer it a value to disagree with at all.

## What it ships

- **`PreToolUse` (the hard half).** Appends
  `{"tool_use_id","agent_id","agent_type"}` to
  `$XDG_STATE_HOME/ai-skills/agent-identity/<session id>.jsonl` (or
  `$AI_SKILLS_AGENT_IDENTITY_DIR`, if set) for every tool call. This is what
  makes the MCP path exact and free: nothing for the model to remember,
  nothing for it to get wrong.
- **`SubagentStart` (the soft half).** Injects `AGENT_ID`/`AGENT_TYPE` into a
  new subagent's own context, and tells it to pass that id explicitly
  (`--session`, or `CHAT_SESSION_ID`) on any CLI call it makes with its own
  shell. This is advisory — the model has to actually do it — which is why it
  is *not* what the MCP path above relies on, but it is still the only
  identity a shell-invoked tool (one with no MCP form at all, like
  `interactive-shell`) can ever be given on this harness. See T123's register
  note for why that gap is left open rather than papered over.

## What this does not do

- **It does not enable skill-mode identity on Claude Code.** A binary
  invoked directly from a subagent's shell gets none of this — Claude Code
  hands a subagent a byte-identical environment and refuses to rewrite a
  command via a hook (`updatedInput` is read from the SDK's own permission
  handler, not from a `settings.json` command hook; measured, not assumed).
  The `SubagentStart` context above is the ceiling for that path: informed,
  not enforced.
- **It says nothing about codex or opencode.** Both can be given a real,
  zero-token per-call identity too — codex already puts one on its MCP wire
  outright (`ai-text-editor-mcp` reads it directly, no plugin needed there),
  and both can be made to inject one into a skill-mode shell command through
  their own native hook/plugin mechanisms. Building those is tracked
  separately (see the TODO register) rather than folded in here, because
  neither is installable or testable from this machine.

## Registration convention

`AI_SKILLS_AGENT_IDENTITY_DIR` and the `<session id>.jsonl` naming are shared,
byte-for-byte, between this plugin's hooks (`hooks/lib.sh`) and the Rust side
(`agent_session_key::hook_register_dir`/`hook_register_path`). Changing one
without the other breaks the join silently — the writer and the reader would
each compute a different path and simply never meet, with no error on either
side. Test both together, not separately, if either changes.
