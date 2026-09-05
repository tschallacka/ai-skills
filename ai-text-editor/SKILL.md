---
name: ai-text-editor
description: Use the local ai-text-editor server and short-lived client to inspect, search, navigate, edit, recover, and safely save text or raw-byte files through an agent-oriented protocol.
---

# ai-text-editor

Use this skill when an agent needs durable editor tabs, server-owned file
state, revision-aware searches, cursor navigation, undo/redo, raw-byte or hex
editing, large-file inspection, or external-change resolution. One server can
own multiple tabs; each tab has its own writer authorization and SQLite data.
The server is the only owner of document state, journals, indexes, and tab
metadata.

## Start

Just open the file. The server is not this skill's concern — `open` starts
one itself when none is running yet, and a server left idle stops itself
later (its journal replays on the next `open`, so nothing is lost):

```text
ai-text-editor open -f /path/to/file
```

Opening a path that does not exist yet is how you create a file: the tab
starts empty and the file appears on disk only when the first `save`
succeeds. Opening a second, unrelated file the same way does not start a second,
unrelated server: this agent's already-running workspace is found again
automatically, and the file is added to it as a new tab — the way opening a
file in an already-running IDE reconnects to that window rather than
launching a second copy. Reopening a file already open there reconnects to
its existing tab. This works with no flags at all inside a coding harness
(Claude Code, codex, opencode all export a session id this skill reads to
tell agents apart); across other kinds of callers, name a shared identity
explicitly with `--session ID` or `--agent ID` on every call, or pin one
server directly with `--endpoint ENDPOINT` (`ai-text-editor-server start
--file PATH` still exists for that, and for advanced options such as `--tcp`
or `--large-threshold-bytes`, but is not a normal step any more).
On Windows, where no usable Unix socket exists, the same plain `open`
autostarts a loopback-TCP server on an ephemeral port with a per-start
private token and discovers it exactly the way the socket is discovered —
every command in this skill works unchanged there.

Edits are journal-and-buffer operations: a successful `insert`/`replace`
returns a new revision but changes nothing on disk until a `save` succeeds.
Every mutating and reading response carries a `dirty` flag saying so; finish
an editing session with `save` and a fresh `read` (or `open`) showing
`dirty: false`. A command that did not apply always says why — errors are
written to stderr in every presentation with a code, a message, and the
recovery choices where there are any — so exit status plus stderr is the
complete story of any call; a silent success-shaped output means the work is
in the journal, not yet that it is on disk.

Every high-frequency flag has a short form (`-f`, `-l`, `-t`, `-o`, `-r`, …) —
run `ai-text-editor help` or `man -l ai-text-editor.1` for the exact,
current list rather than trusting prose here to have kept up with it.

The client uses a short-lived request connection. Save a session with
`--save-session-token PATH`, reuse it with `--session-token PATH`, or use
`--session ID`/`--agent ID` so the client stores and resolves the token under
the private editor metadata directory. Without an explicit id it falls back
to whatever the surrounding coding harness itself exports
(`CLAUDE_CODE_SESSION_ID`, `CODEX_SESSION_ID`, `OPENCODE_PID`) — this is also
what makes the automatic workspace reconnection above possible with no flags.
The server-issued token is tab-scoped, distinct from the TCP authentication
secret, and invalid after that server instance closes. Servers also register
their endpoint and token identity in `sessions.json`; identity lookup refuses
stale or ambiguous candidates rather than guessing. The registry is
server-written coordination metadata, not a client-accessible SQLite store.

An MCP adapter (`ai-text-editor-mcp`) exposes the same protocol over stdio
JSON-RPC for a client that speaks MCP instead of shelling out — same tools,
same resources (the protocol schema, the capability schema, this man page),
and the same autostart and workspace-reconnection behavior above: pass
`file` (and, to reconnect across a fresh, otherwise-forgetful context, an
`agent`/`session` argument) instead of an `endpoint`, and it resolves the
same way the CLI does.

## Capabilities

1. Open isolated editor tabs with one server writer per tab.
2. Inspect text as UTF-8, exact raw bytes, or 16-byte-row hex pairs.
3. Use one-based line and zero-based scalar columns for text; raw and hex use
   byte offsets. Invalid UTF-8 is never replaced silently: use raw or hex mode.
4. Opt into NFC normalization with explicit restoration-conflict reporting.
5. Create any number of numeric cursors; move home/end, by word, line, page,
   context, or explicit wrapped visual coordinates.
6. Insert, replace, delete, transact, restore normalized text when lossless,
   undo, redo, inspect history, and replay.
7. Search with explicit exact-text, exact-byte, wildcard, shell-wildcard,
   path-wildcard, Rust-regex, PCRE2-regex, and six fuzzy modes; all text modes
   are available on bounded large-file ranges.
8. Request counts, pager keys, first-four defaults, context lines, line/byte ranges,
   ordering, completeness, result generations, stale-page detection, and
   explicit historical reads.
   Large searches persist matches incrementally in bounded SQLite chunks and
   retain only the preview in server memory; page large result sets instead
   of expecting one unbounded response.
