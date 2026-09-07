// MODE: DEV
// PACKAGE: PROD
//! The response verbosity ladder (T99).
//!
//! Michael, on the bus: "the json response given back is very very verbose.
//! Is there a way to limit it to the essential feedback if any, with a flag in
//! the mcp/binary for verbosity? no flag: just status good, status bad, with
//! option to get more data. with verbosity 1, just some data useful for
//! verification/next step. with verbosity 2, more data useful for navigating,
//! verbosity 3, all data as it is now" — and "editing two lines, getting 15
//! lines or more json back seems wasteful of tokens".
//!
//! Measured before this existed: `open` on a TWO-LINE file was 1199 bytes over
//! 47 lines, carrying a four-field resources block, a 64-character
//! server_generation and a session token on every call; a one-word `insert` was
//! 378 bytes over 25 lines whose entire actionable content was `revision` and
//! `dirty`. The cost lands hardest on the MCP surface, where every response is
//! context an agent pays for on every edit — so response shape is a token
//! budget, not a formatting preference.
//!
//! ## The levels
//!
//! - **0** — the answer's content and the tab it came from. No metadata. The
//!   frame type is the good/bad status, so for a mutation this is `tab_id`
//!   alone.
//! - **1, the default** — level 0 plus what verification and the next step
//!   need: `revision`, `dirty`, the tab's `mode`, and the resolved edit span
//!   (`offset`, `delete_len`, `bytes_written`, `deleted`).
//! - **2** — level 1 plus what navigation needs: cursors, coordinates,
//!   completeness and paging keys.
//! - **3** — everything, exactly the payload before this existed, so nothing
//!   regresses for a caller that wants it all.
//!
//! ## Two deviations from the specification, both deliberate
//!
//! **The default is 1, not 0.** Level 0 cannot carry a revision, and every
//! mutation's guard requires one (SKILL.md agent responsibility 5). A default
//! that silently breaks the revision contract is worse than a verbose one, so
//! level 0 is reachable explicitly — for a caller that wants status only and
//! accepts that it cannot then mutate safely — and 1 is the floor everything
//! else lands on.
//!
//! **Every level carries the verb's primary result.** A `read` whose text was
//! a level-2 luxury would make level 0 and 1 useless rather than terse; the
//! ladder governs *metadata*. `capabilities` and `resources` are exempt
//! altogether: their payload is entirely metadata because metadata is what
//! they are for. So is every error frame — a refusal's code, message and
//! recovery choices are the answer, at every level.
//!
//! ## Why a per-key table and not a per-verb one
//!
//! Because B237 is what a per-verb list of keys does when it goes stale, and
//! this one would be twenty-eight lists instead of one. A key means the same
//! thing wherever it appears — `revision` is verification everywhere,
//! `cursors` is navigation everywhere — so the tier belongs to the key.
//!
//! An unclassified key is kept at **every** level, which is the safe way round:
//! a new field that is merely more verbose than intended costs tokens, while
//! one that silently vanishes at the default level is a correctness bug in
//! whatever depended on it. `debug_assert` makes it loud instead of permanent —
//! the test suite runs in debug, so a key nobody classified fails the suite
//! rather than quietly shipping.

use serde_json::{Map, Value};

/// The default level. Deliberately 1 rather than the specification's 0; see
/// this module's own documentation for why.
pub const DEFAULT_VERBOSITY: u8 = 1;
pub const MAX_VERBOSITY: u8 = 3;

/// The verbs whose entire payload is the answer, so the ladder does not apply.
pub fn verb_is_exempt(method: &str) -> bool {
    matches!(method, "capabilities" | "resources")
}

