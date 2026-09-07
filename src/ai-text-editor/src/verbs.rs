// MODE: DEV
// PACKAGE: PROD
//! The payload keys each verb reads — one declaration that is both the
//! validator the server's door consults and the schema the MCP adapter
//! advertises.
//!
//! B180 and B187 set out to guarantee that an argument no handler reads is
//! refused by name rather than silently ignored, and the check they produced
//! was one list for the whole protocol: `replace` naming `range_start_line`
//! passed the door because *`read`* takes that key, and the replace handler
//! then dropped all four range keys and edited at the cursor instead — a
//! silent misplaced edit reported as a success (B237). Per-verb key sets did
//! exist, but only inside the MCP adapter's tool schemas, which the CLI and
//! hand-sent NDJSON never consult.
//!
//! The fix is not a fifth patch to a list. It is that the router and the
//! validator are the same declaration: the server refuses anything outside
//! `payload_keys(method)`, and the adapter builds each tool's `inputSchema`
//! from the same function, so a key one of them knows about and the other does
//! not cannot exist. `ADAPTER_ARGUMENTS` did this for the adapter-consumed
//! arguments in B217; this is the same trick for the server-consumed ones.
//!
//! There are two tiers, and the split matters on both surfaces.
//!
//! [`extra_payload_keys`] is what a verb *consumes*: the keys that do
//! something, and the only ones the MCP adapter advertises. A key does NOT
//! belong here merely because a sibling verb reads it — `insert` does not take
//! `delete_len`, which its handler reads for `replace` and ignores for
//! `insert`, so an insert stating "delete five bytes first" was performed as a
//! plain insert and answered as a success. That is the class this module
//! closes, not an exception to it.
//!
//! [`refused_payload_keys`] is what a verb *refuses by name itself*, with a
//! message better than the door's. `search` takes `offset` only to answer
//! `search_offset_unsupported` ("page an existing result set instead"), and
//! `insert` takes the four range keys only so `edit_span` can answer "a range
//! is a span and insert places bytes at a point". They pass the door so the
//! diagnosis survives, and they are deliberately *not* advertised: a schema
//! offering an argument whose only outcome is a refusal is worse than one that
//! does not mention it.

/// Keys every verb accepts.
///
/// The three addressing keys are universal because addressing is: T96 makes
/// `tab_id` sufficient on its own for every verb including the job verbs, T97
/// makes `tab_path` the recovery for a caller that lost the id, and `file` is
/// the original spelling. `presentation` is here because the CLI puts it on
/// every payload it builds, and `verbosity` because the response ladder
/// applies to every answer (T99).
pub const UNIVERSAL_KEYS: &[&str] = &["file", "tab_id", "tab_path", "presentation", "verbosity"];

/// Every method the server dispatches, in the order `handle` matches them.
/// Used by the tests that prove this table and the dispatch cover the same
/// set — a verb added to one and not the other is exactly how a per-verb list
/// goes stale.
pub const METHODS: &[&str] = &[
    "open",
    "capabilities",
    "resources",
    "history",
    "begin_transaction",
    "end_transaction",
    "restore",
    "index",
    "cursor",
    "page",
    "read",
    "replace",
    "insert",
    "large_edit",
    "undo",
    "redo",
    "save",
    "save_as",
    "close",
    "resolve_external",
    "search",
    "job_start",
    "job_poll",
    "job_progress",
    "job_complete",
    "job_cancel",
    "job_transfer",
    "job_release",
];

/// The payload keys `method` consumes, beyond [`UNIVERSAL_KEYS`] — the set the
/// MCP adapter advertises. `None` means the method is not one this server
/// dispatches at all, which is a different refusal (`unknown_method`) and must
/// not be reported as a bad argument.
pub fn extra_payload_keys(method: &str) -> Option<&'static [&'static str]> {
    let keys: &[&str] = match method {
        // B238: `document_mode` shapes the TAB this open creates, not the
        // server hosting it, so it has to reach the server in the payload and
        // not only as the startup argv of a server this call happens to
        // start. It is also an adapter argument (a cold start still needs it
        // on the command line), which is why `call_tool` keeps it in the
        // payload for this one verb after stripping it for every other.
        "open" => &["document_mode"],
        "capabilities" | "resources" | "history" => &[],
        "begin_transaction" | "end_transaction" | "restore" | "undo" | "redo" | "save" => &[],
        "read" => &[
            "cursor_id",
            "before",
            "after",
            "line",
            "offset",
            "length",
            "range_start_line",
            "range_end_line",
            "range_start_byte",
            "range_end_byte",
        ],
        "insert" => &["offset", "cursor_id", "text", "bytes_base64"],
        "replace" => &[
            "offset",
            "cursor_id",
            "delete_len",
            "text",
            "bytes_base64",
            "range_start_line",
            "range_end_line",
            "range_start_byte",
            "range_end_byte",
            "expected_text",
            "expected_bytes_base64",
        ],
        "large_edit" => &[
            "job_id",
            "resume_token",
            "acknowledge_large_edit",
            "offset",
            "delete_len",
            "text",
            "bytes_base64",
        ],
        "save_as" => &["target_path"],
        "close" => &["journal_action"],
        "resolve_external" => &[
            "action",
            "backup_path",
            "preserve_external",
            "acknowledge_force_save",
        ],
        "index" => &["granularity", "offset", "limit"],
        "cursor" => &[
            "action",
            "id",
            "line",
            "column",
            "page_lines",
            "wrap_width",
            "visual",
        ],
        "page" => &["pager_key", "offset", "limit", "historical"],
        "search" => &[
            "mode",
            "query",
            "query_base64",
            "limit",
            "order",
            "gradient",
            "range_start_line",
            "range_end_line",
            "range_start_byte",
            "range_end_byte",
        ],
        "job_start" => &["owner", "detached"],
        "job_poll" | "job_cancel" | "job_release" => &["job_id", "resume_token"],
        "job_progress" => &["job_id", "resume_token", "progress"],
        "job_complete" => &["job_id", "resume_token", "result"],
        "job_transfer" => &["job_id", "resume_token", "owner"],
        _ => return None,
    };
    Some(keys)
}

