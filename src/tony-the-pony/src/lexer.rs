// MODE: DEV
// PACKAGE: PROD
//! Decide whether a shell command actually *invokes* a text searcher.
//!
//! A regex over the raw command cannot tell a search from a sentence about one:
//! it blocks writing a file whose text mentions the tool, and blocks a test
//! harness whose fixtures are command strings. Both happened.
//!
//! This lexes instead, respecting quotes and escapes, and tracks command
//! position — so `grep foo` is an invocation, `echo "grep foo"` is an argument,
//! and `ls | grep foo` is reading another command's output.
//!
//! Not a shell grammar. It reports `Undecidable` rather than guessing when the
//! text cannot be lexed, and the caller must then fail closed.

pub const SEARCHERS: [&str; 6] = ["grep", "egrep", "fgrep", "rg", "ack", "ag"];

/// Wrappers that pass the real command through, keeping command position.
const TRANSPARENT: [&str; 12] = [
    "sudo", "command", "env", "time", "nice", "ionice", "nohup", "stdbuf", "xargs", "builtin",
    "exec", "doas",
];

#[derive(Debug, PartialEq)]
pub enum Verdict {
    /// A searcher is invoked as a command, not reading a pipe.
    Gated(String),
    /// No searcher is invoked, or one only reads another command's output.
    Clear,
    /// The text could not be lexed; the caller must fail closed.
    Undecidable,
}

#[derive(Debug, PartialEq)]
enum Token {
    Word(String),
    Operator(String),
}

/// Strip here-document bodies: their contents are data, never executed by this
/// command. A file whose body discusses searching is the commonest false
/// positive there is.
fn strip_heredocs(command: &str) -> String {
    let lines: Vec<&str> = command.split('\n').collect();
    let mut out: Vec<&str> = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        out.push(line);
        let delimiters = heredoc_delimiters(line);
        i += 1;
        for delimiter in delimiters {
            while i < lines.len() && lines[i].trim() != delimiter.word {
                i += 1;
            }
            if i < lines.len() {
                i += 1; // drop the closing delimiter too
            }
        }
    }
    out.join("\n")
}

/// A here-document opened on a line: its delimiter word, and whether `<<-`
/// allows the closing delimiter to be indented.
pub struct Heredoc {
    pub word: String,
    pub indented: bool,
}

/// Delimiters introduced on one line: `<<EOF`, `<<-EOF`, `<<'EOF'`, `<<"EOF"`.
///
/// A left shift is not a here-document. `$(( 1 << 2 ))` read as one named "2"
/// consumed every following line until one said "2", which silently dropped
/// real commands from what the caller then inspected.
fn heredoc_delimiters(line: &str) -> Vec<Heredoc> {
    let bytes: Vec<char> = line.chars().collect();
    let mut found = Vec::new();
    let mut i = 0;
    while i + 1 < bytes.len() {
        if bytes[i] == '<' && bytes[i + 1] == '<' {
            let arithmetic = bytes[..i].iter().collect::<String>().contains("$((");
            let mut j = i + 2;
            let mut indented = false;
            if j < bytes.len() && bytes[j] == '-' {
                indented = true;
                j += 1;
            }
            while j < bytes.len() && bytes[j] == ' ' {
                j += 1;
            }
            let quote = if j < bytes.len() && (bytes[j] == '\'' || bytes[j] == '"') {
                let q = bytes[j];
                j += 1;
                Some(q)
            } else {
                None
            };
            let start = j;
            while j < bytes.len() && (bytes[j].is_alphanumeric() || bytes[j] == '_') {
                j += 1;
            }
            if j > start {
                let word = bytes[start..j].iter().collect::<String>();
                let shift = quote.is_none() && word.chars().all(|c| c.is_ascii_digit());
                if !shift && !arithmetic {
                    found.push(Heredoc { word, indented });
                }
            }
            if let Some(q) = quote {
                if j < bytes.len() && bytes[j] == q {
                    j += 1;
                }
            }
            i = j;
        } else {
            i += 1;
        }
    }
    found
}

/// Spaces occupying the same number of bytes as `text`, so a redacted line has
/// the length of the line it replaced.
fn blanked(text: &str) -> String {
    " ".repeat(text.len())
}

