// MODE: DEV
// PACKAGE: PROD
//! The TLS chat client, as a library plus its CLI entry point (`run`).
//!
//! The plumbing every front end needs is public: discovery and the resolution
//! ladder, the TLS connect with its TOFU pin, registration, the line I/O, and
//! the per-agent session with its channel cursors. The CLI verbs are private —
//! they are one front end's argument handling, not the client's interface.
//! `chat-mcp` is the second front end (T90).
//!
//! Finds a chat server (UDP announce beacon), connects over TLS, pins the
//! server certificate on first connect (TOFU), and then either sends a message,
//! reads a delta since an id (via the additive FETCH extension), or tails a
//! channel. Speaks the same RFC-grammar wire format as the server (shared via
//! chat-proto).
//!
//! Cert pinning: the server certificate's DER fingerprint is stored under the
//! client's state dir keyed by host:port. The first connect records it; later
//! connects require an exact match (fail closed). `--insecure` bypasses this
//! for testing.

/// The tail-owned control socket, over which the other verbs borrow the one
/// connection instead of opening a second one under the same nick (T107).
pub mod control;

mod cli;
/// Challenge/proof primitives `control.rs`'s loopback-TCP arm uses (T111).
mod control_auth;
mod discovery;
mod local;
mod net;
mod session;
mod wire;

pub use cli::{read_last_id, run};
pub use discovery::{discover_candidates, resolve_server, DEFAULT_BEACON_PORT};
pub use local::{channels_home, local_chan_log, local_last_id};
pub use net::{connect, read_line, server_host, wait_for_welcome, write_line, Client};
pub use session::{
    apply_session, apply_session_with_key, chat_default_home, client_state_dir,
    resolve_session_key, save_cursor, save_cursor_with_key, save_session, save_session_with_key,
    session_key, KeySource, Session,
};
pub use wire::{json_field, mentions, msg_line_id, valid_chan, wire_segments};

// Test-only reach: none of these are part of the crate's public surface (see
// the `pub use` block above for that) and nothing outside the test module
// below calls them at this root path -- `cli`, `net` and `session` reach the
// couple of pub(crate) items they need directly through each other's module
// path instead. The in-file test module needs them exactly as it always did,
// via its own `use super::*;`, and only under `#[cfg(test)]` is anything here
// actually used.
#[cfg(test)]
pub(crate) use cli::parse_opts;
#[cfg(test)]
pub(crate) use local::local_read;
#[cfg(test)]
pub(crate) use net::server_name;
#[cfg(test)]
use rustls_pki_types::ServerName;
#[cfg(test)]
pub(crate) use session::sibling_session_in;
#[cfg(test)]
use std::path::PathBuf;

#[cfg(test)]
mod tests {

    /// B266: a newline used to end the IRC line, so everything after the first
    /// paragraph break was parsed by the server as a command and discarded.
    #[test]
    fn a_multiline_text_becomes_one_segment_per_line() {
        let segments = wire_segments("editor-batch", "#ai-skills", "first\n\nsecond\nthird");
        assert_eq!(segments, vec!["first", " ", "second", "third"]);
        for segment in &segments {
            assert!(
                !segment.contains('\n') && !segment.contains('\r'),
                "a segment may not carry a line terminator: {segment:?}"
            );
        }
    }

    /// Splitting on newlines alone would have replaced one silent truncation
    /// with another: a single paragraph can exceed 512 bytes by itself.
    #[test]
    fn a_long_paragraph_is_split_to_fit_the_irc_line_limit() {
        let nick = "editor-batch";
        let chan = "#ai-skills";
        let text = "word ".repeat(500);
        let segments = wire_segments(nick, chan, &text);
        assert!(segments.len() > 1, "a 2500-byte line must be split");
        for segment in &segments {
            let wire = format!(":{nick}!{nick}@localhost PRIVMSG {chan} :{segment}\r\n");
            assert!(
                wire.len() <= 512,
                "a segment must fit an IRC line, got {} bytes",
                wire.len()
            );
        }
    }

    /// Every byte of the input has to survive somewhere. A fix that fits the
    /// limit by dropping text is the bug with a different cut point.
    #[test]
    fn no_input_word_is_lost_in_splitting() {
        let text = format!("alpha {} omega", "filler ".repeat(300));
        let segments = wire_segments("n", "#c", &text);
        let rejoined = segments.join(" ");
        for word in ["alpha", "omega"] {
            assert!(rejoined.contains(word), "{word} was dropped");
        }
        assert_eq!(
            rejoined.split_whitespace().count(),
            text.split_whitespace().count(),
            "splitting changed the word count"
        );
    }

