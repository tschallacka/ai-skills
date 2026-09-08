# MODE: DEV
# Harness identity: resolving which agent is calling

Every per-agent feature in this repository — the chat client's session file, the
editor's tabs, anything that must not let a subagent write into its parent's
state — needs one answer: *which agent issued this call?* This file is the
implementation contract for answering it, and the procedure for adding a
harness that has not been measured yet.

**The measurements live in
[`.agents/knowledge/agent-identity-across-harnesses.md`](../../.agents/knowledge/agent-identity-across-harnesses.md)**
— what each harness puts on the MCP wire and in a shell command, what it refuses
to supply, and the beliefs that turned out to be false. Read that before
changing `HARNESS_ID_VARS`. The summary: codex supplies a per-agent id on the
wire, codex and opencode can both be made to supply one for free through a
plugin, and Claude Code supplies one on neither surface and must be joined
through a hook-side register instead.

`src/agent-session-key/src/lib.rs` is the implementation. It is deliberately
pure — the environment and the worktree root are handed in — so every rung can
be tested without an actual agent, harness, or repository.

## Where an identity can come from

Three places, and they are not equally good:

1. **The environment**, read at process start. Cheap and universal, but only as
   granular as what the harness chose to export, and a process started once and
   reused across calls sees a single frozen value.
2. **The call itself** — `_meta`, an injected argument, or an injected
   environment variable. Per call, so it survives a long-lived server, and it
   needs no second channel.
3. **A hook, joined to the call by an id both sides see.** The hook knows the
   agent; the call knows its own id; a register maps one to the other.

Where a harness supplies a per-call identity, that wins — a value arriving with
the call cannot go stale and needs no handoff. The ladder below is what runs
when it does not.

## The resolution ladder

`resolve_session_key`, in order:

1. **Explicit** — the caller's own `--session`/`--agent` argument, or the env
   var that caller names (`CHAT_SESSION_ID`, `TSCH_AI_EDITOR_AGENT`). Never
   inferred; an explicitly named session is shared on purpose.
2. **Harness** — every variable in `HARNESS_ID_VARS` that is set, combined.
   Combined rather than first-wins because harnesses nest: a codex launched from
   a Claude Code agent inherits that agent's `CLAUDE_CODE_SESSION_ID` unchanged
   and adds its own `CODEX_SESSION_ID`, so first-wins would give the inner codex
   the outer agent's session.
3. **Worktree** — the git worktree root, for agents that each work in their own
   checkout. Sibling worktrees must not share a key.
4. **Shared** — nothing distinguished this agent, so one key, named `shared` so
   a listing says so.

Three rules the ladder must keep:

- **Nothing from the process tree.** `pid`, `ppid` and `getsid` were measured to
  change between two invocations by one agent, and to be pinned identical for
  every session inside codex's sandbox — stable and identical is worse than
  unstable, because it merges every agent into one.
- **A missing record fails closed.** Where identity comes from a hook register,
  no record means "unknown caller" and the call is refused. Never silently
  attribute a call to nobody, or to whoever wrote last.
- **A supplied id is a claim, not a proof.** It comes from the client. That is
  fine for keying tabs; it is not an authorization boundary.

## Adding a harness

Grok, OpenClaw, Cline and anything else are not measured, because they are not
installed on the maintainer's machine. The measurement *is* the contribution: a
pull request adding a row to the knowledge entry's tables, backed by the
procedure below, is welcome and is the only way a new harness gets supported.

Do not add a variable to `HARNESS_ID_VARS` on the strength of documentation. All
three current entries were measured, and one of them (`OPENCODE_PID`) turned out
to be a worse identity than its name suggests.

### 1. Register a logging passthrough as its own MCP server

Two lines of shell, forwarding to a real MCP binary so the harness sees a
working server:

```sh
#!/usr/bin/env bash
set -uo pipefail
LOG="$SCRATCH/wire.jsonl"
exec tee -a "$LOG" | exec /path/to/a/real/mcp-binary
```

Register it under a distinct name (`wireprobe`) so nothing you are testing is
confused with a server you actually use. Back up the config first.

### 2. Register a hook or plugin that logs its own input

Whatever the harness calls its pre-tool event. Log the *whole* payload plus its
key list — the key set is part of what is being discovered, so filtering to
known keys throws away the answer.

### 3. Drive the harness for real

Not headless. Use `interactive-shell` to run the actual TUI, and issue two
prompts:

```
Call the MCP tool wireprobe <tool> exactly once, with argument session set to "main".
Delegate to a subagent and have it call the MCP tool wireprobe <tool> exactly once,
with argument session set to "sub".
```

The subagent prompt is the one that matters. A main-agent call can look
identifiable when the harness is merely reporting its own process; the question
is whether two agents inside one process come out distinct.

### 4. Compare the two logs

- Does `_meta` contain anything that differs between the parent's call and the
  subagent's? That is a per-call identity — use it.
- If not, does the hook payload carry an agent id, and does any id appear on
  *both* sides? That is a join — write the register.
- If neither, can the hook mutate the arguments? That is injection.
- If none of the three, the harness falls back to the ladder.

### 5. Measure the shell side too

The same two prompts, but running a probe binary that dumps its own
`/proc/self/environ` and argv. Diff the parent's environment against the
subagent's: if nothing but a model setting differs, the harness carries no
per-agent identity in the environment, and the question becomes whether a hook
can inject one. Try the harness's rewrite or env hook and **check the value
reached the probe** — a hook returning a rewrite the harness ignores looks
identical to one that works until you read the far side.

### 6. Report the granularity, not the variable

A row needs to say whether the identity separates *agents* or only *processes*.
`OPENCODE_PID` is exported, stable, and looks like an identity; it merges every
session in one opencode instance. State which of the two a new variable is, and
say what was measured.

### 7. Remove the probe

Restore the backed-up config, delete the hook or plugin file and the shim, and
check the harness's config no longer names the probe. A left-behind passthrough
silently logs every later call.

## What this decides for packaging

A session-dependent tool installs as an MCP on every harness. It installs as a
**skill** only where a plugin can supply the agent id for free — measured as
codex and opencode. On Claude Code the identity would have to come from the
model on every call, so the installer refuses the skill install there and says
why, rather than shipping a surface that silently attributes one agent's writes
to another.
