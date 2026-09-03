# ai-text-editor protocol reference

The authoritative transport is versioned NDJSON: one request per short-lived
connection and zero or more ordered response frames ending in `complete`.
Structured output is the default; text, paging, and streaming are explicit
client presentation choices.

`server start --file PATH` creates the initial tab and shared endpoint.
`open --endpoint ENDPOINT --file PATH` selects an existing tab by canonical
path or creates a new isolated tab in that server. Each tab receives a distinct
session token and metadata database; a token for one tab cannot authorize
another. Closing one tab leaves the endpoint alive while other tabs remain.

Unix sockets are preferred. Loopback TCP is the Windows fallback; the server
requires `--auth-token` and performs a per-connection challenge/proof exchange.
The client sends the secret only to its local process: the TCP request after
authentication does not contain `auth_token`. The fallback must never be bound
publicly.

Endpoint discovery records are atomically replaced JSON containing `endpoint`,
the owning `pid`, the server `generation`, and `status: active`; clients may
still read legacy plain endpoint files. Graceful shutdown removes the active
record. A crashed Unix server leaves its socket and record in place so that
the next server cannot silently impersonate it. Startup refuses an unreachable
socket until the operator explicitly supplies `--takeover-stale-endpoint`; the
old record is renamed with a `stale-` suffix before replacement. The agent is
responsible for verifying the recorded owner is gone before taking over.

For TCP, the server first sends a `challenge` frame containing a fresh 32-byte
URL-safe `nonce` and process `generation`. The client replies with an
`authenticate` envelope whose `payload.proof` is HMAC-SHA256 over the versioned,
length-prefixed tuple `(nonce, request_id, generation)`, using the configured
secret. The following request must use the same `request_id`; the server also
requires the challenge nonce to match exactly. Failed proofs are rejected
before the editor handler runs.

The server accepts either `--auth-token TOKEN` or an owner-only
`--auth-token-file PATH`, never both. With a credential file, the server rereads
the file for each TCP connection, so replacing it rotates the accepted secret;
clients must update their saved session token. On Unix, group/world-readable
credential files are refused.

Every response frame carries a zero-based `sequence`. Data frames also carry
`byte_count`, the UTF-8 byte length of their canonical JSON `payload`. A
consumer can therefore detect skipped frames and account for output without
parsing presentation text.

The protocol frame ceiling is 8 MiB. Large raw reads default to and are capped
at 4 MiB before base64/JSON framing; oversized result or index windows return
`response_too_large` and must be requested as smaller pages.

`open` returns a server-issued `session_token` and `server_generation`. The
client may persist them with `--save-session-token PATH`; subsequent requests
must send that session token, either by using `--session-token PATH` or by
providing it to an equivalent protocol client. The token authorizes one tab,
is invalidated when that server instance closes, and is never a substitute for
the TCP authentication secret. A request without a valid token fails with
`session_unauthorized` before it can mutate, journal, or create a result.
Servers record session candidates in the private editor `sessions.json` with a
token ID, tab UUID, server generation, endpoint, PID, start time, and optional
agent identity. `--session ID` and `--agent ID` resolve those candidates in
newest-first order, reject ambiguity, and refuse unreachable stale records;
they never silently start or impersonate a server.

`capabilities` is read-only and returns the complete protocol mode list,
coordinate bases/units, presentation modes, fuzzy-gradient meanings,
large-file restrictions, transport choices, and server defaults. Agents should
query it when they need machine-readable behavior instead of parsing help text.

Large-tab startup builds only a bounded prefix of the default 10,000-line index
and persists it as incomplete coverage. `open` reports `index_complete` and
`index_coverage`. The explicit `index` operation performs a complete scan for
the requested `granularity`, persists it, and returns `complete: true`; index
data is an optimization and never changes search correctness. Index inspection
returns four blocks by default; use `offset` and `limit` to page the persisted
block list. The response reports `block_count`, `block_offset`, and
`returned_blocks`, so an agent never has to request an unbounded index frame.

