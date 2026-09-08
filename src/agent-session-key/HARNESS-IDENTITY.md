# MODE: DEV
# Harness identity: how a tool learns which agent is calling it

Every per-agent feature in this repository — the chat client's session file, the
editor's tabs, anything that must not let a subagent write into its parent's
state — needs one answer: *which agent issued this call?* The harnesses answer
it in three different places, and one of them does not answer it at all. This
file records what each harness provides, how it was measured, and what a
contributor must do to add a harness that is not listed here.

`src/agent-session-key/src/lib.rs` is the implementation. It is deliberately
pure — the environment and the worktree root are handed in — so every rung can
be tested without an actual agent, harness, or repository.

## The three places an identity can come from

A tool can be told who is calling in exactly three ways, and they are not
equally good:

1. **The environment**, read at process start. Cheap and universal, but it is
   only as granular as what the harness chose to export, and a process started
   once and reused across calls sees a single frozen value.
2. **The MCP call itself** — `params._meta`, or an injected argument. Per call,
   so it survives a long-lived server, and it needs no second channel.
3. **A hook, joined to the call by an id both sides see.** The hook knows the
   agent; the call knows its own id; a register maps one to the other.

Prefer them in that order of *directness*, not that order of preference: where
a harness puts a real agent id on the call, use it, because a value that arrives
with the call cannot go stale and needs no handoff.

## What each harness actually provides

Measured 2026-09-08/09 on this machine, with a logging passthrough registered as
its own MCP server and a hook or plugin recording its own input, so both sides
of one call were captured independently. "Wire" means `params._meta` of the
`tools/call` frame the server receives.

| Harness | Env var | Wire identity | Hook identity | Can a hook inject? |
|---|---|---|---|---|
| Claude Code | `CLAUDE_CODE_SESSION_ID` — session only, **shared with subagents** | `claudecode/toolUseId` (a call id, not an agent id) | PreToolUse: `agent_id`, `agent_type`, `tool_use_id` | No |
| codex | `CODEX_SESSION_ID` (`CODEX_THREAD_ID` measured equal to it) | `x-codex-turn-metadata.thread_id` — a real per-agent id | PreToolUse: `tool_use_id`, plus `agent_id`/`agent_type` **only on a subagent's call** | **Yes** — `updatedInput` with `permissionDecision: "allow"` || opencode | `OPENCODE_PID` — process granularity, not session | nothing but `progressToken` | plugin `tool.execute.before`: `sessionID`, `callID` | **Yes** — mutate `output.args` |

Three different shapes, so three different mechanisms:

**codex needs nothing.** `params._meta["x-codex-turn-metadata"]` carries
`session_id`, `thread_id`, `turn_id` and `thread_source`. `session_id` is the
CLI process; `thread_id` is the agent, and a subagent gets its own. Read
`thread_id` and stop. Measured: parent `thread_id == session_id` with
`thread_source: "user"`; subagent `thread_id 01a08304-96eb-…` with
`thread_source: "subagent"`, matching both the `SubagentStart` hook's `agent_id`
and the id the TUI prints at spawn.

**Claude Code needs the register.** Its `_meta` carries only
`claudecode/toolUseId` and `progressToken`. A tool use id identifies a *call*,
not an agent — two calls from one agent share nothing — and `progressToken` is a
per-connection counter, so neither is an identity on its own. The one place
`agent_id` appears is the PreToolUse hook, which fires before the tool runs. So:

1. a PreToolUse hook on the server's tool prefix records `tool_use_id -> agent_id`
2. the server reads `_meta["claudecode/toolUseId"]` from the request
3. it looks the id up, and now knows the agent

Measured on one subagent call, both sides recorded independently: hook
`tool_use_id toolu_011yDmciC7Zc5RHFC7VJoA5F` / `agent_id a6b475db023954c21`;
wire `_meta["claudecode/toolUseId"] toolu_011yDmciC7Zc5RHFC7VJoA5F`. A main
agent's hook payload carries `agent_id: null`, which is a usable identity for
the parent rather than a gap.

