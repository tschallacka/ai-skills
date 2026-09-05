# MODE: DEV
# Maintainer notes: ai-text-editor

This is internals documentation for whoever next touches `src/ai-text-editor/`,
`src/stale-lock/`, or `src/agent-session-key/`. `SKILL.md` and the man page
(`ai-text-editor/ai-text-editor.1`) are the user-facing contract; this file is
the *why* behind the parts of the implementation that are not obvious from
reading one function at a time, written after a session that added autostart,
idle shutdown, and cross-file workspace reconnection all landed together and
interact in ways that are easy to half-remember later.

## The mental model: server is the workspace, client is the viewport

Think of it like an IDE. One `ai-text-editor-server` process is a workspace:
it can hold any number of open tabs (`Tab` in `ai-text-editor-server.rs`), each
with its own document, history, journal, and SQLite metadata. The
`ai-text-editor` client is not a persistent thing at all — every invocation is
a short-lived connection that sends one request and exits. It is the
*viewport*, not the workspace.

The agent driving this tool is expected to be forgetful in the way a fresh
Claude Code turn is forgetful: it may not remember it already opened file A
five minutes ago when it now opens file B. The whole point of the machinery
below is that this does not matter — `open --file B` from the same agent
finds the *same* server A is already open in, and B becomes a second tab
there, rather than spinning up an unrelated second workspace. Reopening a file
already open in that workspace reconnects to its existing tab rather than
recreating it, the same way opening a file already open in a running IDE
window focuses that window instead of starting a second copy.

## The server is not the agent's concern

`ai-text-editor-server` is not meant to be invoked directly any more. `open`
starts one itself when none is discoverable, waits for it to announce, and
proceeds — see `autostart_server` in `ai-text-editor.rs`. The binary is found
as a sibling of the client's own `current_exe()` first (the installed,
colocated layout), falling back to `PATH`.

**Windows has no usable Unix socket to autostart onto, so `autostart_server`
falls back to the loopback TCP transport**: `--tcp 127.0.0.1:0` (an
OS-assigned ephemeral port) plus a per-start nonce in a token file beside the
discovery record — a file rather than an argv flag so the secret stays out of
the process list, which the server already re-reads per connection anyway.
Nothing downstream needs to know: `announce()` publishes whatever endpoint
the server bound, so discovery, `Endpoint::is_live` probing, the session
cache, and `file_mismatch` all behave as they do for a socket. The token
travels back as `autostart_server`'s `Ok(Some(token))`, onto `Resolved`, and
into the cache like any other session secret. This is also why
`endpoint_owner_is_gone` is `#[cfg(unix)]`-only: on Windows there is no socket
file to reclaim, only a dead TCP address that `is_live` already sees.
`tests/tcp_flow.rs` drives this transport on every platform; the autostart
half of it can only run where the socket is genuinely absent, so its
assertions are `#[cfg(windows)]` and live in the Windows CI job.

A server that nobody is using stops itself too (`spawn_idle_watchdog` in
`ai-text-editor-server.rs`), after `--idle-timeout-seconds` (default 600, `0`
disables it). Restarting is cheap and lossless: the journal replays on the
next `open`, so there is no reason to keep an idle process around. **"Idle"
means no request in flight and no active job — not merely no request having
arrived recently.** A long synchronous operation (a large-file edit, an
unbounded-feeling search) holds `ServerState::in_flight` above zero for its
whole duration via `BusyGuard`, held across the entire connection in `serve`/
`serve_tcp`, not just the JSON dispatch. A detached job (`job-start
--detached`, the large-edit machinery) can be active with no connection open
at all, which is why the watchdog also checks `JobRegistry::has_active()`
across every tab before it will actually exit. Get either of these wrong and
the watchdog kills a server mid-edit — this was flagged explicitly as a
requirement, not an afterthought, so do not simplify it back down to a bare
"time since last response" check.