## Coordinates and modes

1. `text_utf8`: one-based lines, zero-based Unicode scalar columns.
2. `raw_bytes`: one-based logical lines, zero-based byte offsets.
3. `hex_view`: 16-byte rows, complete byte-pair edits only, never nibbles.
4. Invalid UTF-8 returns `invalid_utf8` for text operations without replacement
   characters; raw/hex modes preserve and save exact bytes.
5. NFC is opt-in with the server's `--normalize-nfc` startup option.
   Coordinates continue to address stored bytes; normalized search results
   report positions in the normalized presentation. Mapping-preserving edits
   may restore original bytes; lossy edits return `restoration_conflict`.
   The `restore` operation explicitly disables normalized presentation when the
   mapping is still lossless; it returns `restoration_conflict` after a lossy
   normalized edit.
6. `wrap_width` is optional and reports visual coordinates in addition to the
   stored logical cursor. Visual rows are one-based and columns are zero-based
   within the wrapped row; newline boundaries always start a new visual row.
   Send `visual: true` with `line` and `column` to interpret the input as a
   visual position. A visual request must include a positive `wrap_width`.
   Responses always retain logical `line` and `column`, and include `visual`
   and `wrap_width` when requested.

## Search

`history` is read-only and returns the current revision, undo depth, redo depth,
and journal sequence. It does not expose another tab's journal or document.
`resources` is read-only and reports host-available memory when the platform
exposes it, estimated server overhead, a recommended working set, and the
active large-file threshold. `open` includes the same report.
`begin_transaction` and `end_transaction` explicitly group ordinary edits into
one undo step; each individual edit is still journaled for crash recovery.

Mutating methods `insert`, `replace`, `large_edit`, `restore`, `undo`, `redo`,
and `save` require the envelope's `revision` field. The server refuses a
missing field with `revision_required` and refuses a value other than the
current revision with `stale_revision`; it never silently treats an omitted
revision as last-write-wins. Read `open` or `history` again after either error.
`save_as`, `close`, and `resolve_external` have their own target or recovery
decisions and are not covered by this list.

The server intentionally has no project-root jail. A valid session token grants
the server user the same path access as the server process. Keep endpoints
private and use OS-level isolation when a client must not access other files.

Search mode is required and must be one of `exact_text`, `exact_bytes`,
`wildcard`, `shell_wildcard`, `path_wildcard`, `regex_rust`, `regex_pcre2`,
`fuzzy_edit`, `fuzzy_subsequence`, `fuzzy_token`, `fuzzy_ngram`,
`fuzzy_phonetic`, or `fuzzy_soundex`. Results include line/column coordinates,
matched contents, a revision-bound result identifier, count, pager key,
completeness, and the first four matches when no limit is supplied.

`path_wildcard` matches canonical absolute document paths under an explicit
search root; it does not search line contents and returns null line/columns.
Fuzzy searches accept an optional `gradient` from `0.0` through `1.0`; its
meaning is strategy-specific and its default is reported by help/capabilities.
For edit distance it is the permitted distance fraction; for subsequence,
token, and n-gram modes it is the minimum score; phonetic modes use a binary
match score.

Search responses include `search_range` with the requested line and byte
bounds. For a large tab, every text search mode requires an explicit inclusive
`range_start_line`/`range_end_line`; an omitted end line is refused rather than
silently scanning an unbounded file. Large `exact_bytes` searches instead
require an explicit half-open `range_start_byte`/`range_end_byte` range no
larger than the bounded read limit and return absolute `byte_start`/
`byte_end` coordinates with base64 contents.
Large searches scan their bounded range incrementally and persist matches in
SQLite chunks; only the requested preview is retained in server memory. Use
`page` with the returned `pager_key`, `offset`, and `limit` to retrieve a
bounded slice without loading the complete result set into memory. A failed
scan remains incomplete and cannot be paged as a complete result.
Small-tab `exact_bytes` results likewise include absolute `byte_start` and
`byte_end` fields. Their line/column fields describe the start and end
coordinates separately, so a match crossing a newline is not represented by
an invalid same-line column range.