A hook cannot *supply* on Claude Code. `updatedInput` was tried twice — alone,
and alongside `permissionDecision: "allow"` — and the call ran unmodified both
times. The binary's own wording says why: `updatedInput` is read from the SDK
permission handler (`canUseTool`), not from a `settings.json` command hook. So
on Claude Code a hook can identify and refuse, but not rewrite.

**opencode needs injection.** Its wire carries no joinable id at all — the
plugin sees a `callID`, and that `callID` never reaches the server, so the
Claude Code join is impossible here. What opencode does allow is the thing
Claude Code refuses: `tool.execute.before` receives `(input, output)` and
mutating `output.args` changes the arguments the server receives. Measured: a
plugin setting `output.args.injected_session = input.sessionID` produced
`{"session":"ocmain2","injected_session":"ses_f7cf20aaaffeRqEqqHbOvc7VB3"}` on
the wire, and a subagent's call carried a **different** sessionID
(`ses_f7cf12523ffeBhKlWX1E2oVB6P`), so the granularity is per agent and not per
process. This is strictly better than `OPENCODE_PID`, which is the opencode
*process* and merges every session inside it.

## The skill side: a binary invoked from a shell command

A skill does not get an MCP request. It runs a binary, and that binary receives
exactly two things: its argv, chosen by the model, and its environment,
inherited from the harness. There is no `_meta` and no call id, so there is
nothing for a register to key on — the hook and the binary share no value. (A
join on the literal command text via `/proc/self/cmdline` collides as soon as
two agents run the same command, which is exactly when it is needed.)

So the shell side needs **env injection**, and it was measured the same way: a
probe recording its own `/proc/self/environ` and argv, run once by a main agent
and once by a subagent, on each harness.

| Harness | Env differs per agent? | Injection route | Skill side |
|---|---|---|---|
| Claude Code | **No** | none found | **unsolved** |
| codex | No (`CODEX_SESSION_ID` shared) | PreToolUse `updatedInput` | solved |
| opencode | No (`OPENCODE_PID` shared) | plugin `shell.env` | solved |

**Claude Code hands a subagent a byte-identical environment.** Comparing a
parent's environ with a `code-researcher` subagent's, the only variable that
differed at all was `CLAUDE_EFFORT` (a model setting). `CLAUDE_CODE_SESSION_ID`,
`CLAUDE_PID`, `CLAUDE_CODE_MESSAGING_TOKEN`, `AI_AGENT` and
`CLAUDE_CODE_CHILD_SESSION` were identical, and `CLAUDE_CODE_AGENT` read
`claude` for the subagent as well as the parent — so it names the harness, not
the agent. With no injection route either (`updatedInput` is refused for command
hooks), a Claude Code skill binary cannot learn which agent invoked it. The
identity has to come from the model, which costs tokens on every call.

**codex rewrites the command.** Its PreToolUse hook accepts `updatedInput`, but
only alongside `permissionDecision: "allow"` — without it the run fails with
`PreToolUse hook returned updatedInput without permissionDecision:allow`, which
is the error Claude Code never emits because it never reads the field. With
both, the rewrite lands: a hook prefixing `CODEX_AGENT_SESSION=<agent>` produced
a command that actually ran as
`CODEX_AGENT_SESSION=01a08321-381d-… /path/to/probe`, and the probe read the
variable. The parent's call carries `agent_id: null` (fall back to
`session_id`); the subagent's carries its own thread id, so parent and subagent
came out distinct.

**opencode supplies the environment directly.** The plugin hook `shell.env`
receives `{cwd, sessionID, callID}` and its `output.env` is merged into the
command's environment. A plugin setting `OC_AGENT_SESSION = input.sessionID`
gave the parent `ses_f7ce6206affe…` and the subagent `ses_f7ce460e0ffe…`, while
`OPENCODE_PID` was identical for both — so this is per-agent granularity where
the env var is only per-process.