Two agents racing to autostart a server for the same file are coordinated by
a `stale-lock::StaleLock` on `<endpoint-discovery-path>.start.lock` (see
`autostart_server`): only one of them actually spawns; the other waits for
the lock, then finds the endpoint the winner already announced. See
"stale-lock" below for what this primitive does and does not guarantee.

**Known race, not yet worth fixing:** `autostart_server` only coordinates
against *other autostart attempts*, not against a server started by hand
(`ai-text-editor-server start --file X &`, which the shipped shell test at
`tests/test-ai-text-editor.sh` still does, predating autostart). If a
client's `open` call lands in the narrow window before a manually-started
server has bound its socket, `read_endpoint` sees nothing and autostart spawns
a second, competing server process for the same file, entirely outside that
test's own process tracking — it would not be killed by the test's cleanup
trap. In practice the manual server binds well inside the test's 100ms poll
interval, so this has not been observed to bite, but it is a real gap: the
long-term fix is updating that test to rely on autostart instead of a manual
start, not hardening the race further.

## client.rs: one resolver shared by the CLI and the MCP adapter

`src/ai-text-editor/src/client.rs` (`resolve`, `identity`, `cache_path`,
`persist_cache`, `autostart_server`) is a library module, not something
either binary owns. `bin/ai-text-editor.rs`'s `main` and
`ai-text-editor-mcp`'s `call_tool` both build a `ResolveRequest` from their
own argument source (CLI flags; MCP `arguments`) and call the same
`client::resolve`. This was a deliberate extraction, not the original shape:
the CLI had all of this inline in `main` first, and it moved into the lib
specifically so the MCP adapter did not grow a second copy that could drift
from the CLI's — which, before the extraction, is exactly what the MCP
adapter had (a hand-rolled `endpoint` argument requirement with none of the
resolution below).

## Identity: the `agent-session-key` ladder

`client::identity()` (client) and `register_session()` (server) both resolve
"which agent is this" through `agent_session_key::resolve_session_key` — the
same precedence ladder `chat-client-rs` resolves its own session key with,
pulled into its own crate (`src/agent-session-key/`) specifically so a second
tool could reuse it without depending on chat. It has not been back-ported
into chat-client-rs itself; that remains a candidate, not something this
session did, since chat is a separate skill with its own owner and test
suite.

The ladder, most specific first: an explicit id (a flag, or a caller-named
"told directly" env var) → the coding harness's own session env vars
(`CLAUDE_CODE_SESSION_ID`, `CODEX_SESSION_ID`, `OPENCODE_PID`, combined and
hashed so a nested harness gets its own key rather than inheriting the outer
one) → a git worktree root → one shared default. Full rationale and the
"why not pid/ppid" reasoning is in `agent-session-key`'s own doc comments.

**This tool does not use the worktree rung at all** (`None` is always passed
for `worktree_root`): identity here distinguishes *agents*, not *checkouts*,
and this is a per-file tool, not a per-project shared server the way chat is.

**A real regression, worth remembering exactly:** `CLAUDE_CODE_SESSION_ID` is
set in essentially every Claude Code shell — it is not a rare signal, it is
ambient. An earlier version of this change let the harness rung's *failure*
to find a registered session be fatal (`session::resolve(...).unwrap_or_else(
|error| die(&error))`), on the theory that "if the ladder resolved to
something, an identity lookup makes sense." That broke the very first plain
`open --file X` call in any Claude Code session: nothing is registered under
that harness key yet, `resolve` correctly says so, and the call died with
`session_stale` instead of ever reaching file-based autostart. **The fix is
not to gate the harness rung off when a file is given** (an earlier draft of
this fix did that, which quietly also disabled cross-file workspace reuse for
the common case — see the second lesson below); **it is to make identity
resolution failure non-fatal whenever a file is available as a fallback.**
See the `identity_lookup` variable in `client::resolve`: a miss falls through
to the ordinary per-file discovery/autostart path, exactly as if no identity
had resolved at all. Only "an identity was named and there is no file to fall
back on" (identity resolved to `Some`, no file was given, resolution still
failed) stays fatal — there is nothing else to try.