`page` rejects an unavailable or post-edit result by default with
`stale_result`. An agent may explicitly send `historical: true` (the CLI flag
is `--historical`) to read a complete persisted result from its original
revision. Such a response includes `source_revision` and `stale: true`; it is
read-only and must not be used as current edit coordinates.

## External changes and saves

The next agent interaction receives an external-change alert. The error's
`details` object contains the byte count, allowed choices, and required force-save
acknowledgement; the agent chooses
reload, merge, keep, or acknowledged `force_save`. `merge` creates a three-way
working view and returns `merge_base_unavailable` if the base is absent.
`backup` captures exact external bytes and leaves the alert pending so the
agent can decide the subsequent resolution. `preserve_external` captures exact external bytes before an overwrite/discard
using an exclusive atomic same-directory write to `<file>.back` by default or
an explicitly selected versioned path. Collision is `backup_exists`; write or
sync failure is `backup_failed`; neither commits the resolution.
For large tabs, `backup` performs the same capture by streaming the current
file to an atomic backup without materialising it in memory, and `reload`
reopens the file, records `external_reload`, and resets the lazy index. Large
`merge` and `force_save` still require an explicit bounded rewrite job because
the server does not hold the whole large document in memory.

`save_as` creates a new, non-existing target atomically and leaves the active
tab path unchanged; it refuses to overwrite an existing target. `close` is
explicit and terminal. Without `journal_action`, it returns
`journal_close_decision_required` with `preserve` and `clean` choices. Send
`journal_action: "preserve"` to retain recovery history, or `"clean"` to
delete the tab journal and SQLite metadata before the server exits.

## Streams and jobs

Paging/streaming readers restart after a write with the exact delimiter:

```text
===== FILE EDITED: RESTARTING =====
```

Non-streaming readers restart transparently. Jobs are queued, running,
completed, cancelled, failed, released, or evicted. Detached retention is ten
minutes by default, configurable up to one hour with dangerous acknowledgement;
release/eviction permanently invalidates resume tokens.
Each request still uses one short-lived connection, but the server may process
connections concurrently. A streamed text read is a revision snapshot: if a
write is committed before all its frames are sent, the server emits the
delimiter as a data frame and then sends a fresh stream plus a new `complete`
frame. Agents consuming raw protocol frames should treat the pre-delimiter
stream as superseded.

Job requests use `job_id` and, where stated, `resume_token`. `job-start`
creates a queued record and returns its opaque resume token. `job-progress`
accepts any JSON `progress` value; `job-complete` accepts a JSON array in
`result`; `job-poll` returns the current snapshot; `job-cancel` wins against a
not-yet-terminal completion; `job-transfer` requires the current token and
sets a new owner; and `job-release` permanently invalidates the token. These
operations manage server lifecycle metadata only: the driving agent owns the
actual work and must report truthful progress and terminal state.

`large_edit` is the explicit exception to the normal large-file mutation
refusal. It requires a current revision, `job_id`, and
`acknowledge_large_edit=true`; the server streams the byte-range rewrite through
a same-directory temporary file, syncs it, replaces the source, refreshes its
index, and completes the job. A failed rewrite marks the job `Failed`; the
client is responsible for polling and retaining or releasing the result.
Each committed large edit also retains file-backed before/after snapshots, so
`undo` and `redo` restore the file atomically without materialising it in RAM.
The snapshots can consume substantial disk space and are removed by the tab's
explicit `clean` close action.

The optional MCP bridge exposes the same operations as tools and publishes
`resources/list` entries for this protocol, the capabilities schema, and the
installed man page. Reading those resources is local and read-only; tool calls
still require the endpoint and retain the server's revision/authentication
rules.
