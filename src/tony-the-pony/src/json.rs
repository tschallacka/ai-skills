// MODE: DEV
// PACKAGE: PROD
//! Just enough JSON to read one string out of a hook payload, and to write one
//! string back.
//!
//! A serialiser crate would be the obvious choice, but the reason this binary
//! exists is that no host guarantees a runtime, and every dependency is another
//! thing to audit in a tool whose job is to be trusted. The payload shape is
//! fixed and documented: `{"tool_input": {"command": "..."}}`.
//!
//! Deliberately narrow: it walks to a key path and reads a string. It does not
//! validate the document, and a malformed payload yields `None` rather than an
//! error — the caller treats an unreadable payload as "no opinion", because
//! failing to parse a hook payload must never block a tool call.

/// Read the string at a key path, e.g. `["tool_input", "command"]`.
pub fn string_at(document: &str, path: &[&str]) -> Option<String> {
    let mut rest = document;
    for (depth, key) in path.iter().enumerate() {
        let at = find_key(rest, key)?;
        rest = &rest[at..];
        if depth + 1 == path.len() {
            return read_string(rest);
        }
    }
    None
}

/// Byte offset just past `"key":`, skipping matches inside string literals.
fn find_key(document: &str, key: &str) -> Option<usize> {
    let bytes = document.as_bytes();
    let needle = format!("\"{key}\"");
    let mut i = 0;
    let mut in_string = false;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' if in_string => {
                i += 2;
                continue;
            }
            b'"' => {
                if !in_string && document[i..].starts_with(&needle) {
                    let mut j = i + needle.len();
                    while j < bytes.len() && (bytes[j] as char).is_whitespace() {
                        j += 1;
                    }
                    if j < bytes.len() && bytes[j] == b':' {
                        return Some(j + 1);
                    }
                }
                in_string = !in_string;
            }
            _ => {}
        }
        i += 1;
    }
    None
}

/// Read the JSON string starting at the first `"` in `text`, unescaping it.
fn read_string(text: &str) -> Option<String> {
    let mut chars = text.char_indices().skip_while(|(_, c)| c.is_whitespace());
    let (_, first) = chars.next()?;
    if first != '"' {
        return None;
    }
    let mut out = String::new();
    let mut iter = text[text.find('"')? + 1..].chars();
    while let Some(c) = iter.next() {
        match c {
            '"' => return Some(out),
            '\\' => match iter.next()? {
                'n' => out.push('\n'),
                't' => out.push('\t'),
                'r' => out.push('\r'),
                'b' => out.push('\u{8}'),
                'f' => out.push('\u{c}'),
                '/' => out.push('/'),
                '\\' => out.push('\\'),
                '"' => out.push('"'),
                'u' => {
                    let hex: String = iter.by_ref().take(4).collect();
                    let code = u32::from_str_radix(&hex, 16).ok()?;
                    // A lone surrogate is not representable; keep the payload
                    // readable rather than failing the whole parse.
                    out.push(char::from_u32(code).unwrap_or('\u{fffd}'));
                }
                other => out.push(other),
            },
            _ => out.push(c),
        }
    }
    None // unterminated
}

/// Encode a string as a JSON string literal, quotes included.
pub fn encode_string(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for c in value.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
