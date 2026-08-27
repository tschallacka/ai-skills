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
            while i < lines.len() && lines[i].trim() != delimiter {
                i += 1;
            }
            if i < lines.len() {
                i += 1; // drop the closing delimiter too
            }
        }
    }
    out.join("\n")
}

/// Delimiters introduced on one line: `<<EOF`, `<<-EOF`, `<<'EOF'`, `<<"EOF"`.
fn heredoc_delimiters(line: &str) -> Vec<String> {
    let bytes: Vec<char> = line.chars().collect();
    let mut found = Vec::new();
    let mut i = 0;
    while i + 1 < bytes.len() {
        if bytes[i] == '<' && bytes[i + 1] == '<' {
            let mut j = i + 2;
            if j < bytes.len() && bytes[j] == '-' {
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
                found.push(bytes[start..j].iter().collect::<String>());
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
