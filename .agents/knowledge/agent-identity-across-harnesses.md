<!-- MODE: DEV -->
# Which agent is calling? What each harness will tell a tool

Measured 2026-09-08/09. **No harness identifies the calling agent on every
surface, and Claude Code identifies it on neither surface by itself.** Two of
three can be made to supply it for free on both; Claude Code can be made to
supply it on neither, and needs a hook-side register to *recognise* the caller
instead.

| harness | version / model | MCP call carries an agent id? | shell command carries one? |
|---|---|---|---|
| Claude Code | 2.1.263, opus | **no** — only a call id | **no** — subagent env is identical to parent's |
| codex | 0.153.2, gpt-5.6-luna | **yes** — `thread_id`, per agent | only via a hook rewrite |
| opencode | 1.18.29, big-pickle | **no** — only `progressToken` | only via a plugin env hook |

The distinction that matters throughout: a **call id** is unique per tool call,
so two calls from one agent share nothing and it identifies nobody on its own.
An **agent id** is stable for an agent across its calls. Several harnesses ship
the first and look like they ship the second.

## How it was measured

The same shape on every harness, so the rows are comparable:

- **MCP side.** A logging passthrough registered as its own MCP server
  (`tee -a "$LOG" | exec <a real mcp binary>`), so the frames recorded are the
  ones a server actually receives rather than an adapter's parse of them.
  Alongside it, a hook or plugin logging its own stdin, whole, including the key
  list — the key set is part of what is being discovered, so filtering to known
  keys throws away the answer.
- **Shell side.** A probe binary dumping its own `/proc/self/environ` and argv
  to a log.
- **Both sides, both agents.** Each harness was driven for real through
  `interactive-shell` — not headless — and asked twice: call the probe directly,
  then delegate to a subagent and have *it* call the probe. The subagent leg is
  the one that matters; a main-agent call can look identifiable when the harness
  is merely reporting its own process.

## The MCP side

**codex puts a per-agent id on the wire.** `params._meta` carries `callId`,
`threadId`, and an `x-codex-turn-metadata` object holding `session_id`,
`thread_id`, `turn_id`, `thread_source`, plus workspace, git, sandbox and model
detail. `session_id` is the CLI process; `thread_id` is the agent:

    parent    session_id 01a082fd-07fc-…  thread_id 01a082fd-07fc-…  thread_source "user"
    subagent  session_id 01a082fd-07fc-…  thread_id 01a08304-96eb-…  thread_source "subagent"

The subagent's `thread_id` equals the `SubagentStart` hook's `agent_id` and the
id the TUI prints at spawn — three independent sources, same value. A server
reads `thread_id` and is done.

**Claude Code puts only a call id on the wire**, `claudecode/toolUseId`, plus a
`progressToken` that is a per-connection counter. Neither is an identity. The
only place `agent_id` appears is the PreToolUse hook, which fires before the
tool runs, so the two must be joined. Measured on one subagent call with both
sides recorded independently:

    hook  tool_use_id                   toolu_011yDmciC7Zc5RHFC7VJoA5F
          agent_id                      a6b475db023954c21
          agent_type                    general-purpose
    wire  _meta["claudecode/toolUseId"] toolu_011yDmciC7Zc5RHFC7VJoA5F

A main agent's hook payload carries `agent_id: null`, which is a usable identity
for the parent rather than a gap. The hook key set is `agent_id`, `agent_type`,
`cwd`, `hook_event_name`, `permission_mode`, `prompt_id`, `scratchpad_dir`,
`session_id`, `tool_input`, `tool_name`, `tool_use_id`, `transcript_path`. It
fires for MCP calls, not only Bash.

**opencode puts nothing joinable on the wire.** `_meta` holds `progressToken`
alone. Its plugin sees a `callID`, and that `callID` never reaches the server,
so the Claude Code join is impossible here. What opencode allows instead is
mutation: `tool.execute.before` receives `(input, output)` and changing
`output.args` changes the arguments the server receives. A plugin setting
`output.args.injected_session = input.sessionID` produced, on the wire:

    {"session":"ocmain2","injected_session":"ses_f7cf20aaaffeRqEqqHbOvc7VB3"}

and the subagent's call carried `ses_f7cf12523ffeBhKlWX1E2oVB6P` — a different
value, so the granularity is per agent.

`clientInfo` is generic on all three (`claude-code`, `codex-mcp-client`,
`opencode`) and identifies nobody.

## The shell side

A skill runs a binary, which receives argv — chosen by the model — and an
environment. There is no `_meta` and no call id, so there is nothing for a
register to key on: the hook and the binary share no value. (A join on the
command text via `/proc/self/cmdline` collides as soon as two agents run the
same command, which is exactly when it is needed.) So the only mechanism is env
injection.

