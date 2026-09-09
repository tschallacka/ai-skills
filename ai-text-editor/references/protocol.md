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

## Addressing a tab

A request says which tab it means in one of four ways, and they are tried in
this order:

1. **`tab_id`** — the handle every response reports. It is addressing enough on
   its own, for every method including the job methods: the session registry
   knows which endpoint serves that tab and which token authorizes it, so a
   caller holding an id needs no path, no cache and no identity. An id that
   names no tab is refused with `tab_unknown` and the open tabs listed; it
   never falls through to whichever tab discovery would have found instead,
   because a caller that named a tab did not ask for a different one.
2. **`tab_path`** — a filename, or a trailing run of path components, naming an
   open tab. The recovery for a caller that has lost the id. Matched on
   component boundaries, not as a substring, so `port.txt` does not name
   `report.txt`. Naming several tabs is refused with `tab_ambiguous` and a
   candidate list of `{tab_id, path}`; naming none with `tab_unmatched` and the
   open tabs. Neither is a guess and neither is a bare error — the answer to
   "which one did you mean" is the set, with the ids needed to say.
3. **`file`** — the path. See below: naming a file also opens it.
4. **Nothing at all** — the session's *focused* tab, which is the tab the last
   successful call under this identity was served by. `open` focuses the tab it
   opens, so "open, then work" means what it looks like. A refused call never
   moves the focus: the tab that answered a refusal is not the tab the caller
   meant. With no focus and nothing named, the request is refused with the four
   forms above named, rather than served by a tab nobody chose.

`tab_id` is not a weaker credential than the session token it stands for: it is
a BLAKE3 of that token and the server generation, so holding one is holding the
other. It is the stable, quotable half of the pair, and the half an agent can
carry in its own notes.

**Every response names the tab it answered**, in `tab_id`. That is the other
half of allowing an unmarked request at all: a caller that assumed the wrong
focus has to be able to see it in the answer rather than in the damage.

Naming a `file` on a request means "act on that file's tab", and opening it is
part of that. Every method does it, not only `open`: a client that discovers no
tab for the named file opens one and starts a server if none is running, then
serves the request. The single exception is the revision-guarded methods
(`insert`, `replace`, `large_edit`, `restore`, `undo`, `redo`, `save`) on a file
that has no tab at all — the revision they carry cannot have come from a tab
that never existed, so applying the edit against a guessable revision 0 is
refused by name. When a tab did exist and its server has since died, they do
start a replacement: the journal replays and the server's own `stale_revision`
check decides, by number, whether the revision they hold still means anything.

A path whose parent directory does not exist is refused with that directory
named; nothing is created. Retry with `acknowledge_create_parents` (CLI
`--acknowledge-create-parents`) to create the directory chain and open the tab.
The confirmation is deliberate: a silent recursive create would build a
directory tree out of a typo.

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
socket until `--takeover-stale-endpoint` is explicitly supplied; the old record
is renamed with a `stale-` suffix before replacement. The caller is responsible
for verifying the recorded owner is gone before taking over. Both clients carry
that confirmation through: the CLI as `--takeover-stale-endpoint` and the MCP
adapter as `takeover_stale_endpoint`, forwarded to the server this call starts.
Without it the refusal names the recorded pid and generation and nothing is
replaced.

A record whose path would be too long to hold a Unix socket beside it — the
`sun_path` limit — is written to a fallback directory instead, keyed to both
the runtime directory it stands in for and the user it belongs to. It used to
be one machine-global directory, which silently abandoned the isolation the
configured runtime directory expresses and left one 0700 directory shared
between every user on the host.

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

## Response verbosity

Every method takes `verbosity` 0-3, and **1 is the default**:

