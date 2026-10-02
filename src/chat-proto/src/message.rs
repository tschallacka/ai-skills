// MODE: DEV
// PACKAGE: PROD
//! The shared IRC message and numeric-tag model.
//!
//! Single source of truth for the wire grammar so the server and the client
//! crate cannot drift. RFC 1459 message form:
//!
//! ```text
//! [":" prefix] command [ params ... [ ":" trailing ] ]
//! ```
//!
//! This crate models that grammar (parse + serialize), centralizes the numeric
//! tags the registration/channel lifecycle needs, and owns the additive
//! history-extension reply and its terminating marker.

use std::error::Error;
use std::fmt;

/// The terminating marker the server sends after a FETCH history reply. The
/// client stops reading history on exactly this line.
pub const FETCH_END: &str = ":server 000 end-of-history";

/// One IRCv3 message tag: a key, and an optional value (a bare key with no
/// `=` is a flag tag, per the spec -- `value` is `None` for exactly that
/// case, not `Some("")`, so a round-trip cannot invent an `=` that was never
/// there).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tag {
    pub key: String,
    pub value: Option<String>,
}

/// A parsed IRC message: optional tags, prefix, command, params, and trailing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    /// IRCv3 message tags, in the order they appeared on the wire. Empty when
    /// the line carried no leading `@...` segment.
    pub tags: Vec<Tag>,
    /// `nick!user@host` form, or a server name; `None` when absent.
    pub prefix: Option<String>,
    /// The command or numeric (e.g. `PRIVMSG`, `001`).
    pub command: String,
    /// Positional parameters (never includes the trailing part).
    pub params: Vec<String>,
    /// The trailing text after a `:` (delivery payload). `None` when absent.
    pub trailing: Option<String>,
}

impl Message {
    /// Parse one IRC line (no trailing `\r\n`) into its parts.
    ///
    /// Returns an error only on a structurally impossible line; a line with no
    /// command is rejected. Leading and trailing spaces are tolerated.
    pub fn parse(line: &str) -> Result<Message, ParseError> {
        let line = line.trim_matches(['\r', '\n']);
        let line = line.trim_start();
        if line.is_empty() {
            return Err(ParseError("empty line".into()));
        }
        let (tags, line) = if let Some(rest) = line.strip_prefix('@') {
            let (tag_segment, tail) = rest
                .split_once(' ')
                .ok_or_else(|| ParseError("tags with no command".into()))?;
            (parse_tags(tag_segment)?, tail.trim_start())
        } else {
            (Vec::new(), line)
        };
        let (prefix, rest) = if let Some(p) = line.strip_prefix(':') {
            match p.split_once(' ') {
                Some((prefix, tail)) => {
                    let prefix = prefix.trim();
                    if prefix.is_empty() {
                        return Err(ParseError("empty prefix".into()));
                    }
                    (Some(prefix.to_string()), tail.trim_start().to_string())
                }
                None => return Err(ParseError("prefix with no command".into())),
            }
        } else {
            (None, line.to_string())
        };

        let mut command = String::new();
        let mut params: Vec<String> = Vec::new();
        let mut trailing: Option<String> = None;

        let mut part = String::new();
        let mut in_trailing = false;
        let mut had_trailing_colon = false;
        for ch in rest.chars() {
            if in_trailing {
                part.push(ch);
                continue;
            }
            match ch {
                ' ' if command.is_empty() && part.is_empty() => {}
                ' ' => {
                    if command.is_empty() {
                        command = std::mem::take(&mut part);
                    } else {
                        params.push(std::mem::take(&mut part));
                    }
                }
                ':' if part.is_empty() && !command.is_empty() => {
                    in_trailing = true;
                    had_trailing_colon = true;
                }
                _ => part.push(ch),
            }
        }
        if !part.is_empty() || (in_trailing && had_trailing_colon) {
            if command.is_empty() {
                command = std::mem::take(&mut part);
            } else if in_trailing {
                trailing = Some(std::mem::take(&mut part));
            } else {
                params.push(std::mem::take(&mut part));
            }
        }
        if command.is_empty() {
            return Err(ParseError("no command".into()));
        }
        Ok(Message {
            tags,
            prefix,
            command,
            params,
            trailing,
        })
    }
}

impl Message {
    /// Serialize back to the wire form (no trailing `\r\n`).
    ///
    /// The tags segment is omitted ENTIRELY when `tags` is empty -- not
    /// emitted as a bare `@ ` -- so every message built before tags existed,
    /// and everything this crate has already serialized in the past, still
    /// round-trips byte-identical.
    pub fn serialize(&self) -> String {
        let mut out = String::new();
        if !self.tags.is_empty() {
            out.push('@');
            out.push_str(&serialize_tags(&self.tags));
            out.push(' ');
        }
        if let Some(p) = &self.prefix {
            out.push(':');
            out.push_str(p);
            out.push(' ');
        }
        out.push_str(&self.command);
        for p in &self.params {
            out.push(' ');
            out.push_str(p);
        }
        if let Some(t) = &self.trailing {
            out.push_str(" :");
            out.push_str(t);
        }
        out
    }
}

