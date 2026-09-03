---
name: ai-text-editor
description: Use the local ai-text-editor server and short-lived client to inspect, search, navigate, edit, recover, and safely save text or raw-byte files through an agent-oriented protocol.
---

# ai-text-editor

Use this skill when an agent needs a durable editor tab, server-owned file
state, revision-aware searches, cursor navigation, undo/redo, raw-byte or hex
editing, large-file inspection, or external-change resolution. The server is
the only owner of document state, journals, indexes, and per-tab SQLite data.

## Start

The installed binaries may be outside PATH. Use the colocated client/server
paths described by the man page when necessary:

```text
ai-text-editor-server start --file /path/to/file
ai-text-editor open --file /path/to/file
ai-text-editor help
man -l ai-text-editor.1
```

The client uses a short-lived request connection. Save a session with
`--save-session-token PATH`, reuse it with `--session-token PATH`, or use
`--session ID`/`--agent ID` so the client stores and resolves the token under
the private editor metadata directory. Without those flags it checks
`TSCH_AI_EDITOR_AGENT`, `CODEX_AGENT_ID`, and `AGENT_ID`, in that order.
The server-issued token is tab-scoped, distinct from the TCP authentication
secret, and invalid after that server instance closes. Servers also register
their endpoint and token identity in `sessions.json`; identity lookup refuses
stale or ambiguous candidates rather than guessing. The registry is
server-written coordination metadata, not a client-accessible SQLite store.

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
   path-wildcard, Rust-regex, PCRE2-regex, and six fuzzy modes.
8. Request counts, pager keys, first-four defaults, context lines, line/byte ranges,
   ordering, completeness, result generations, stale-page detection, and
   explicit historical reads.
9. Build or inspect lazy 10,000-line/byte-offset indexes, including explicit
   ranges, full-index requests, granularity, cancellation, and resource limits.
10. Resolve external changes with backup, reload, merge, keep, or acknowledged
    force-save; `backup` preserves external bytes and leaves resolution
    pending, while other choices resolve the alert. External bytes can also be
    preserved atomically as `.back` during a discard or overwrite.
11. Stream or page large results with restart delimiters after writes, or use
    transparent restart for non-streaming output.
12. Recover journaled edits, inspect runtime state, and use Unix sockets or
    loopback TCP on Windows. TCP endpoints must remain private to the host.
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
5. Keep snapshots/commits in the surrounding workflow when useful; the editor
   journal is recovery history, not a replacement for Git history. TCP can
   require `--auth-token` or an owner-only `--auth-token-file`; the client proves it through a per-connection HMAC
   challenge and does not send the secret in the editor request. Never expose
   an unauthenticated endpoint publicly.

Large files above the server's 256 MiB threshold use file-backed bounded reads
(`read --offset N --length N`) and sparse index scans. Ordinary mutations are
refused. Exact text searches require `--range-start-line` and
`--range-end-line`; exact byte searches require bounded
`--range-start-byte`/`--range-end-byte`; unbounded scans are refused. To
rewrite one, start a job, then call `large-edit` with its `--job-id`,
`--acknowledge-large-edit`, byte range, and replacement; the server streams the
rewrite, atomically replaces the file, and completes the job. The agent accepts
the I/O duration and must keep the job result or release its token.

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