| level | what the payload carries |
|---|---|
| 0 | the method's own result and the `tab_id` that produced it, and nothing else |
| 1 | level 0 plus what verification and the next step need: `revision`, `dirty`, `disk_diverged`, `external_change_pending`, the tab's `mode`, the resolved edit span (`offset`, `delete_len`, `bytes_written`, `deleted`), `complete`/`eof`, `start_line`/`end_line`, `returned_bytes`, a search's `pager_key` and `count`, and a zero-result search's `note` |
| 2 | level 1 plus navigation: `cursors`, `total_bytes`, `start_byte`/`end_byte`, `result_id`, `limit`, index block paging, undo/redo depths |
| 3 | everything, exactly the payload before the ladder existed |

Three rules the levels do not bend:

1. **Every level carries the method's own result and names its tab.** A `read`
   returns its text at level 0; a search returns its matches. The ladder
   governs *metadata*. `tab_id` is level 0 because addressing the wrong tab
   silently is the failure the addressing design exists to prevent.
2. **A refusal is never trimmed.** An error frame keeps its `code`, `message`
   and recovery `choices` at every level, because for a refused request those
   *are* the answer.
3. **`capabilities` and `resources` ignore the level entirely**, their payload
   being metadata by definition.

A level outside 0-3 is refused with `verbosity_invalid` and a description of
the levels, rather than clamped — a typo must not silently buy a different
answer than the one asked for.

The default is 1 and not 0 deliberately, and it is a documented deviation from
the original specification. Level 0 cannot carry a `revision`, and every
mutation's guard requires one, so a level-0 default would silently break the
revision contract for any caller that then tried to edit. Level 0 stays
reachable explicitly, for a caller that wants status only and accepts that it
cannot mutate safely from that answer.

An argument no handler for a method reads is refused with `unknown_argument`,
naming the key, the method, and the keys that method does accept. The check is
per method: a key belonging to a *different* method is refused too, which it
was not before — one protocol-wide list let `replace` name `range_start_line`
because `read` takes that key, and the replace handler then dropped all four
range keys and edited at the cursor instead. The MCP adapter's advertised
`inputSchema` is built from the same declaration the server refuses against, so
a key one surface offers and the other rejects cannot exist.

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
5. A tab's mode is chosen per TAB, not per server: `open` takes
   `document_mode` (`text_utf8`, `raw_bytes`, `hex_view`) and applies it to the
   tab it opens, whether or not a workspace is already running. It used to
   shape only a newly started server, which — since every method autostarts and
   reconnects — meant an agent's very first `open` decided the mode of every
   tab it would ever open. A tab's mode is fixed for its lifetime, because its
   buffer, index and every coordinate committed to one reading of the bytes:
   reopening an existing tab under a different mode is refused with
   `document_mode_conflict`, and `close` then `open` is the way to change it.
   An unknown mode name is refused with `document_mode_invalid`. The `mode` a
   response reports uses these same names, so a caller can compare what it
   asked for against what it was given.
6. NFC is opt-in with the server's `--normalize-nfc` startup option.
   Coordinates continue to address stored bytes; normalized search results
   report positions in the normalized presentation. Mapping-preserving edits
   may restore original bytes; lossy edits return `restoration_conflict`.
   The `restore` operation explicitly disables normalized presentation when the
   mapping is still lossless; it returns `restoration_conflict` after a lossy
   normalized edit.
7. `wrap_width` is optional and reports visual coordinates in addition to the
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

### Addressing a span, and verifying it

`insert` places bytes at a point: `offset`, or the position of `cursor_id` when
`offset` is omitted. `replace` changes a span, and addresses it three ways —
exactly one of them per request:

1. `offset` plus `delete_len`, in bytes.
2. `range_start_line` and `range_end_line`: inclusive, one-based, whole lines.
   The last line's terminator is part of the span, so a `replace` with no text
   deletes those lines outright rather than leaving a blank one behind. Text
   tabs only; on a raw or hex tab lines are not the coordinate.
3. `range_start_byte` and `range_end_byte`: half-open — the same shape a search
   hit reports as `byte_start`/`byte_end`. A span across two hits is therefore
   the start of one and the end of the other, copied across, with no arithmetic
   and no question of whether a bound is inclusive.