9. Build or inspect lazy 10,000-line/byte-offset indexes, including explicit
   ranges, full-index requests, granularity, cancellation, and resource limits.
   Large-tab startup records bounded prefix coverage; use `index` for an
   explicit full scan and inspect `index_complete` before relying on coverage.
   Index block output is paged by default; use `--offset` and `--limit` to
   inspect further persisted blocks without requesting an unbounded response.
10. Resolve external changes with backup, reload, merge, keep, or acknowledged
    force-save; `backup` preserves external bytes and leaves resolution
    pending, while other choices resolve the alert. External bytes can also be
    preserved atomically as `.back` during a discard or overwrite.
    Large-tab backup and reload are file-backed; large merge and force-save
    require an acknowledged bounded rewrite job.
11. Stream or page large results with restart delimiters after writes, or use
    transparent restart for non-streaming output.
    Streamed text reads are revision snapshots; if another request writes
    during delivery, discard frames before the delimiter and consume the fresh
    stream that follows it.
12. Recover journaled edits, inspect runtime state, and use Unix sockets or
    loopback TCP on Windows. TCP endpoints must remain private to the host.
    Active endpoint discovery records include the owning PID and server
    generation. A dead or unreachable Unix socket whose recorded owner is
    demonstrably gone is reclaimed automatically by the next `open` (which
    replays the journal); only when the owning process is still alive or
    unknown does a replacement require the explicit `--takeover-stale-endpoint`
    flag after verifying ownership.
13. Query `open` or the read-only `resources` command for available memory,
   estimated server overhead, recommended working set, and the active
   large-file threshold before accepting a costly rewrite.
14. Query the read-only `capabilities` command for the protocol's complete
    mode, coordinate, presentation, transport, and default-value contract.
15. Start and manage explicit server jobs with `job-start`, `job-poll`,
   `job-progress`, `job-complete`, `job-cancel`, `job-transfer`, and
    `job-release`; detached jobs can outlive a client connection. Use the
    acknowledged `large-edit` operation for streaming large-file rewrites.

## Agent responsibilities

1. Select every search and presentation mode explicitly when the default is not
   desired; do not infer fuzzy, regex, wildcard, byte, or path search.
2. Treat revisions, result generations, completeness, and stale-page errors as
   authoritative; refetch after a change instead of applying stale coordinates.
3. Acknowledge recovery, large-file work, force-save, and other safety prompts.
4. Decide whether external bytes need a `.back` copy and whether a closing tab's
   journal is preserved or explicitly cleaned. Backup failure blocks the action.
5. Every mutating request (`insert`, `replace`, `large_edit`, `restore`, `undo`,
   `redo`, and `save`) must include the revision most recently returned by
   `open`, `history`, or a completed mutation. Missing revisions are refused;
   stale revisions are never merged implicitly.
6. A request that names a file is only served by the tab holding that file;
   a request routed to a different tab is refused with `file_mismatch` and
   the fix is to `open` the named file first. Mutating `--offset`/`--delete-len`
   address bytes, not columns, and a delete that crosses a line end is
   reported as `spans_lines`.
7. Keep snapshots/commits in the surrounding workflow when useful; the editor
   journal is recovery history, not a replacement for Git history. TCP can
   require `--auth-token` or an owner-only `--auth-token-file`; the client proves it through a per-connection HMAC
   challenge and does not send the secret in the editor request. Never expose
   an unauthenticated endpoint publicly.

The server does not confine paths to a configured project root: an authenticated
client may open any path readable or writable by the server's operating-system
user. Treat the endpoint token as full file-access authority, keep TCP on
loopback, and use operating-system permissions or a separately isolated user
when stronger confinement is required.

Large files above the server's 256 MiB threshold use file-backed bounded reads
(`read --offset N --length N`) and sparse index scans. Ordinary mutations are
refused. All text searches require `--range-start-line` and
`--range-end-line`; exact byte searches require bounded
`--range-start-byte`/`--range-end-byte`; unbounded scans are refused. To
rewrite one, start a job, then call `large-edit` with its `--job-id`,
`--acknowledge-large-edit`, byte range, and replacement; the server streams the
rewrite, atomically replaces the file, and completes the job. The agent accepts
the I/O duration and must keep the job result or release its token.
Large raw reads are capped at 4 MiB so base64/JSON stays below the 8 MiB frame
ceiling; oversized result windows return `response_too_large` and should be
paged with `--offset` and `--limit`.

Large edits retain file-backed before/after snapshots in the tab journal
directory, so `undo` and `redo` restore them without loading the full file into
RAM. These snapshots consume disk space roughly proportional to edited
versions; use the close `clean` action when that recovery history is no longer
wanted.

Job commands manage lifecycle state; they do not execute arbitrary work for
the client. A driver that starts a job is responsible for performing or
delegating the work, reporting progress, completing or cancelling it, and
releasing its resume token when the result is no longer needed.

For exact schemas, coordinates, stream framing, search formulas, backup rules,
and errors, read [references/protocol.md](references/protocol.md).