/// The payload keys `method` accepts only in order to refuse them by name
/// itself, with a message the door cannot give. Accepted at the door,
/// deliberately never advertised — see this module's own documentation.
pub fn refused_payload_keys(method: &str) -> &'static [&'static str] {
    match method {
        // "historical belongs to search and page; a read always returns the
        // live buffer."
        "read" => &["historical"],
        // "search always scans the whole (bounded) document; page an existing
        // result set with `page --pager-key <key> --offset N` instead."
        "search" => &["offset"],
        // Two named refusals, both already in the handler:
        //  - `edit_span`: "a range is a span and insert places bytes at a
        //    point; `replace` takes the range (with no text, it deletes it)".
        //  - `expected_span_bytes`: "expected_text verifies the bytes a span
        //    replaces and insert deletes nothing" (`expected_text_unsupported`).
        "insert" => &[
            "range_start_line",
            "range_end_line",
            "range_start_byte",
            "range_end_byte",
            "expected_text",
            "expected_bytes_base64",
        ],
        _ => &[],
    }
}

/// Whether `method` reads `key` at all — universal keys included, and the
/// refused tier counted as read so the handler's own message gets the chance
/// to be the answer.
///
/// Says nothing about whether `method` exists: a universal key is universal.
/// "Is this a method at all" is [`extra_payload_keys`] answering `None`, and
/// the door must ask that first — an unknown method is `unknown_method`, never
/// a bad argument.
pub fn reads_payload_key(method: &str, key: &str) -> bool {
    if UNIVERSAL_KEYS.contains(&key) {
        return true;
    }
    if refused_payload_keys(method).contains(&key) {
        return true;
    }
    extra_payload_keys(method).is_some_and(|keys| keys.contains(&key))
}

/// Every payload key `method` accepts, universal keys first, for a refusal
/// that tells the caller what it could have sent instead. `None` for a method
/// this server does not dispatch.
pub fn payload_keys(method: &str) -> Option<Vec<&'static str>> {
    let extra = extra_payload_keys(method)?;
    let mut keys: Vec<&'static str> = UNIVERSAL_KEYS.to_vec();
    keys.extend_from_slice(extra);
    keys.extend_from_slice(refused_payload_keys(method));
    keys.sort_unstable();
    Some(keys)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_dispatched_method_declares_its_keys() {
        for method in METHODS {
            assert!(
                extra_payload_keys(method).is_some(),
                "{method} is dispatched and declares no key set"
            );
        }
    }

    #[test]
    fn an_undispatched_method_is_not_an_argument_error() {
        assert!(extra_payload_keys("teleport").is_none());
        assert!(payload_keys("teleport").is_none());
        // Deliberately NOT asserted the other way round: `reads_payload_key`
        // is about keys, not about methods, so a universal key answers true
        // for any spelling. The door asks `payload_keys` first, which is what
        // keeps an unknown method out of the argument-refusal path.
        assert!(!reads_payload_key("teleport", "delete_len"));
    }

    /// The defect itself, at table level: `range_start_line` is a `read` and
    /// `replace` key, and `insert` must not inherit it as one it consumes.
    /// (That the *door* consults this is what cli_flow pins; this only pins
    /// that the table says the right thing.)
    #[test]
    fn a_key_one_verb_reads_is_not_a_key_every_verb_reads() {
        assert!(reads_payload_key("read", "range_start_line"));
        assert!(reads_payload_key("replace", "range_start_line"));
        assert!(!reads_payload_key("history", "range_start_line"));
        assert!(!reads_payload_key("save", "offset"));
        assert!(!reads_payload_key("insert", "delete_len"));
        // In the refused tier, not the consumed one: the handler answers
        // `expected_text_unsupported`, which beats the door's message.
        assert!(reads_payload_key("insert", "expected_text"));
        assert!(!reads_payload_key("search", "pager_key"));
    }

    #[test]
    fn universal_keys_are_legal_on_every_verb() {
        for method in METHODS {
            for key in UNIVERSAL_KEYS {
                assert!(
                    reads_payload_key(method, key),
                    "{method} refuses the universal key {key}"
                );
            }
        }
    }
}