/// The lowest level at which `key` is reported, or `None` for a key nobody has
/// classified.
///
/// Tier 0 is the verb's own result and the tab that produced it; tier 1 is
/// verification and the next step; tier 2 is navigation; tier 3 is everything
/// else, which is to say diagnostics and identity a caller needs only when it
/// asks for all of it.
fn declared_tier(key: &str) -> Option<u8> {
    let tier = match key {
        // ---- 0: the answer itself, and which tab answered ----------------
        // T98: the tab is named at every level, because addressing the wrong
        // tab silently is the failure that design prevents.
        "tab_id" => 0,
        // The content of a read, a search, an index inspection, a page.
        "text" | "bytes_base64" | "matches" | "blocks" | "lines" => 0,
        // Terminal facts of a verb that has no other content: without these
        // the answer is empty rather than terse.
        "saved" | "closed" | "restored" | "resolved" | "transaction" | "job" | "id" => 0,
        // A stream's own framing. Dropping these would break the reader's
        // ability to reassemble the document at all.
        "stream" | "restart" => 0,
        // B258: the `cursor` verb's own answer. A cursor position is what the
        // verb is FOR, so it is content and not metadata about content — the
        // same reason a read's `text` is tier 0. Leaving these unclassified is
        // what fired the debug_assert, and the symptom was the connection
        // thread dying with no frame written: "the server closed the
        // connection without answering", which names nothing a caller could
        // act on.
        "line" | "column" => 0,

        // ---- 1: verification, and the next step --------------------------
        "revision" | "dirty" => 1,
        // B238: with mode a per-tab property a caller chooses, a response that
        // does not name it leaves the caller unsure which coordinate space it
        // is in — bytes or 16-byte hex rows.
        "mode" => 1,
        // B226 and B230: added precisely because a caller could not see what
        // its arithmetic had addressed. That makes them verification-grade,
        // not decoration — the verify-read after every edit existed because
        // these were missing.
        "offset" | "delete_len" | "bytes_written" | "deleted" => 1,
        // A delete that crossed a line end changed more than the caller may
        // have meant; that belongs with the span it applies to.
        "spans_lines" => 1,
        // SKILL.md states the contract this level has to satisfy: "every
        // mutating and reading response carries three state flags you must
        // branch on" — dirty, disk_diverged, external_change_pending. A
        // default that dropped any of the three would break an agent
        // responsibility, not merely shorten an answer.
        "disk_diverged" | "external_change_pending" => 1,
        "resolution_pending" | "save_required" => 1,
        // Agent responsibility 2: "treat revisions, result generations,
        // completeness, and stale-page errors as authoritative". Completeness
        // and result generation are named there, so they are verification and
        // not navigation, however much they look like paging furniture.
        // `eof` is `complete` for a byte window rather than a line window.
        "complete" | "eof" | "generation" | "source_revision" | "stale" => 1,
        // The handle to the rest of the answer, and the number that says
        // whether there is a rest. A search reporting four of forty matches
        // without its pager key is a dead end, not a terse success — this is
        // "what the next step needs" as literally as the level gets, and the
        // same argument that puts `tab_id` at level 0.
        "pager_key" | "count" => 1,
        // What a bounded read actually returned, as against what was asked
        // for. A read whose answer does not say which lines or how many bytes
        // it is cannot be verified against the request at all. `total_bytes`
        // is a different question - how much more there is - and stays at the
        // navigation level.
        "start_line" | "end_line" | "returned_bytes" => 1,
        // The granularity an index was built at, echoed back: verification
        // that the argument took effect, which is the whole reason B237
        // exists as a class.
        "granularity" => 1,
        // When a search finds nothing, this IS the answer. B218 is the cost
        // of not having it: an agent whose query found zero concluded the
        // text was absent and edited elsewhere, when the text was there and
        // its own query was HTML-escaped. A caller that sees only count 0 is
        // one step from that mistake, so the note that says "the unescaped
        // form matches once" is what the next step needs, not decoration.
        "note" | "unescaped_query_matches" => 1,
        // A recovered tab's replayed revision must never read as fresh work
        // (B196), which is a verification fact, not a diagnostic.
        "journal_replay" => 1,
        // Reconnection identity. `open` seeds a session, and a client that
        // cannot reconnect is worse off than one that got a long answer.
        "session_token" => 1,
        // Whether this tab is a bounded large tab decides which verbs are
        // even available on it.
        "large_file" => 1,
        // The path the answer is about: cheap, and the one field that makes a
        // response readable without holding the request beside it.
        "path" => 1,
        "changed" => 1,

        // ---- 2: navigation -----------------------------------------------
        "cursors" | "cursor" => 2,
        // The wrapped presentation of the same position: asked for with
        // wrap_width, and needed only when navigating a wrapped view.
        "visual" | "wrap_width" | "range" => 2,
        "start_byte" | "end_byte" => 2,
        "total_bytes" | "bytes" => 2,
        "result_id" | "returned" | "limit" => 2,
        "block_offset" | "block_count" | "returned_blocks" | "coverage" => 2,
        "index_loaded" | "index_complete" | "index_coverage" => 2,
        "search_range" | "order" => 2,
        "undo_depth" | "redo_depth" | "journal_sequence" => 2,

        // ---- 3: diagnostics, identity, and the rest ----------------------
        "server_generation" | "server_pid" => 3,
        "resources" => 3,
        "normalize_nfc" => 3,
        "text_undo_depth" | "text_redo_depth" | "large_undo_depth" | "large_redo_depth" => 3,
        "journal" | "journal_action" | "active_path" | "backup_path" => 3,
        "unsaved_changes" | "history_event" | "edits" | "through_sequence" => 3,
        _ => return None,
    };
    Some(tier)
}

/// The lowest level at which `key` is reported. An unclassified key is
/// reported at every level, and fails a debug build so the table cannot go
/// stale in silence.
pub fn tier(key: &str) -> u8 {
    match declared_tier(key) {
        Some(tier) => tier,
        None => {
            debug_assert!(
                false,
                "response key `{key}` has no verbosity tier; classify it in \
                 ai_text_editor::verbosity::declared_tier"
            );
            0
        }
    }
}