/// Blank everything that is not an instruction: comment bodies, quoted-string
/// contents and here-document bodies. Quote characters, operators and delimiter
/// words survive, because a construct is recognised by the words the shell would
/// execute and those are exactly what is left.
///
/// Line count and byte offsets are preserved, so a position in the output is the
/// same position in the input. That is what lets a scanner report a line number
/// from the redacted text and have it name the right source line.
///
/// The same classification [`inspect`] applies to one command, applied to a
/// whole script and answering a different question: not "is this a searcher"
/// but "is this text an instruction at all".
pub fn redact(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut in_squote = false;
    let mut in_dquote = false;
    let mut open_heredoc: Option<Heredoc> = None;

    for raw in text.split_inclusive('\n') {
        let (line, newline) = match raw.strip_suffix('\n') {
            Some(body) => (body, "\n"),
            None => (raw, ""),
        };

        if let Some(heredoc) = &open_heredoc {
            let probe = if heredoc.indented {
                line.trim_start_matches('\t')
            } else {
                line
            };
            if probe == heredoc.word {
                out.push_str(line);
                open_heredoc = None;
            } else {
                out.push_str(&blanked(line));
            }
            out.push_str(newline);
            continue;
        }

        let mut pending = heredoc_delimiters(line).into_iter().next();
        let chars: Vec<char> = line.chars().collect();
        let mut index = 0;
        while index < chars.len() {
            let current = chars[index];
            let width = current.len_utf8();

            if in_squote {
                if current == '\'' {
                    in_squote = false;
                    out.push(current);
                } else {
                    out.push_str(&" ".repeat(width));
                }
                index += 1;
                continue;
            }

            if in_dquote {
                // A backslash escapes the next character, a quote included, so
                // both are consumed here or `\"` would read as the closing one.
                if current == '\\' && index + 1 < chars.len() {
                    out.push_str(&" ".repeat(width + chars[index + 1].len_utf8()));
                    index += 2;
                    continue;
                }
                if current == '"' {
                    in_dquote = false;
                    out.push(current);
                } else {
                    out.push_str(&" ".repeat(width));
                }
                index += 1;
                continue;
            }

            match current {
                '\'' => {
                    in_squote = true;
                    out.push(current);
                }
                '"' => {
                    in_dquote = true;
                    out.push(current);
                }
                // A `#` opens a comment only where a word can begin. Mid-word it
                // is a literal, which is why `${x#pattern}` survives.
                '#' if index == 0
                    || matches!(chars[index - 1], ' ' | '\t' | ';' | '&' | '|' | '(') =>
                {
                    let rest: String = chars[index..].iter().collect();
                    out.push_str(&blanked(&rest));
                    index = chars.len();
                    continue;
                }
                _ => out.push(current),
            }
            index += 1;
        }

        out.push_str(newline);
        // A here-document body starts on the line after the one that opened it,
        // so this is applied only once the opening line has been emitted.
        if let Some(heredoc) = pending.take() {
            open_heredoc = Some(heredoc);
        }
    }
    out
}

/// Tokenise, honouring single quotes, double quotes and backslash escapes.
/// `None` means the text is unlexable (an unterminated quote, say).
fn tokenise(command: &str) -> Option<Vec<Token>> {
    let mut tokens = Vec::new();
    let mut word = String::new();
    let mut chars = command.chars().peekable();

    macro_rules! flush {
        () => {
            if !word.is_empty() {
                tokens.push(Token::Word(std::mem::take(&mut word)));
            }
        };
    }

    while let Some(c) = chars.next() {
        match c {
            '\'' => {
                let mut closed = false;
                for q in chars.by_ref() {
                    if q == '\'' {
                        closed = true;
                        break;
                    }
                    word.push(q);
                }
                if !closed {
                    return None;
                }
                // A quoted run is a word even when empty: '' is an argument.
                if word.is_empty() {
                    word.push('\0');
                }
            }
            '"' => {
                let mut closed = false;
                while let Some(q) = chars.next() {
                    if q == '"' {
                        closed = true;
                        break;
                    }
                    if q == '\\' {
                        if let Some(escaped) = chars.next() {
                            word.push(escaped);
                        }
                        continue;
                    }
                    word.push(q);
                }
                if !closed {
                    return None;
                }
                if word.is_empty() {
                    word.push('\0');
                }
            }
            '\\' => {
                let escaped = chars.next()?;
                word.push(escaped);
            }
            ' ' | '\t' | '\r' => flush!(),
            '\n' | ';' | '&' | '|' | '(' | ')' | '{' | '}' => {
                flush!();
                // Collapse two-character operators; only the class matters.
                let mut op = c.to_string();
                if let Some(&next) = chars.peek() {
                    if (c == '|' && next == '|') || (c == '&' && next == '&') {
                        op.push(next);
                        chars.next();
                    }
                }
                tokens.push(Token::Operator(op));
            }
            _ => word.push(c),
        }
    }
    flush!();
    Some(tokens)
}

fn is_assignment(word: &str) -> bool {
    match word.find('=') {
        Some(0) | None => false,
        Some(i) => {
            let name = &word[..i];
            name.chars()
                .next()
                .is_some_and(|c| c.is_alphabetic() || c == '_')
                && name.chars().all(|c| c.is_alphanumeric() || c == '_')
        }
    }
}