    /// A token longer than the budget still has to make progress, and must not
    /// be cut mid-character.
    #[test]
    fn an_unbroken_token_and_multibyte_text_still_terminate() {
        let segments = wire_segments("n", "#c", &"x".repeat(2000));
        assert!(segments.len() > 1);
        assert!(segments.iter().all(|s| !s.is_empty()));

        let multibyte = "é".repeat(1000);
        let segments = wire_segments("n", "#c", &multibyte);
        assert!(segments.iter().all(|s| !s.is_empty()));
        // Reassembly proves no character was severed: a cut inside a UTF-8
        // sequence could not round-trip as the same string.
        assert_eq!(segments.concat(), multibyte);
    }

    /// B265: a plain substring match let "@bob" match inside "@bobby" (a
    /// false wake for the shorter nick) and, symmetrically, let a shorter
    /// nick claim a mention meant for a longer one that only starts the
    /// same way.
    #[test]
    fn mentions_does_not_match_a_nick_that_is_only_a_prefix() {
        assert!(!mentions("hey @bob check this", "bobby"));
        assert!(!mentions("hey @bobby check this", "bob"));
        assert!(mentions("hey @bob check this", "bob"));
        assert!(mentions("hey @bobby check this", "bobby"));
    }

    #[test]
    fn mentions_respects_punctuation_and_hyphenated_nicks() {
        assert!(mentions("ping @editor-batch-2!", "editor-batch-2"));
        assert!(!mentions("ping @editor-batch-2!", "editor-batch"));
        assert!(mentions("@bob, are you there", "bob"));
        assert!(!mentions("no mention here", "bob"));
    }

    use super::*;
    use std::fs;

    fn tmp_state(name: &str) -> PathBuf {
        let d =
            std::env::temp_dir().join(format!("chat-session-test-{}-{}", std::process::id(), name));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        // Session files live under sessions/<key>.json; a test that writes one
        // by hand needs the directory to exist, as `save` would have made it.
        fs::create_dir_all(d.join("sessions")).unwrap();
        d
    }