**A second regression, same family: an explicit identity that autostart
never told the new server about.** `--agent NAME`/MCP `arguments.agent` is an
explicit override, out of band from environment — nothing about it is
naturally visible to a spawned child process the way an ambient harness
variable already is (`Command` inherits the parent's environment, so
`CLAUDE_CODE_SESSION_ID` reaches the child for free; a name that arrived as a
flag or a JSON field does not). `register_session` (server side) only ever
reads *its own* environment; it has no argv concept of `--agent`. Without
`autostart_server` explicitly setting `agent_env_var` on the spawned child, a
client resolving `--agent NAME` and the server it just autostarted register
two *different* keys — the client's literal `NAME` and the server's own
harness-derived key — and every subsequent file for that name falls through
to yet another new server instead of reconnecting, exactly the failure mode
cross-file reconnection exists to prevent. Caught while testing the MCP
adapter specifically (its `arguments.agent` has no ambient env fallback the
CLI's own `TSCH_AI_EDITOR_AGENT` sometimes coincides with), then re-confirmed
on the CLI's `--agent` flag too, which had the identical latent bug.

**Two different questions need two different registry lookups.** Once an
agent has two or more tabs open, they all share one `agent_id` in
`sessions.json` but each has its own `token_id`/tab. `session::resolve`
demands exactly one unambiguous match and rightly refuses to guess between
several — that is correct for "resume the one tab I was working on"
(`open --agent NAME` with no file to disambiguate with; a silent wrong-tab
guess there would be worse than an explicit `session_ambiguous` error).
It is the wrong function for "where is my workspace, so `--file X` can route
to (or create) X's own tab within it" — that question only needs *a*
reachable copy of the server, since every tab on one server shares its
endpoint and the file-based routing in `select_tab` (server side) does the
rest. That second question is `session::resolve_workspace` — tolerant of any
number of tabs sharing the identity, picking the most recent reachable
distinct endpoint. `client::resolve`'s `identity_lookup` picks between the
two based on exactly one condition: `method == "open" && file.is_some()`.

**The server routes by `session_token` before it ever looks at `file`.**
`select_tab`'s "open" handling checks `envelope.session_token` first,
unconditionally, for every method — if it is `Some` and matches a tab, that
tab wins regardless of what `file` says in the payload. This means an
`open --file X` request must never carry a *different* tab's session_token,
or the server silently reconnects to that other tab instead of routing to X.
Both places a session_token could leak in from a stale source
(`identity_lookup`'s registry hit, and the local session-path cache below)
explicitly withhold it exactly when `method == "open" && file.is_some()`, and
only then.

**The two invariants above were not both true until the brutal-review
round.** The prose above claimed the session-path cache withholds the token
for open-with-file like the registry branch does; the cache branch never
actually did it, and a first real test-drive proved the consequence: a
refused request was still persisted into the per-`(identity, file)` cache
with the *answering* tab's endpoint+token, so every later command for that
file replayed the misroute — the "large files wedge, small files work"
finding in `.Ai-EDITOR-BRUTAL-REVIEW` was this, not a size problem. Three
rules now hold it shut, and tests in `tests/cli_flow.rs` pin each:
`Endpoint::is_live` (transport) gates every endpoint the client is about to
trust (cache hit and file discovery alike — a `.endpoint` file outlives a
`kill -9`), a refused request persists nothing (CLI and MCP adapter both
check `failed` first), and `select_tab`'s `ensure_tab_file` refuses, server-
side, a request whose payload names a file the routed tab does not hold
(`file_mismatch`), so the last line of defense does not depend on the client
being careful.

Two response-state facts the same review asked for and which later changes
must keep true: mutating and reading responses carry `dirty` (buffer differs
from disk — the "edits are journal-only until save" contract made visible),
and a text-presentation CLI call that was refused says so on stderr with the
server's code and message (the old silent-discard of error frames is what
made stale revisions and wedged sessions indistinguishable from success).
`autostart_server` captures the spawned server's stderr into
`<discovery>.start-<pid>.log` and folds it into a start failure — "never
announced an endpoint" with no cause was how F5's missing-file failure
read; the server side now opens missing paths as empty tabs and reclaims an
endpoint whose owning pid is demonstrably dead without
`--takeover-stale-endpoint` (unknown owners still require the flag).

## The client's local session cache is per-`(identity, file)`, not per-identity

`client::cache_path(identity, file)` hashes identity *and* file into the cache key.
It used to be identity alone, and that was a real bug, caught by hand while
testing this session's other changes rather than by any existing test: with
a single cache slot per identity, opening file B after file A overwrote A's
cached session_token with B's (every response to `open` returns the
responding tab's own `session_token`, and the cache is unconditionally
rewritten at the end of every call). A later `read --file A` would then find
the cache "hit" for that identity, forward B's session_token because it was
the only one cached, and the server would route the request to B's tab
before ever consulting `file` — silently returning B's content for a request
that named A. Confirmed exactly this way while testing: `read -f a.txt`
returned `file B content`. Keying the cache per file closed it, and it was
re-verified afterward by opening three files in several interleaved orders
(`a b c b a c c a`) and reading them back in a different interleaved order —
every open reported its own path and every read returned its own content,
regardless of order or repetition.