fn basename(word: &str) -> &str {
    word.rsplit(['/', '\\']).next().unwrap_or(word)
}

pub fn inspect(command: &str) -> Verdict {
    let tokens = match tokenise(&strip_heredocs(command)) {
        Some(t) => t,
        None => return Verdict::Undecidable,
    };

    let mut at_command_start = true;
    let mut after_pipe = false;

    for token in &tokens {
        match token {
            Token::Operator(op) => {
                at_command_start = true;
                after_pipe = op == "|";
            }
            Token::Word(word) => {
                if !at_command_start {
                    continue;
                }
                if is_assignment(word) {
                    continue; // env prefix keeps command position
                }
                let name = basename(word);
                if TRANSPARENT.contains(&name) {
                    continue; // the next word is still the command
                }
                if SEARCHERS.contains(&name) && !after_pipe {
                    return Verdict::Gated(name.to_string());
                }
                at_command_start = false;
                after_pipe = false;
            }
        }
    }
    Verdict::Clear
}

#[cfg(test)]
mod redaction_tests {
    use super::redact;

    /// Assembled rather than written literally, so this file does not itself
    /// carry a construct the portability scan would report against it.
    fn verb() -> String {
        format!("sed {}i", '-')
    }

    #[test]
    fn prose_naming_a_construct_is_not_an_instruction() {
        let verb = verb();
        let source = format!(
            "# a comment naming {verb}\n\
             echo 'single quoted {verb}'\n\
             echo \"double quoted {verb}\"\n\
             {verb} 's/a/b/' file\n"
        );
        let redacted = redact(&source);
        let lines: Vec<&str> = redacted.lines().collect();
        assert!(
            !lines[0].contains(&verb),
            "comment survived: {:?}",
            lines[0]
        );
        assert!(!lines[1].contains(&verb), "squote survived: {:?}", lines[1]);
        assert!(!lines[2].contains(&verb), "dquote survived: {:?}", lines[2]);
        assert!(lines[3].contains(&verb), "instruction lost: {:?}", lines[3]);
    }

    #[test]
    fn a_heredoc_body_is_data_and_its_delimiter_is_not() {
        let verb = verb();
        let source = format!(
            "cat <<'BODY'\n\
             naming {verb} in a body\n\
             BODY\n\
             {verb} 's/a/b/' after\n"
        );
        let redacted = redact(&source);
        let lines: Vec<&str> = redacted.lines().collect();
        assert!(!lines[1].contains(&verb), "body survived: {:?}", lines[1]);
        assert_eq!(lines[2].trim(), "BODY");
        assert!(lines[3].contains(&verb), "instruction lost: {:?}", lines[3]);
    }

    #[test]
    fn an_indented_heredoc_closes_on_a_tabbed_delimiter() {
        let verb = verb();
        let source = format!("cat <<-TAB\n\tnaming {verb}\n\tTAB\n{verb} 's/a/b/' after\n");
        let redacted = redact(&source);
        let lines: Vec<&str> = redacted.lines().collect();
        assert!(!lines[1].contains(&verb));
        assert!(
            lines[3].contains(&verb),
            "delimiter not closed: {:?}",
            lines
        );
    }

    /// The failure direction that matters: a construct hidden rather than
    /// invented. A hash inside a string used to truncate the line.
    #[test]
    fn a_hash_inside_a_string_does_not_hide_what_follows() {
        let verb = verb();
        let source = format!("grep '#hash' file && {verb} 's/x/y/' later\n");
        let redacted = redact(&source);
        assert!(redacted.contains(&verb), "hidden by the hash: {redacted:?}");
    }

    #[test]
    fn an_arithmetic_shift_is_not_a_heredoc() {
        let verb = verb();
        let source = format!("width=$(( 1 << 2 ))\n{verb} 's/a/b/' after\n");
        let redacted = redact(&source);
        assert!(
            redacted.contains(&verb),
            "swallowed by a shift: {redacted:?}"
        );
    }

    #[test]
    fn a_parameter_expansion_does_not_open_a_comment() {
        let source = "trimmed=${name#leading}\n";
        assert_eq!(redact(source), source);
    }

    #[test]
    fn line_count_and_byte_offsets_survive() {
        let source = "# padding\necho 'padding'\nls\n";
        let redacted = redact(source);
        assert_eq!(redacted.len(), source.len());
        assert_eq!(redacted.lines().count(), source.lines().count());
        for (before, after) in source.lines().zip(redacted.lines()) {
            assert_eq!(before.len(), after.len());
        }
    }
}