Naming two of the three, or half of a pair, is refused with `edit_range_conflict`
or `edit_range_incomplete`; a range on `insert` is refused with
`edit_range_unsupported`. A range whose end precedes its start, or that runs
past the buffer, is `edit_range_invalid` and names the limit.

`replace` also takes `expected_text` (or `expected_bytes_base64` for bytes that
are not UTF-8): the bytes the caller believes are at the span. The server
verifies them **before** deleting anything and refuses `expected_text_mismatch`
otherwise, quoting both what was expected and what is actually there. When
`expected_text` is the only thing naming a length, its own length is the length,
so `delete_len` need not be computed at all; supplying both with different
lengths is `expected_text_length_mismatch`.

An `insert` deletes nothing, so its span is empty and any `expected_text`
could only ever mismatch; it is refused with `expected_text_unsupported`
rather than left to fail confusingly.

`replace` also takes `match_id` — a search hit's own id
(`<result_id>#<index>`, carried on every match a `search` returns) — as a
fourth way to address the span, instead of a line range, a byte range, or a
recomputed offset. It resolves to the exact `byte_start`/`byte_end` the search
recorded, and carries its own content guard: the bytes at that span must
still equal what the search found there, or the request is refused as
`match_id_stale`, the same way an edit since the search clears the whole
result set the id names (looked up and refused as `match_id_stale` too, for
the same reason). `match_id` takes no `range_*` key, `offset`, `cursor_id`, or
`expected_text` — naming one alongside it is `edit_range_conflict`, since it
already carries the guard `expected_text` would add. An `insert` refuses
`match_id` the same way it refuses a range: `edit_range_unsupported`, because
a match is a span and `insert` places bytes at a point. An id shaped wrong, or
naming an index past the result set, is `match_id_invalid`.

`search` takes `preview_lines` (an integer, `0` — the default — meaning
unshrunk): when a match's `contents`/`contents_base64` spans more than
`preview_lines * 2` lines, the response replaces it with `contents_preview`
— `{head, tail, lines, omitted_lines, omitted_bytes, truncated: true}`,
`head`/`tail` each holding the match's first/last `preview_lines` lines as
`{text}` or `{base64}`. `byte_start`/`byte_end`, and so `match_id`, are
unaffected — a `replace` addressed by a truncated match's `match_id` still
edits the real, complete span. A match short enough not to need shrinking
keeps its full `contents`/`contents_base64` unchanged, so a caller need not
branch on which field a small hit carries.

This is not the revision guard, and it catches what the revision guard cannot.
A revision proves the *document* has not moved since the caller last read it. It
says nothing about whether `delete_len` still matches the text at `offset` — so
a caller whose own earlier mutation changed the length of the very text it is
addressing holds a perfectly current revision and is still wrong. That is how a
`replace` deleted 35 bytes of a 36-byte token, left the orphan digit, and
reported success.

Neither guard is optional and neither is the default in every case: the
revision guard is required on every revision-guarded method regardless
(`revision_required_methods` above), and `expected_text` is additive on top of
it. The preference is which span-correctness guard a caller should reach for.
The revision guard alone is enough when both endpoints came from a `read` at
that same revision — the property it proves is exactly the property that
matters there, and `expected_text` would only re-send content the server
already has, at output-token cost. Add `expected_text` when an endpoint did
not come from such a read: carried across the caller's own edits, taken from
an older revision's search hit, or computed by arithmetic. A caller unsure
which case it is in should re-`read` the span rather than guess — that costs
input tokens, the cheaper side of the same trade.

Every applied `insert`/`replace` reports the `offset` and `delete_len` it
resolved, `bytes_written`, and — when it deleted anything — `deleted`, the bytes that
went. `deleted.bytes` is always the span's full length and `deleted.truncated`
says whether anything was left out of the quoted content, which is capped at
256 bytes and carried as `text` when it is UTF-8 and `base64` when it is not.
A caller can therefore verify an edit from its own answer instead of reading
the file back.