/// The level a request asked for, or the default. A value outside 0..=3 is a
/// caller error rather than something to clamp silently — `None` here means
/// "refuse it by name", which is what the door does with it.
pub fn requested(payload: &Value) -> Option<u8> {
    match payload.get("verbosity") {
        None => Some(DEFAULT_VERBOSITY),
        Some(Value::Number(number)) => number
            .as_u64()
            .filter(|level| *level <= MAX_VERBOSITY as u64)
            .map(|level| level as u8),
        // The MCP schema types this as an integer, but a client that sends
        // "1" is following the same convention `expected_revision` already
        // tolerates (B175), and refusing it would be a distinction without a
        // difference to the caller.
        Some(Value::String(text)) => text
            .trim()
            .parse::<u8>()
            .ok()
            .filter(|level| *level <= MAX_VERBOSITY),
        Some(_) => None,
    }
}

/// Drop every key above `level` from one response payload.
///
/// `verbosity` itself never appears in an answer: it is the request's own
/// argument, and echoing it back would be one of the bytes this exists to
/// save.
pub fn apply(payload: &mut Map<String, Value>, level: u8) {
    if level >= MAX_VERBOSITY {
        return;
    }
    payload.retain(|key, _| tier(key) <= level);
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn the_default_is_one_and_carries_the_revision_guard() {
        assert_eq!(DEFAULT_VERBOSITY, 1);
        // The reason for the deviation, pinned so it cannot be "tidied" back
        // to the specification's 0 without this failing: a mutation's guard
        // needs the revision its answer reports.
        assert!(tier("revision") <= DEFAULT_VERBOSITY);
        assert!(tier("dirty") <= DEFAULT_VERBOSITY);
        assert!(tier("mode") <= DEFAULT_VERBOSITY);
        for key in ["offset", "delete_len", "bytes_written", "deleted"] {
            assert!(
                tier(key) <= DEFAULT_VERBOSITY,
                "{key} is what B226/B230 added so a caller could see what its \
                 arithmetic addressed; it belongs at the default level"
            );
        }
    }

    #[test]
    fn the_tab_is_named_at_every_level() {
        assert_eq!(tier("tab_id"), 0);
    }

    #[test]
    fn a_verbs_own_result_survives_the_lowest_level() {
        for key in ["text", "bytes_base64", "matches", "blocks", "saved"] {
            assert_eq!(tier(key), 0, "{key} is an answer, not metadata");
        }
    }

    #[test]
    fn level_three_is_exactly_todays_payload() {
        let mut payload = json!({"revision": 1, "resources": {"a": 1}, "cursors": {}})
            .as_object()
            .unwrap()
            .clone();
        let before = payload.clone();
        apply(&mut payload, 3);
        assert_eq!(payload, before);
    }

    #[test]
    fn a_lower_level_drops_only_what_is_above_it() {
        let full = json!({
            "tab_id": "abc", "text": "x", "revision": 2, "dirty": true,
            "cursors": {"0": {"line": 1, "column": 0}}, "resources": {"a": 1},
            "server_generation": "deadbeef"
        });
        let object = full.as_object().unwrap();

        let mut level0 = object.clone();
        apply(&mut level0, 0);
        assert_eq!(level0.keys().len(), 2, "level 0: {level0:?}");
        assert!(level0.contains_key("tab_id") && level0.contains_key("text"));

        let mut level1 = object.clone();
        apply(&mut level1, 1);
        assert!(level1.contains_key("revision") && level1.contains_key("dirty"));
        assert!(!level1.contains_key("cursors"), "navigation is level 2");
        assert!(!level1.contains_key("resources"), "level 3 only");

        let mut level2 = object.clone();
        apply(&mut level2, 2);
        assert!(level2.contains_key("cursors"));
        assert!(!level2.contains_key("server_generation"));
        assert!(!level2.contains_key("resources"));
    }

    #[test]
    fn an_out_of_range_level_is_not_silently_clamped() {
        assert_eq!(requested(&json!({})), Some(DEFAULT_VERBOSITY));
        assert_eq!(requested(&json!({"verbosity": 0})), Some(0));
        assert_eq!(requested(&json!({"verbosity": 3})), Some(3));
        assert_eq!(requested(&json!({"verbosity": "2"})), Some(2));
        for bad in [json!(4), json!(-1), json!(1.5), json!("loud"), json!(true)] {
            assert_eq!(
                requested(&json!({"verbosity": bad})),
                None,
                "{bad} must be refused, not clamped"
            );
        }
    }

    #[test]
    fn discovery_verbs_are_exempt() {
        assert!(verb_is_exempt("capabilities"));
        assert!(verb_is_exempt("resources"));
        assert!(!verb_is_exempt("read"));
        assert!(!verb_is_exempt("open"));
    }
}