The no-file identity resume case (`open --agent NAME` alone) still uses a
single slot per identity (`file: None`) — there is no file to key by, and
that call means "resume whatever tab I was last using," which a single slot
answers correctly.

## stale-lock: what the guarantee actually is

`src/stale-lock/` is a generic `create_new`-based advisory file lock,
generalized out of what was a private `RegistryLock` inside
`ai-text-editor::session`, specifically so more than one place could use it
(the session registry's own lock, and the server-start coordination lock
above) without copy-pasting the same handful of lines with a different name
each time.

Release does **not** depend on `Drop` running. Reclaiming an abandoned lock
is entirely `mtime`-based, inside `acquire`'s own retry loop: a waiter checks
the file's last-modified age against its own `stale_after` and deletes it if
too old, whether the original holder is alive, crashed, or was killed
(`panic = "abort"` in this workspace's release profile means a panicking
thread aborts the *entire process* with no unwind — `Drop` never runs, and
that is fine, because nothing about release depends on it having run). The
one thing `Drop` running normally *does* need to get right: a lock stolen out
from under a still-alive holder (its own `stale_after` elapsed while it was
legitimately still working) must not have its file deleted by that holder's
eventual normal drop — that would release a lock the new holder is actively
relying on. That is why `StaleLock` writes a token into the file at acquire
time and only removes it at drop time if the file still holds that same
token; a mismatch means someone else reclaimed this path, and the right
behavior is to leave their file alone. Covered by
`a_stolen_locks_original_holder_does_not_delete_the_new_holders_lock` in
`stale-lock`'s own tests.

## CLI ergonomics

Every high-frequency flag (`--file`, `--line`, `--column`, `--action`,
`--text`, `--query`, `--offset`, `--expected-revision`, …) has a short alias;
see `help()` in `ai-text-editor.rs` for the current list. Safety
acknowledgements (`--acknowledge-force-save`, `--acknowledge-large-edit`) and
auth/session flags are deliberately kept long-form only — those are exactly
the flags where a terse typo should not silently do the dangerous thing.

`SKILL.md` describes capabilities in prose; it does not restate every flag,
because the flag surface changes faster than that document should need
editing. The generated man page (`ai-text-editor.1`, served over MCP as the
`ai-text-editor://ai-text-editor.1` resource) and `help()`'s own output are
the sources of truth for exact flag syntax — point there rather than letting
prose and code drift apart again.
