// MODE: DEV
// PACKAGE: PROD
//! Wire-format helpers shared by both front ends: splitting a message into
//! IRC-line-safe segments, a minimal JSON-field reader for beacon packets,
//! the channel-name rule, and the mention matcher (T101 split out of
//! lib.rs).

/// The wire segments one message becomes: never more than one IRC line each,
/// and never a line carrying an embedded newline.
///
/// Both cuts are made here because either one, left out, is a silent loss. A
/// newline inside a PRIVMSG trailing is a second line terminator, so a
/// multi-line message reaches the server as its first line alone (B266); and
/// RFC 1459 caps a message at 512 bytes including the prefix the server
/// prepends and the CRLF, so one long paragraph overruns on its own. The room
/// the text has is what is left after `:nick!nick@localhost PRIVMSG #chan :`,
/// and the caller sends one PRIVMSG per segment.
///
/// In the library rather than a front end because both front ends need it and
/// neither may disagree about it.
pub fn wire_segments(nick: &str, chan: &str, text: &str) -> Vec<String> {
    let overhead = format!(":{}!{}@localhost PRIVMSG {} :", nick, nick, chan).len() + 2;
    let budget = 512usize.saturating_sub(overhead).max(1);
    let mut out = Vec::new();
    for line in text.split('\n') {
        // A CRLF-terminated input line keeps no stray CR: it would reach the
        // wire as a second line terminator.
        let line = line.strip_suffix('\r').unwrap_or(line);
        if line.is_empty() {
            // A blank line is a paragraph break, and an empty PRIVMSG trailing
            // is what a server is entitled to drop. One space keeps the break
            // visible in a line-based medium rather than silently closing it up.
            out.push(" ".to_string());
            continue;
        }
        let mut rest = line;
        while !rest.is_empty() {
            let (head, tail) = split_at_budget(rest, budget);
            out.push(head.to_string());
            rest = tail;
        }
    }
    out
}

/// Split a line at no more than `budget` bytes, on a word boundary where there
/// is one and always on a character boundary. The head is never empty, so a
/// caller looping on the tail terminates.
fn split_at_budget(line: &str, budget: usize) -> (&str, &str) {
    if line.len() <= budget {
        return (line, "");
    }
    // The last byte index that is both within budget and a char boundary.
    let mut cut = budget;
    while cut > 0 && !line.is_char_boundary(cut) {
        cut -= 1;
    }
    // Prefer the last space inside the budget, so a cut lands between words
    // rather than inside one. A single word longer than the budget has none,
    // and is cut where it is.
    if let Some(space) = line[..cut].rfind(' ') {
        if space > 0 {
            return (&line[..space], line[space + 1..].trim_start_matches(' '));
        }
    }
    if cut == 0 {
        // A single character wider than the budget: emit it rather than loop.
        let one = line
            .char_indices()
            .nth(1)
            .map(|(index, _)| index)
            .unwrap_or(line.len());
        return (&line[..one], &line[one..]);
    }
    (&line[..cut], &line[cut..])
}

pub fn json_field(s: &str, key: &str) -> Option<String> {
    let marker = format!("\"{}\":", key);
    let idx = s.find(&marker)?;
    let rest = &s[idx + marker.len()..];
    let rest = rest.trim_start();
    if let Some(r) = rest.strip_prefix('"') {
        // String value: read to the closing quote.
        let end = r.find('"')?;
        Some(r[..end].to_string())
    } else {
        // Bare literal (the beacon's port and started are numbers): read to
        // the next delimiter.
        let end = rest.find([',', '}'])?;
        Some(rest[..end].trim().to_string())
    }
}

/// The channel log a local read walks: the same file the server appends to.
/// The server's own channel-name rule, restated here because a LOCAL read
/// never reaches the server to be checked by it.
///
/// A remote read is validated server-side (`valid_chan` in chat-server-rs,
/// called before every JOIN, PRIVMSG and fetch), so `--chan` could be trusted
/// for as long as every path went through a socket. `--local` walks the log
/// file directly, which takes the server out of the loop and turns the channel
/// name into a path segment: `local_chan_log` joins it straight into
/// `<home>/channels/<chan>.log`, and `Path::join` does not resolve `..`, so
/// `--chan ../../../../tmp/x` reads `/tmp/x.log`. The exit code then differs by
/// whether that file exists, which answers "does this path exist" for any path
/// the caller names.
///
/// Kept character-for-character identical to the server's rule rather than
/// merely "safe": a name the server would refuse must not be readable by going
/// around it, or the two disagree about what a channel is.
pub fn valid_chan(c: &str) -> bool {
    c.len() > 1
        && c.len() <= 33
        && c.starts_with('#')
        && c[1..]
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_' || ch == '-')
}

/// Whether `text` mentions `nick` as a bounded "@nick", not merely a
/// substring (B265): the character right after the match, if any, must not
/// itself be a nick character, or "@bob" matches inside "@bobby" and a
/// shorter nick wakes on a longer one that only starts the same way.
pub fn mentions(text: &str, nick: &str) -> bool {
    let needle = format!("@{nick}");
    let mut start = 0;
    while let Some(found) = text[start..].find(&needle) {
        let pos = start + found;
        let after = pos + needle.len();
        let bounded = text[after..]
            .chars()
            .next()
            .is_none_or(|ch| !(ch.is_ascii_alphanumeric() || ch == '_' || ch == '-'));
        if bounded {
            return true;
        }
        start = after;
    }
    false
}

/// One stored `MSG #chan <id> ...` line's id, or None for anything else.
pub fn msg_line_id(line: &str) -> Option<u64> {
    if !line.starts_with("MSG ") {
        return None;
    }
    line.split_whitespace().nth(2)?.parse::<u64>().ok()
}