    /// An env lookup over a fixed table, so a rung can be tested without an
    /// actual harness and without touching the process environment (which
    /// would race the other tests in this binary).
    fn env_of<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        move |name: &str| {
            pairs
                .iter()
                .find(|(k, _)| *k == name)
                .map(|(_, v)| v.to_string())
        }
    }

    #[test]
    fn explicit_session_flag_wins_over_everything() {
        let env = env_of(&[
            ("CHAT_SESSION_ID", "from-env"),
            ("CLAUDE_CODE_SESSION_ID", "claude-1"),
        ]);
        let (key, src) = resolve_session_key(Some("agent-b"), &env, Some("/repo"), None);
        assert_eq!(key, "agent-b");
        assert_eq!(src, KeySource::Explicit);
    }

    #[test]
    fn chat_session_id_env_wins_over_inference() {
        let env = env_of(&[
            ("CHAT_SESSION_ID", "from-env"),
            ("CLAUDE_CODE_SESSION_ID", "claude-1"),
        ]);
        let (key, src) = resolve_session_key(None, &env, Some("/repo"), None);
        assert_eq!(key, "from-env");
        assert_eq!(src, KeySource::Explicit);
    }

    #[test]
    fn explicit_id_is_reduced_to_a_safe_filename() {
        let env = env_of(&[]);
        let (key, src) = resolve_session_key(Some("../../etc/passwd"), &env, None, None);
        assert_eq!(src, KeySource::Explicit);
        assert!(!key.contains('/'), "key must not contain a path separator");
        // What matters is where the key resolves: one file directly inside
        // sessions/, never a path that climbs out of it.
        let d = tmp_state("explicit_id_is_reduced_to_a_safe_filename");
        let path = Session::path_for(&d, &key);
        assert_eq!(
            path.parent().unwrap(),
            d.join("sessions"),
            "resolved to {}",
            path.display()
        );
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn an_all_dots_explicit_id_falls_through_rather_than_naming_a_directory() {
        let env = env_of(&[("CLAUDE_CODE_SESSION_ID", "claude-1")]);
        let (key, src) = resolve_session_key(Some(".."), &env, Some("/repo"), None);
        assert_eq!(src, KeySource::Harness, "got key {}", key);
    }

    #[test]
    fn each_harness_session_id_gives_a_distinct_key() {
        let a = resolve_session_key(
            None,
            &env_of(&[("CLAUDE_CODE_SESSION_ID", "a")]),
            None,
            None,
        );
        let b = resolve_session_key(
            None,
            &env_of(&[("CLAUDE_CODE_SESSION_ID", "b")]),
            None,
            None,
        );
        assert_eq!(a.1, KeySource::Harness);
        assert_ne!(a.0, b.0, "two Claude Code sessions must not share a key");

        let c = resolve_session_key(None, &env_of(&[("CODEX_SESSION_ID", "c")]), None, None);
        let d = resolve_session_key(None, &env_of(&[("CODEX_SESSION_ID", "d")]), None, None);
        assert_eq!(c.1, KeySource::Harness);
        assert_ne!(c.0, d.0, "two codex sessions must not share a key");

        let e = resolve_session_key(None, &env_of(&[("OPENCODE_PID", "111")]), None, None);
        assert_eq!(e.1, KeySource::Harness);
        assert_ne!(
            e.0,
            resolve_session_key(None, &env_of(&[("OPENCODE_PID", "222")]), None, None).0
        );
    }

    #[test]
    fn a_harness_key_is_stable_for_the_same_ids() {
        let pairs = [("CODEX_SESSION_ID", "same"), ("OPENCODE_PID", "9")];
        let first = resolve_session_key(None, &env_of(&pairs), Some("/a"), None);
        let again = resolve_session_key(None, &env_of(&pairs), Some("/b"), None);
        assert_eq!(
            first.0, again.0,
            "the harness rung must not depend on the worktree"
        );
    }

    /// B278. The nick is a suffix on an identity that does not include it, so
    /// a call with no --nick can still find the session it owns. Hashing the
    /// two together made that impossible and broke the one thing a saved
    /// session is for.
    #[test]
    fn a_key_carries_the_nick_as_a_findable_suffix() {
        let env = env_of(&[("CLAUDE_CODE_SESSION_ID", "one")]);
        let bare = resolve_session_key(None, &env, None, None).0;
        let named = resolve_session_key(None, &env, None, Some("solo")).0;
        assert_eq!(
            named,
            format!("{bare}-solo"),
            "the nick must be a suffix on the nick-free key, not hashed into it"
        );
    }

    /// The nick-less call adopts the one session its identity owns.
    #[test]
    fn a_call_with_no_nick_finds_the_single_session_for_its_identity() {
        let dir = tmp_state("a_call_with_no_nick_finds_the_single_session");
        let sessions = dir.join("sessions");
        fs::write(sessions.join("h-abc-solo.json"), "{}").unwrap();
        assert_eq!(
            sibling_session_in(&dir, "h-abc"),
            Some("h-abc-solo".to_string())
        );
        let _ = fs::remove_dir_all(&dir);
    }

    /// Two nicks under one identity is a parent and its subagent. Picking
    /// either would put one agent back in the other's file, which is B271.
    #[test]
    fn an_ambiguous_identity_is_not_guessed_at() {
        let dir = tmp_state("an_ambiguous_identity_is_not_guessed_at");
        let sessions = dir.join("sessions");
        fs::write(sessions.join("h-abc-parent.json"), "{}").unwrap();
        fs::write(sessions.join("h-abc-child.json"), "{}").unwrap();
        assert_eq!(sibling_session_in(&dir, "h-abc"), None);
        let _ = fs::remove_dir_all(&dir);
    }

    /// A nick-free session that already exists is the caller's own file and
    /// wins over any suffixed sibling.
    #[test]
    fn an_existing_nick_free_session_is_not_redirected() {
        let dir = tmp_state("an_existing_nick_free_session_is_not_redirected");
        let sessions = dir.join("sessions");
        fs::write(sessions.join("h-abc.json"), "{}").unwrap();
        fs::write(sessions.join("h-abc-solo.json"), "{}").unwrap();
        assert_eq!(sibling_session_in(&dir, "h-abc"), None);
        let _ = fs::remove_dir_all(&dir);
    }

    /// A Claude Code subagent shares its parent's process and its
    /// CLAUDE_CODE_SESSION_ID, so the harness rung alone hands both the same
    /// key: measured, a subagent joined as itself and wrote into the parent's
    /// session file, moving the parent's cursors past unread messages.
    #[test]
    fn a_subagent_under_one_harness_id_gets_its_own_session_per_nick() {
        let env = env_of(&[("CLAUDE_CODE_SESSION_ID", "one-session")]);
        let parent = resolve_session_key(None, &env, None, Some("aiskills"));
        let child = resolve_session_key(None, &env, None, Some("t90-chat-mcp"));
        assert_eq!(parent.1, KeySource::Harness);
        assert_ne!(
            parent.0, child.0,
            "one harness id and two nicks must not share a session file"
        );
        // Same nick, same key: a session has to survive the next invocation.
        assert_eq!(
            parent.0,
            resolve_session_key(None, &env, None, Some("aiskills")).0
        );
    }

    /// The nick alone must not name a session, or any process on the machine
    /// could claim another's by choosing the name.
    #[test]
    fn the_same_nick_under_a_different_harness_id_is_a_different_session() {
        let a = resolve_session_key(
            None,
            &env_of(&[("CLAUDE_CODE_SESSION_ID", "a")]),
            None,
            Some("aiskills"),
        );
        let b = resolve_session_key(
            None,
            &env_of(&[("CLAUDE_CODE_SESSION_ID", "b")]),
            None,
            Some("aiskills"),
        );
        assert_ne!(a.0, b.0, "the harness id must still separate two sessions");
    }

    /// Two agents in ONE worktree have nothing but the nick to tell them apart.
    #[test]
    fn two_nicks_in_one_worktree_do_not_share_a_session() {
        let env = env_of(&[]);
        let a = resolve_session_key(None, &env, Some("/repo"), Some("one"));
        let b = resolve_session_key(None, &env, Some("/repo"), Some("two"));
        assert_eq!(a.1, KeySource::Worktree);
        assert_ne!(a.0, b.0);
        let c = resolve_session_key(None, &env, None, Some("one"));
        let d = resolve_session_key(None, &env, None, Some("two"));
        assert_eq!(c.1, KeySource::Shared);
        assert_ne!(c.0, d.0, "the shared rung is per nick too");
    }

    /// `--session ID` is a caller naming a session, so two callers naming the
    /// same one mean to share it. Folding the nick in would break that.
    #[test]
    fn an_explicit_session_id_is_not_split_by_nick() {
        let env = env_of(&[("CLAUDE_CODE_SESSION_ID", "one-session")]);
        let a = resolve_session_key(Some("shared-desk"), &env, None, Some("one"));
        let b = resolve_session_key(Some("shared-desk"), &env, None, Some("two"));
        assert_eq!(a.1, KeySource::Explicit);
        assert_eq!(a.0, b.0, "an explicitly named session is shared on purpose");
    }

    #[test]
    fn a_nested_harness_does_not_inherit_the_outer_agents_session() {
        // Measured: a codex launched from a Claude Code agent keeps that
        // agent's CLAUDE_CODE_SESSION_ID and adds its own CODEX_SESSION_ID.
        let outer = resolve_session_key(
            None,
            &env_of(&[("CLAUDE_CODE_SESSION_ID", "a")]),
            None,
            None,
        );
        let inner = resolve_session_key(
            None,
            &env_of(&[("CLAUDE_CODE_SESSION_ID", "a"), ("CODEX_SESSION_ID", "z")]),
            None,
            None,
        );
        assert_ne!(
            outer.0, inner.0,
            "the inner codex must get its own session, not the outer agent's"
        );
    }

    #[test]
    fn the_worktree_rung_separates_worktrees_and_only_applies_without_a_harness() {
        let env = env_of(&[]);
        let a = resolve_session_key(None, &env, Some("/repo/.claude/worktrees/one"), None);
        let b = resolve_session_key(None, &env, Some("/repo/.claude/worktrees/two"), None);
        assert_eq!(a.1, KeySource::Worktree);
        assert_ne!(a.0, b.0, "sibling worktrees must not share a session");
        // Same root, twice: the same key.
        assert_eq!(
            a.0,
            resolve_session_key(None, &env, Some("/repo/.claude/worktrees/one"), None).0
        );
    }

    #[test]
    fn outside_a_repository_with_no_harness_one_shared_session_is_named_as_such() {
        let (key, src) = resolve_session_key(None, &env_of(&[]), None, None);
        assert_eq!(key, "shared");
        assert_eq!(src, KeySource::Shared);
    }

    #[test]
    fn distinct_session_keys_get_distinct_files() {
        let d = tmp_state("distinct_session_keys_get_distinct_files");
        assert_ne!(
            Session::path_for(&d, "h-1111111111111111"),
            Session::path_for(&d, "h-2222222222222222")
        );
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn an_agent_with_no_file_yet_inherits_the_old_shared_session_without_moving_it() {
        let d = tmp_state("legacy_session_is_inherited_not_moved");
        fs::write(
            Session::legacy_path(&d),
            "{\"server\":\"h:1\",\"nick\":\"old\",\"cursors\":{\"#a\":4}}",
        )
        .unwrap();
        let s = Session::load(&d);
        assert_eq!(s.nick, "old");
        assert_eq!(s.cursor("#a"), 4);
        assert!(
            Session::legacy_path(&d).exists(),
            "the shared file other agents still read must stay put"
        );
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn session_round_trips_server_nick_and_cursors() {
        let d = tmp_state("session_round_trips_server_nick_and_cursors");
        let s = Session {
            server: "127.0.0.1:1234".into(),
            nick: "agent".into(),
            cursors: std::collections::HashMap::from([("#ops".to_string(), 7)]),
        };
        s.save(&d).unwrap();

        let loaded = Session::load(&d);
        assert_eq!(loaded.server, "127.0.0.1:1234");
        assert_eq!(loaded.nick, "agent");
        assert_eq!(loaded.cursor("#ops"), 7);
        assert_eq!(loaded.cursor("#other"), 0);
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn two_keys_keep_separate_sessions_in_one_state_dir() {
        let d = tmp_state("two_keys_keep_separate_sessions_in_one_state_dir");
        save_session_with_key(&d, "agent-a", "h:1", "alice");
        save_session_with_key(&d, "agent-b", "h:2", "bob");

        let a = Session::load_with_key(&d, "agent-a");
        let b = Session::load_with_key(&d, "agent-b");
        assert_eq!(a.server, "h:1");
        assert_eq!(a.nick, "alice");
        assert_eq!(b.server, "h:2");
        assert_eq!(b.nick, "bob");
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn apply_session_with_key_fills_from_the_named_keys_own_session() {
        let d = tmp_state("apply_session_with_key_fills_from_the_named_keys_own_session");
        save_session_with_key(&d, "agent-a", "h:1", "alice");

        let (server, nick, used) = apply_session_with_key("", "", &d, false, "agent-a");
        assert_eq!(server, "h:1");
        assert_eq!(nick, "alice");
        assert!(used);

        // A different key sees no session at all, even in the same state dir.
        let (server, nick, used) = apply_session_with_key("", "", &d, false, "agent-b");
        assert_eq!(server, "");
        assert_eq!(nick, "");
        assert!(!used);
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn session_missing_file_is_empty_session() {
        let d = tmp_state("session_missing_file_is_empty_session");
        let s = Session::load(&d);
        assert_eq!(s.server, "");
        assert_eq!(s.nick, "");
        assert_eq!(s.cursor("#x"), 0);
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn session_malformed_json_recovers_to_empty() {
        let d = tmp_state("session_malformed_json_recovers_to_empty");
        fs::write(Session::path(&d), "{ not valid json !!!").unwrap();
        let s = Session::load(&d);
        assert_eq!(s.server, "");
        assert_eq!(s.nick, "");
        assert_eq!(s.cursor("#x"), 0);
        // a subsequent save overwrites the malformed file cleanly
        let s2 = Session {
            server: "h:1".into(),
            nick: String::new(),
            cursors: Default::default(),
        };
        s2.save(&d).unwrap();
        let reloaded = Session::load(&d);
        assert_eq!(reloaded.server, "h:1");
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn session_partial_json_keeps_what_parses_or_resets() {
        // A JSON object missing the cursors key must still deserialize
        // (serde default) rather than failing the whole file.
        let d = tmp_state("session_partial_json_keeps_what_parses_or_resets");
        fs::write(Session::path(&d), "{\"server\":\"h:1\",\"nick\":\"n\"}").unwrap();
        let s = Session::load(&d);
        assert_eq!(s.server, "h:1");
        assert_eq!(s.nick, "n");
        assert_eq!(s.cursor("#x"), 0);
        let _ = fs::remove_dir_all(&d);
    }

    // ---- wire_segments ---------------------------------------------------

    #[test]
    fn a_multi_line_message_becomes_one_segment_per_line() {
        let out = wire_segments("me", "#c", "alpha\nbeta\ngamma");
        assert_eq!(out, vec!["alpha", "beta", "gamma"]);
    }

    #[test]
    fn a_crlf_line_carries_no_stray_carriage_return() {
        let out = wire_segments("me", "#c", "alpha\r\nbeta");
        assert_eq!(out, vec!["alpha", "beta"]);
    }

    #[test]
    fn a_blank_line_stays_a_paragraph_break() {
        let out = wire_segments("me", "#c", "alpha\n\nbeta");
        assert_eq!(out, vec!["alpha", " ", "beta"]);
    }

    #[test]
    fn a_paragraph_longer_than_the_line_limit_is_split_not_truncated() {
        let word = "word ".repeat(200); // 1000 bytes on one line
        let out = wire_segments("me", "#c", word.trim_end());
        assert!(
            out.len() > 1,
            "one line was not split: {} segments",
            out.len()
        );
        let overhead = ":me!me@localhost PRIVMSG #c :".len() + 2;
        for segment in &out {
            assert!(
                segment.len() + overhead <= 512,
                "a segment overruns the 512-byte message: {} bytes",
                segment.len()
            );
        }
        // Nothing is lost: the words come back in order and in full.
        assert_eq!(out.join(" ").split_whitespace().count(), 200);
    }

    #[test]
    fn a_single_word_wider_than_the_budget_still_terminates() {
        let out = wire_segments("me", "#c", &"x".repeat(2000));
        assert!(out.len() >= 5, "{} segments", out.len());
        assert_eq!(out.concat().len(), 2000, "no bytes were dropped");
    }

    #[test]
    fn a_split_never_lands_inside_a_character() {
        // Multi-byte characters only, so a byte-indexed cut would panic or
        // produce mojibake rather than a shorter line.
        let out = wire_segments("me", "#c", &"é".repeat(600));
        assert_eq!(out.concat().chars().count(), 600);
    }

    #[test]
    fn cursor_updates_monotonically() {
        let d = tmp_state("cursor_updates_monotonically");
        save_cursor(&d, "#ops", 5, false);
        save_cursor(&d, "#ops", 3, false); // lower: ignored
        assert_eq!(Session::load(&d).cursor("#ops"), 5);
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn no_session_flag_skips_cursor_save() {
        let d = tmp_state("no_session_flag_skips_cursor_save");
        save_cursor(&d, "#ops", 9, true);
        assert_eq!(Session::load(&d).cursor("#ops"), 0);
        let _ = fs::remove_dir_all(&d);
    }

    // --state was documented and never parsed. These assert the flag path
    // only, which returns before any environment is read, so the test does
    // not race other tests over AI_CHAT_HOME.
    #[test]
    fn state_flag_selects_the_client_state_dir() {
        let dir = client_state_dir(&["--state".into(), "/tmp/chat-state-probe".into()]);
        assert_eq!(dir, PathBuf::from("/tmp/chat-state-probe"));

        // It is found wherever it sits in the argument list, since it is
        // resolved from the raw arguments rather than by a subcommand parser.
        let dir = client_state_dir(&[
            "--chan".into(),
            "#c".into(),
            "--state".into(),
            "/tmp/elsewhere".into(),
            "--nick".into(),
            "me".into(),
        ]);
        assert_eq!(dir, PathBuf::from("/tmp/elsewhere"));
    }

    #[test]
    fn a_blank_or_absent_state_flag_does_not_win() {
        // A blank value must not resolve the state dir to "": it falls through
        // to $AI_CHAT_HOME / the XDG default, whatever those are here.
        let dir = client_state_dir(&["--state".into(), "   ".into()]);
        assert_ne!(dir, PathBuf::from(""));
        assert_ne!(dir, PathBuf::from("   "));

        // A trailing --state with no value must not panic.
        let dir = client_state_dir(&["--state".into()]);
        assert_ne!(dir, PathBuf::from(""));
    }

    // B246: presence is opt-in. Every existing reader of `tail` receives PRIVMSG
    // only, so turning membership on by default would change what all of them
    // see. The flag is the whole contract, so the default is worth asserting.
    #[test]
    fn presence_is_off_unless_asked_for() {
        let o = parse_opts(&["--chan".into(), "#ops".into(), "--nick".into(), "me".into()]);
        assert!(!o.presence, "membership lines must not appear by default");
        let p = parse_opts(&[
            "--chan".into(),
            "#ops".into(),
            "--nick".into(),
            "me".into(),
            "--presence".into(),
        ]);
        assert!(p.presence, "--presence turns them on");
    }

    #[test]
    fn parse_opts_reads_mention_flags() {
        let o = parse_opts(&[
            "--mentions".into(),
            "--mention-exit".into(),
            "--chan".into(),
            "#m".into(),
            "--server".into(),
            "h:1".into(),
            "--nick".into(),
            "me".into(),
        ]);
        assert!(o.mentions);
        assert!(o.mention_exit);
        assert_eq!(o.chan, "#m");
        assert!(!o.no_session);
    }

    #[test]
    fn session_clear_drops_cursors_keeps_identity() {
        let d = tmp_state("session_clear_drops_cursors_keeps_identity");
        let mut s = Session {
            server: "h:1".into(),
            nick: "me".into(),
            cursors: std::collections::HashMap::from([("#a".to_string(), 3)]),
        };
        s.save(&d).unwrap();
        s.cursors.clear();
        s.save(&d).unwrap();
        let loaded = Session::load(&d);
        assert_eq!(loaded.server, "h:1");
        assert_eq!(loaded.nick, "me");
        assert_eq!(loaded.cursor("#a"), 0);
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn read_last_id_parses_numeric_reply() {
        // Simulate the server's `:server 999 nick #chan 42` reply line.
        let line = ":server 999 me #ops 42";
        let id = line
            .split_whitespace()
            .last()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0);
        assert_eq!(id, 42);
        // A malformed reply yields 0 (client treats it as an empty channel).
        let bad = " 999 me #ops";
        let id2 = bad
            .split_whitespace()
            .last()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0);
        assert_eq!(id2, 0);
    }

    // ---- B118: an IPv6 server address must be dialable -------------------

    #[test]
    fn server_host_reads_ipv4_and_hostnames_exactly_as_before() {
        assert_eq!(server_host("127.0.0.1:6667"), "127.0.0.1");
        assert_eq!(server_host("192.168.1.5:1234"), "192.168.1.5");
        assert_eq!(server_host("localhost:6667"), "localhost");
        assert_eq!(server_host("chat.example.org:6667"), "chat.example.org");
        assert_eq!(server_host("localhost"), "localhost");
        assert_eq!(server_host("localhost:"), "localhost");
    }

    #[test]
    fn server_host_extracts_the_host_from_an_ipv6_address() {
        // Bracketed: the first-colon split used to return "[".
        assert_eq!(server_host("[::1]:6667"), "::1");
        assert_eq!(
            server_host("[fe80::1ff:fe23:4567:890a]:6667"),
            "fe80::1ff:fe23:4567:890a"
        );
        // Bare, the form the announce beacon's host+port concatenation makes:
        // the first-colon split used to return the empty string. std's
        // to_socket_addrs splits this on the LAST colon, so this must too.
        assert_eq!(server_host("::1:6667"), "::1");
        assert_eq!(
            server_host("fe80::1ff:fe23:4567:890a:6667"),
            "fe80::1ff:fe23:4567:890a"
        );
    }

    #[test]
    fn an_ip_literal_becomes_an_ip_server_name_not_a_dns_name() {
        use std::net::{IpAddr, Ipv6Addr, SocketAddr};
        // The whole point of B118: ServerName::try_from rejects "::1" as an
        // invalid DNS name, so an IP must not take the DNS-name path at all.
        let v6: SocketAddr = SocketAddr::new(IpAddr::V6(Ipv6Addr::LOCALHOST), 6667);
        for addr in ["[::1]:6667", "::1:6667"] {
            let name = server_name(addr, &v6)
                .unwrap_or_else(|e| panic!("{} must yield a server name, got {}", addr, e));
            assert!(
                matches!(name, ServerName::IpAddress(_)),
                "{} must present an IP server name, got {:?}",
                addr,
                name
            );
        }
        let v4: SocketAddr = "127.0.0.1:6667".parse().unwrap();
        assert!(
            matches!(
                server_name("127.0.0.1:6667", &v4).unwrap(),
                ServerName::IpAddress(_)
            ),
            "an IPv4 literal is an IP, not a DNS name"
        );
        // A hostname still presents a DNS name.
        assert!(
            matches!(
                server_name("localhost:6667", &v4).unwrap(),
                ServerName::DnsName(_)
            ),
            "a hostname must keep the DNS-name path"
        );
    }

    // ---- the local channel reader (no server) -----------------------------

    fn write_log(home: &std::path::Path, chan: &str, lines: &[&str]) {
        let dir = home.join("channels");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join(format!("{}.log", chan)), lines.join("\n") + "\n").unwrap();
    }

    #[test]
    fn msg_line_id_reads_only_stored_message_rows() {
        assert_eq!(msg_line_id("MSG #ops 7 1700000000 alice :hi"), Some(7));
        // Anything that is not a stored MSG row carries no id, and a
        // malformed row must be skipped rather than aborting a read.
        assert_eq!(msg_line_id(""), None);
        assert_eq!(msg_line_id("JOIN #ops"), None);
        assert_eq!(msg_line_id("MSG #ops notanid 0 a :x"), None);
        assert_eq!(msg_line_id("MSG #ops"), None);
        assert_eq!(msg_line_id(" MSG #ops 7 0 a :x"), None);
    }

    #[test]
    fn local_last_id_takes_the_maximum_not_the_final_line() {
        let home = tmp_state("local_last_id_takes_the_maximum");
        // A missing channel has no messages, and must not be an error here:
        // a tail seeding its cursor on an empty channel starts at 0.
        assert_eq!(local_last_id(&home, "#nope"), 0);
        // The last line is deliberately NOT the highest: an interleaved or
        // truncated final write must not walk the cursor backwards.
        write_log(
            &home,
            "#ops",
            &[
                "MSG #ops 1 0 alice :one",
                "MSG #ops 9 0 alice :nine",
                "garbage",
                "MSG #ops 4 0 alice :four",
            ],
        );
        assert_eq!(local_last_id(&home, "#ops"), 9);
    }

    #[test]
    fn local_read_returns_only_rows_after_the_cursor() {
        let home = tmp_state("local_read_after_the_cursor");
        write_log(
            &home,
            "#ops",
            &[
                "MSG #ops 1 0 alice :one",
                "MSG #ops 2 0 alice :two",
                "MSG #ops 3 0 alice :three",
            ],
        );
        // The return value is the highest id printed, which is what the
        // caller saves as the new cursor.
        assert_eq!(local_read(&home, "#ops", 0, None), 3);
        assert_eq!(local_read(&home, "#ops", 2, None), 3);
        // Caught up: nothing to print, so no cursor movement.
        assert_eq!(local_read(&home, "#ops", 3, None), 0);
        assert_eq!(local_read(&home, "#ops", 99, None), 0);
    }

    #[test]
    fn local_read_mention_filter_matches_only_the_named_nick() {
        let home = tmp_state("local_read_mention_filter");
        write_log(
            &home,
            "#ops",
            &[
                "MSG #ops 1 0 alice :nothing for anyone",
                "MSG #ops 2 0 alice :ping @bob please",
                "MSG #ops 3 0 alice :ping @carol please",
            ],
        );
        assert_eq!(local_read(&home, "#ops", 0, Some("bob")), 2);
        assert_eq!(local_read(&home, "#ops", 0, Some("carol")), 3);
        assert_eq!(local_read(&home, "#ops", 0, Some("dave")), 0);
        // The filter must not leak past the cursor either.
        assert_eq!(local_read(&home, "#ops", 2, Some("bob")), 0);
    }

    #[test]
    fn local_reads_use_the_shared_channel_home_not_a_private_state_dir() {
        // --state is one client's own pins and sessions; the channel logs are
        // the server's shared storage. An agent handed its own --state must
        // still read the channels everyone shares, so channels_home() resolves
        // AI_CHAT_HOME (or the XDG default) and never looks at --state.
        let private = tmp_state("local_reads_ignore_state").join("private");
        let opts = parse_opts(&[
            "--local".to_string(),
            "--state".to_string(),
            private.display().to_string(),
            "--chan".to_string(),
            "#ops".to_string(),
        ]);
        assert!(opts.local, "--local must be parsed, not swallowed");
        assert_ne!(
            channels_home(),
            private,
            "a private --state dir must not become the channel home"
        );
    }
}