/// Parse an IRCv3 tag segment (the part between `@` and the following
/// space, already split off by the caller): `key[=value][;key[=value]]...`.
/// A key with no value is a flag tag (`value` is `None`). An empty key
/// anywhere in the list is refused -- it cannot round-trip to anything a
/// second parse would agree on.
fn parse_tags(segment: &str) -> Result<Vec<Tag>, ParseError> {
    let mut tags = Vec::new();
    for entry in segment.split(';') {
        let (key, value) = match entry.split_once('=') {
            Some((key, value)) => (key, Some(unescape_tag_value(value))),
            None => (entry, None),
        };
        if key.is_empty() {
            return Err(ParseError(format!("empty tag key in @{segment}")));
        }
        tags.push(Tag {
            key: key.to_string(),
            value,
        });
    }
    Ok(tags)
}

fn serialize_tags(tags: &[Tag]) -> String {
    tags.iter()
        .map(|tag| match &tag.value {
            Some(value) => format!("{}={}", tag.key, escape_tag_value(value)),
            None => tag.key.clone(),
        })
        .collect::<Vec<_>>()
        .join(";")
}

/// IRCv3 tag-value escaping: `;`, space, CR, LF and `\` itself cannot appear
/// literally in a tag value (`;` and space would be read as delimiters,
/// CR/LF would end the line), so each is backslash-escaped on the way out.
fn escape_tag_value(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            ';' => out.push_str("\\:"),
            ' ' => out.push_str("\\s"),
            '\\' => out.push_str("\\\\"),
            '\r' => out.push_str("\\r"),
            '\n' => out.push_str("\\n"),
            _ => out.push(ch),
        }
    }
    out
}

/// The inverse of [`escape_tag_value`]. A trailing lone backslash, or a
/// backslash before a character with no defined escape, drops the backslash
/// and keeps the following character literally -- the IRCv3 spec leaves this
/// case's exact handling to the implementation, and silently dropping rather
/// than refusing the whole message keeps one malformed tag from taking the
/// rest of the line down with it.
fn unescape_tag_value(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut chars = value.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }
        match chars.next() {
            Some(':') => out.push(';'),
            Some('s') => out.push(' '),
            Some('\\') => out.push('\\'),
            Some('r') => out.push('\r'),
            Some('n') => out.push('\n'),
            Some(other) => out.push(other),
            None => {}
        }
    }
    out
}

/// Error type for a malformed message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError(pub String);

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "message parse error: {}", self.0)
    }
}

impl Error for ParseError {}

/// Centralized RFC registration/channel numeric tags.
pub mod numerics {
    pub const RPL_WELCOME: &str = "001";
    pub const RPL_YOURHOST: &str = "002";
    pub const RPL_CREATED: &str = "003";
    pub const RPL_MYINFO: &str = "004";
    pub const RPL_ISUPPORT: &str = "005";
    pub const ERR_NICKNAMEINUSE: &str = "433";
    pub const RPL_NAMREPLY: &str = "353";
    pub const RPL_ENDOFNAMES: &str = "366";
    pub const RPL_MOTDSTART: &str = "375";
    pub const RPL_MOTD: &str = "372";
    pub const RPL_ENDOFMOTD: &str = "376";
}

/// Build a numeric-tagged reply with the given server prefix.
pub fn numeric(server: &str, code: &str, nick: &str, text: &str) -> Message {
    Message {
        tags: Vec::new(),
        prefix: Some(server.to_string()),
        command: code.to_string(),
        params: vec![nick.to_string()],
        trailing: Some(text.to_string()),
    }
}