The server intentionally has no project-root jail. A valid session token grants
the server user the same path access as the server process. Keep endpoints
private and use OS-level isolation when a client must not access other files.

Search mode is required and must be one of `exact_text`, `exact_bytes`,
wildcard, `shell_wildcard`, `path_wildcard`, `regex_rust`, `regex_pcre2`,
`fuzzy_edit`, `fuzzy_subsequence`, `fuzzy_token`, `fuzzy_ngram`,
`fuzzy_phonetic`, or `fuzzy_soundex`. Results include line/column coordinates,
matched contents, a revision-bound result identifier, count, pager key,
completeness, and the first four matches when no limit is supplied. Text-mode
results also carry absolute `byte_start`/`byte_end` offsets into the document —
the coordinates `insert` and `replace` consume, so a hit is editable without
manual line/column-to-byte arithmetic. `exact_bytes` decodes its query as
base64-encoded bytes whether it arrives in `query` or `query_base64`; an
undecodable `query` is refused by `invalid_base64` naming the rule.

**Every text mode matches within one line.** The document is split on newlines
and each line is matched without its terminator, so a query containing a newline
can never match — that is a property of the scope, not of the query. Such a
query is refused with `search_query_crosses_lines` rather than answered with a
zero, because a zero there is indistinguishable from "the text is not in this
file" and an agent acting on it concludes an anchor is absent and edits
elsewhere. `exact_bytes` is the mode that spans lines: it matches its decoded
bytes, newlines included, against the whole buffer. A regular expression that
would match a newline (`\n`, `[\s\S]`) is subject to the same line scope even
though its query text carries no newline byte for the refusal to catch.

A text search that finds nothing and whose query contains HTML entities
(`&lt;` `&gt;` `&amp;` `&quot;` `&apos;` `&#39;` `&nbsp;`) carries a `note` and
`unescaped_query_matches` when the same query unescaped *does* match — the count
it would have found. This fires only on an actual match, so a genuine absence
answers a plain zero and is never explained away.

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

## Tab state: dirty, disk_diverged, external_change_pending

Every reading and mutating response carries these three, and callers must
branch on them. They answer three different questions and no two of them are
the same fact:

- **`dirty`** — the tab's buffer differs from what this tab last wrote to or
  read from the file. It is *your own unsaved work*: it becomes true on an edit
  and false again on a successful `save`. It is compared against the tab's
  saved digest, never against the file, so an external change cannot make it
  true. A large tab reports `false`, since it holds no whole buffer to compare.
- **`disk_diverged`** — the file on disk differs from what this tab last synced
  with. That is *someone else's* change: another process wrote the file under
  the tab. The bytes compared are read from the file, never taken from the
  buffer — hashing the buffer here made every unsaved edit read as an external
  change, which is the one thing this field exists to keep separate from
  `dirty`.
- **`external_change_pending`** — a divergence has been *observed and not yet
  resolved*. Observing one arms the guard: mutating methods are then refused
  with `external_change` until `resolve_external` chooses `reload`, `merge`,
  `keep`, `backup`, or an acknowledged `force_save`. Reads keep answering from
  the editor's buffer throughout.

The four combinations are all reachable and all mean something. Both false is a
clean tab. `dirty` alone is ordinary unsaved work. `disk_diverged` alone is a
file that moved under a tab you have not edited — a `reload` is lossless.
`dirty` and `disk_diverged` together is the conflict: two sets of changes, and
`merge` or an explicit choice is required.

The observation happens at the start of every request except `resolve_external`
and `save_as`, before the handler runs — so the very call that first notices a
divergence is the one that reports `external_change_pending: true`, and a
mutating call is refused in that same answer rather than one later. Once armed,
the flag stays armed until `resolve_external` clears it; a second external write
while a resolution is pending does not re-arm it, because it is already armed.

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
