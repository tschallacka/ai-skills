# ai-text-editor protocol reference

The authoritative transport is versioned NDJSON: one request per short-lived
connection and zero or more ordered response frames ending in `complete`.
Structured output is the default; text, paging, and streaming are explicit
client presentation choices.

Unix sockets are preferred. Loopback TCP is the Windows fallback; the server
requires `--auth-token` and performs a per-connection challenge/proof exchange.
The client sends the secret only to its local process: the TCP request after
authentication does not contain `auth_token`. The fallback must never be bound
publicly.

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

`open` returns a server-issued `session_token` and `server_generation`. The
client may persist them with `--save-session-token PATH`; subsequent requests
must send that session token, either by using `--session-token PATH` or by
providing it to an equivalent protocol client. The token authorizes one tab,
is invalidated when that server instance closes, and is never a substitute for
the TCP authentication secret. A request without a valid token fails with
`session_unauthorized` before it can mutate, journal, or create a result.

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
bounds. For a large tab, `exact_text` currently requires an explicit inclusive
`range_start_line`/`range_end_line`; an omitted end line is refused rather than
silently scanning an unbounded file. Large `exact_bytes` searches instead
require an explicit half-open `range_start_byte`/`range_end_byte` range no
larger than the bounded read limit and return absolute `byte_start`/
`byte_end` coordinates with base64 contents.

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