Both working routes cost the agent nothing: no argument to pass, no token spent
per call, and nothing for the model to remember or get wrong.

## The resolution ladder

`resolve_session_key` is what a tool calls when it has no better answer, and it
is the fallback for harnesses that supply nothing per call. Rungs, in order:

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

Two rules the ladder must keep:

- **Nothing from the process tree.** `pid`, `ppid` and `getsid` were all
  measured to change between two invocations by the same agent, because a runner
  such as `timeout` or `env` gives a fresh pid every call. Inside codex's
  sandbox they are worse: pinned at 3/2/1 for every session on the machine, so
  they are stable *and identical*, which merges every codex agent into one.
- **A missing record fails closed.** Where identity comes from a hook register,
  no record means "unknown caller" and the call is refused. It must never
  silently attribute the call to nobody, or to whoever wrote last.

Where a harness does supply a per-call identity, that identity wins over the
ladder — the ladder is what runs when it does not.

## Adding a harness

Grok, OpenClaw, Cline and anything else are not measured here, because they are
not installed on the maintainer's machine. The measurement is the contribution:
a pull request adding a row to the table above, backed by the procedure below,
is welcome and is the only way a new harness gets supported.

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

Register it in the harness's own config under a distinct name (`wireprobe`), so
nothing you are testing is confused with a server you actually use. Back up the
config first, and restore it afterwards.

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
identifiable when the harness is simply reporting the process; the question is
whether two agents inside one process come out distinct.

### 4. Compare the two logs

- Does the wire's `params._meta` contain anything that differs between the
  parent's call and the subagent's? That is a per-call identity — use it.
- If not, does the hook payload carry an agent id, and does any id appear on
  *both* sides? That is a join — write the register.
- If neither, can the hook mutate the arguments? That is injection.
- If none of the three, the harness falls back to the ladder, and the honest
  answer is the env rung plus whatever granularity it actually has.

### 5. Measure the shell side too

The same two prompts, but running a probe binary that dumps its own
`/proc/self/environ` and argv. Diff the parent's environment against the
subagent's: if nothing but a model setting differs, the harness carries no
per-agent identity in the environment, and the question becomes whether a hook
can inject one. Try the harness's own rewrite or env hook and check the value
reached the probe — a hook that returns a rewrite the harness ignores looks
identical to one that works until you read the environment on the far side.

### 6. Report the granularity, not the variable
A row in the table needs to say whether the identity separates *agents* or only
*processes*. `OPENCODE_PID` is exported, stable, and looks like an identity; it
merges every session in one opencode instance. State which of the two a new
variable is, and say what was measured, so the next reader does not have to
re-derive it.

### 7. Remove the probe
Restore the backed-up config, delete the hook or plugin file and the shim, and
check that the harness's config no longer names the probe. A left-behind
passthrough silently logs every later call.

## Known-wrong things to not repeat

- `CLAUDE_CODE_SESSION_ID` is **shared between a parent and its subagents**
  (B303). Earlier notes in this repository called it "one per session and per
  subagent"; that was wrong, and it is why the Claude Code answer is the hook
  register rather than the env var.
- The codex `thread_id` is supplied by the client, so it is an identity *claim*,
  not a proof. That is fine for keying tabs — a codex agent lying about its
  thread id is the same trust boundary as it calling the tool at all — but it is
  not an authorization boundary. Where enforcement matters, the Claude Code hook
  route is the stronger one, because the hook is out of band from the model and
  a declared session that disagrees with the recorded agent is detectable.
- codex's hook trust gate is global, not per session: trusting writes
  `[hooks.state."<file>:<event>:<index>"] trusted_hash = "sha256:…"` into
  `~/.codex/config.toml`, and it survives every later session. The hash covers
  the **registration**, not the script: rewriting the hook script's contents at
  the same path did not re-prompt, while adding a second registered event did.
  So a trusted registration keeps trusting whatever that path later contains.
  Tool approval is a separate, also-persistent record, and it is per tool — the
  parent's "always allow" did not cover a subagent's first call.
