<!-- MODE: PROD -->
# ai-text-editor

The editor skill is an agent-facing entry point for the server and short-lived
client. Read `SKILL.md` first, then `../ai-text-editor.1` and
`../references/protocol.md` for the complete command and wire contracts.

The server owns file state, journals, indexes, SQLite metadata, and session
state. Clients request operations and terminate after the response. Unix
sockets are preferred on Unix; loopback TCP is the fallback for Windows and
explicit transport selection. TCP startup requires `--auth-token`; each
connection uses a fresh HMAC challenge/proof and does not place the secret in
the editor request. `--auth-token-file` is also supported for owner-only
credentials and rotation. The endpoint must remain private to the host.
An `open` response returns a tab-scoped `session_token`; persist it with
`--save-session-token` and reuse it for all later requests. Session tokens are
distinct from TCP credentials and expire when the owning server instance ends.
Unix endpoint discovery records are atomic and include the active server PID
and generation. Graceful shutdown removes the active record. If a process
crashes, the socket and record remain as stale diagnostics; a new server
refuses to remove them unless `--takeover-stale-endpoint` is explicitly given.