**Claude Code hands a subagent a byte-identical environment.** Diffing a
parent's environ against a `code-researcher` subagent's, exactly one variable
differed, and it was `CLAUDE_EFFORT` — a model setting, not an identity. These
were identical on both sides:

    CLAUDE_CODE_SESSION_ID    1de1736f-ae05-4cef-88a4-cecdf6e96510
    CLAUDE_PID                4876
    CLAUDE_CODE_MESSAGING_TOKEN  361efa66cb4df7ae85ead2023fe83561
    CLAUDE_CODE_CHILD_SESSION 1
    AI_AGENT                  claude-code_2-1-263_agent
    CLAUDE_CODE_AGENT         claude

`CLAUDE_CODE_AGENT` reading `claude` for a `code-researcher` subagent is the
trap: it names the harness, not the agent. And there is no injection route —
`updatedInput` was tried both alone and with `permissionDecision: "allow"`, and
the command ran unmodified both times. The binary's own strings say why:
`updatedInput` is read from the SDK permission handler (`canUseTool`), not from
a `settings.json` command hook. One documentation line reads "updatedInput —
Modified tool input (PreToolUse only)", which describes *when* the field is
valid rather than *who* may send it; reading it as a hook capability is what
cost two attempts.

**codex rewrites the command.** Its PreToolUse hook honours `updatedInput`, but
only with `permissionDecision: "allow"` alongside. Without it the run fails
loudly — `PreToolUse hook returned updatedInput without permissionDecision:allow`
— which is the error Claude Code never emits, because it never reads the field.
With both, the rewrite lands and the command genuinely ran as:

    CODEX_AGENT_SESSION=01a08321-381d-7ad0-a30f-d6246396d085 /path/to/probe cx-parent

`CODEX_SESSION_ID` is identical for parent and subagent, so the env alone is
session granularity; the hook's `agent_id` is what supplies agent granularity
(`null` for the parent, the subagent's own thread id for a subagent), and
injection carries it across.

**opencode supplies the environment directly.** The plugin hook `shell.env`
receives `{cwd, sessionID, callID}` and its returned `output.env` is merged into
the command's environment:

    parent    OC_AGENT_SESSION=ses_f7ce6206affeZj7BO5fnNCntoP   OPENCODE_PID=1229369
    subagent  OC_AGENT_SESSION=ses_f7ce460e0ffeCKKlc5NrZe1ykb   OPENCODE_PID=1229369

Per-agent where the exported variable is only per-process.

## Refuted beliefs

- **`CLAUDE_CODE_SESSION_ID` is not per subagent.** A subagent shares its
  parent's, byte for byte (B303). Earlier notes in this repository said
  otherwise; that is why the chat client needs a nick suffix to keep a parent
  and its subagent out of one session file.
- **`OPENCODE_PID` is not a session id.** It is the opencode process, so every
  session inside one instance shares it, and a recycled pid can adopt a dead
  instance's state. It is exported, stable, and looks like an identity.
- **A hook that returns a rewrite is not necessarily a hook that rewrites.** On
  Claude Code the probe emitted documented JSON, exited 0, and changed nothing.
  A rewrite hook is only proven by reading the far side.
- **codex's hook trust is not content-keyed.** Trusting writes
  `[hooks.state."<file>:<event>:<index>"] trusted_hash = "sha256:…"` into
  `~/.codex/config.toml` and survives every later session, but the hash covers
  the *registration*: rewriting the hook script at the same path did not
  re-prompt, while adding a second registered event did. A trusted registration
  keeps trusting whatever that path later contains. Tool approval is a separate
  persistent record and is per tool — a parent's "always allow" did not cover a
  subagent's first call.
- **Nothing from the process tree identifies an agent.** `pid`, `ppid` and
  `getsid` change between two invocations by one agent, because a runner such as
  `timeout` or `env` gives a fresh pid. Inside codex's sandbox they are worse:
  pinned at 3/2/1 for every session on the machine, so they are stable *and*
  identical, which merges every codex agent into one.

## What it means here

**A session-dependent tool is installable as an MCP on all three harnesses, and
as a skill on two.** On codex and opencode a plugin supplies the calling agent's
id for free — no argument, no tokens, nothing for the model to remember. On
Claude Code a skill binary cannot learn who invoked it, so the identity would
have to come from the model on every call, which is a per-call token cost for
something the harness already knows.

So the installer gates the skill install **per harness** rather than dropping it
everywhere: offered on codex and opencode, refused on Claude Code with the
reason stated. The MCP install is offered everywhere.

Where a harness supplies a per-call identity, it wins; `resolve_session_key` in
`src/agent-session-key/` is what runs when it does not, and
`src/agent-session-key/HARNESS-IDENTITY.md` carries that ladder and the
procedure for adding a harness not measured here.

One caveat on the ids that arrive with a call: they are supplied by the client,
so they are an identity *claim*, not a proof. That is fine for keying tabs — an
agent lying about its id is the same trust boundary as it calling the tool at
all — but it is not an authorization boundary. Claude Code's hook route is the
stronger one there, because the hook is out of band from the model, so a
declared session that disagrees with the recorded agent is detectable.