/// Build the FETCH history-end marker line exactly once: `FETCH_END #chan`.
pub fn fetch_end(chan: &str) -> Message {
    Message {
        tags: Vec::new(),
        prefix: Some("server".to_string()),
        command: "000".to_string(),
        params: vec!["end-of-history".to_string(), chan.to_string()],
        trailing: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_privmsg_prefix_form() {
        let m = Message::parse(":nick!user@host PRIVMSG #chan :hello world").unwrap();
        assert_eq!(m.prefix.as_deref(), Some("nick!user@host"));
        assert_eq!(m.command, "PRIVMSG");
        assert_eq!(m.params, vec!["#chan".to_string()]);
        assert_eq!(m.trailing.as_deref(), Some("hello world"));
        // round-trip identity
        assert_eq!(m.serialize(), ":nick!user@host PRIVMSG #chan :hello world");
    }

    #[test]
    fn parses_numeric_welcome() {
        let m = Message::parse(":server 001 nick :Welcome").unwrap();
        assert_eq!(m.prefix.as_deref(), Some("server"));
        assert_eq!(m.command, "001");
        assert_eq!(m.params, vec!["nick".to_string()]);
        assert_eq!(m.trailing.as_deref(), Some("Welcome"));
    }

    #[test]
    fn parses_params_without_trailing() {
        let m = Message::parse("JOIN #ops").unwrap();
        assert_eq!(m.prefix, None);
        assert_eq!(m.command, "JOIN");
        assert_eq!(m.params, vec!["#ops".to_string()]);
        assert_eq!(m.trailing, None);
    }

    #[test]
    fn rejects_empty() {
        assert!(Message::parse("").is_err());
        assert!(Message::parse("  ").is_err());
    }

    #[test]
    fn numeric_helper_forms_correct_tag() {
        let m = numeric("server", numerics::RPL_WELCOME, "nick", "Welcome to chat");
        assert_eq!(m.serialize(), ":server 001 nick :Welcome to chat");
    }

    #[test]
    fn fetch_end_marker_is_stable() {
        let m = fetch_end("#ops");
        assert_eq!(m.serialize(), ":server 000 end-of-history #ops");
    }

    #[test]
    fn untagged_lines_round_trip_unchanged() {
        // The exact invariant T132 exists to protect: nothing built or parsed
        // before tags existed may gain a byte from their addition.
        let m = Message::parse(":nick!user@host PRIVMSG #chan :hello world").unwrap();
        assert!(m.tags.is_empty());
        assert_eq!(m.serialize(), ":nick!user@host PRIVMSG #chan :hello world");
    }

    #[test]
    fn parses_and_round_trips_a_single_tag() {
        let m = Message::parse("@msgid=42 :nick!user@host PRIVMSG #chan :hi").unwrap();
        assert_eq!(
            m.tags,
            vec![Tag {
                key: "msgid".to_string(),
                value: Some("42".to_string()),
            }]
        );
        assert_eq!(m.serialize(), "@msgid=42 :nick!user@host PRIVMSG #chan :hi");
    }

    #[test]
    fn parses_multiple_tags_in_order() {
        let m = Message::parse("@a=1;b=2;flag PRIVMSG #chan :hi").unwrap();
        assert_eq!(
            m.tags,
            vec![
                Tag {
                    key: "a".to_string(),
                    value: Some("1".to_string())
                },
                Tag {
                    key: "b".to_string(),
                    value: Some("2".to_string())
                },
                Tag {
                    key: "flag".to_string(),
                    value: None
                },
            ]
        );
        assert_eq!(m.serialize(), "@a=1;b=2;flag PRIVMSG #chan :hi");
    }

    #[test]
    fn tags_with_no_prefix_still_parse() {
        let m = Message::parse("@msgid=7 JOIN #ops").unwrap();
        assert_eq!(m.tags[0].key, "msgid");
        assert_eq!(m.prefix, None);
        assert_eq!(m.command, "JOIN");
    }

    #[test]
    fn tag_value_escaping_round_trips() {
        // ';', space, backslash, CR and LF each need escaping, or they would
        // be read as delimiters or end the line outright.
        let raw = "a;b c\\d\r\n";
        let m = Message::parse(&format!(
            "@text={} PRIVMSG #chan :hi",
            escape_tag_value(raw)
        ))
        .unwrap();
        assert_eq!(m.tags[0].value.as_deref(), Some(raw));
        // And the serialized form re-parses to the same value.
        let round_tripped = Message::parse(&m.serialize()).unwrap();
        assert_eq!(round_tripped.tags[0].value.as_deref(), Some(raw));
    }

    #[test]
    fn a_bare_key_is_a_flag_tag_not_an_empty_value() {
        let m = Message::parse("@away PRIVMSG #chan :hi").unwrap();
        assert_eq!(m.tags[0].key, "away");
        assert_eq!(m.tags[0].value, None);
        // Serializing must not invent an "=" that was never on the wire.
        assert_eq!(m.serialize(), "@away PRIVMSG #chan :hi");
    }

    #[test]
    fn malformed_tag_segments_are_refused() {
        assert!(Message::parse("@=novalue PRIVMSG #chan :hi").is_err());
        assert!(Message::parse("@a=1;;b=2 PRIVMSG #chan :hi").is_err());
        assert!(Message::parse("@").is_err());
    }
}

#[cfg(test)]
mod parse_dbg {
    use super::*;
    #[test]
    fn dbg_user_and_nick() {
        let n = Message::parse("NICK alice").unwrap();
        assert_eq!(n.command, "NICK");
        assert_eq!(n.params, vec!["alice"]);
        let u = Message::parse("USER alice 0 * :Alice").unwrap();
        assert_eq!(u.command, "USER");
        assert_eq!(u.params, vec!["alice", "0", "*"]);
        assert_eq!(u.trailing.as_deref(), Some("Alice"));
        let q = Message::parse("QUIT").unwrap();
        assert_eq!(q.command, "QUIT");
    }
}
