// MODE: DEV
// PACKAGE: PROD
//! Shell text into simple commands, each knowing its redirects and what feeds it.
//! Not a shell grammar: only what the gate's checks need to read.

use super::resolve;

// Longest first: the lexer takes the first one that matches.
const OPERATORS: &[&str] = &[
    "&>>", "<<<", "<<-", "||", "&&", ";;", "|&", "&>", ">>", ">&", "<<", "<&", "<>", ";", "(", ")",
    "<", ">", "|", "&",
];

#[derive(Debug, PartialEq, Clone)]
enum Token {
    Word(String),
    Op(&'static str),
    /// The digits glued to the front of a redirect, as in `2>`.
    Fd(String),
}

#[derive(Default)]
struct Lexer {
    chars: Vec<char>,
    i: usize,
    tokens: Vec<Token>,
    word: String,
    in_word: bool,
    quoted: bool,
}

impl Lexer {
    fn flush(&mut self) {
        if self.in_word {
            self.tokens
                .push(Token::Word(std::mem::take(&mut self.word)));
        }
        self.in_word = false;
        self.quoted = false;
    }

    fn single_quoted(&mut self) -> Option<()> {
        self.i += 1;
        while *self.chars.get(self.i)? != '\'' {
            self.word.push(self.chars[self.i]);
            self.i += 1;
        }
        self.close_quote();
        Some(())
    }

    fn double_quoted(&mut self) -> Option<()> {
        self.i += 1;
        while *self.chars.get(self.i)? != '"' {
            if self.chars[self.i] == '\\' {
                self.i += 1;
            }
            self.word.push(*self.chars.get(self.i)?);
            self.i += 1;
        }
        self.close_quote();
        Some(())
    }

    fn close_quote(&mut self) {
        self.in_word = true;
        self.quoted = true;
        self.i += 1;
    }

    fn escape(&mut self) {
        match self.chars.get(self.i + 1) {
            Some('\n') | None => {}
            Some(next) => {
                self.word.push(*next);
                self.in_word = true;
            }
        }
        self.i += 2;
    }

    fn comment(&mut self) {
        while self.i < self.chars.len() && self.chars[self.i] != '\n' {
            self.i += 1;
        }
    }

    fn separator(&mut self, c: char) {
        self.flush();
        if c == '\n' || c == '`' {
            self.tokens.push(Token::Op(";"));
        }
        self.i += 1;
    }

    fn operator_or_char(&mut self, c: char) {
        let end = self.chars.len().min(self.i + 3);
        let rest: String = self.chars[self.i..end].iter().collect();
        let Some(op) = OPERATORS.iter().find(|op| rest.starts_with(**op)) else {
            self.word.push(c);
            self.in_word = true;
            self.i += 1;
            return;
        };
        let digits = self.in_word && self.word.chars().all(|d| d.is_ascii_digit());
        if !self.quoted && digits && (op.starts_with('<') || op.starts_with('>')) {
            self.tokens.push(Token::Fd(std::mem::take(&mut self.word)));
            self.in_word = false;
        }
        self.flush();
        self.tokens.push(Token::Op(op));
        self.i += op.len();
    }

    /// Reads everything. `None` means an unterminated quote, which the shell
    /// refuses too, so nothing would run.
    fn run(mut self) -> Option<Vec<Token>> {
        while self.i < self.chars.len() {
            match self.chars[self.i] {
                '\'' => self.single_quoted()?,
                '"' => self.double_quoted()?,
                '\\' => self.escape(),
                '#' if !self.in_word => self.comment(),
                c @ (' ' | '\t' | '\r' | '\n' | '`') => self.separator(c),
                c => self.operator_or_char(c),
            }
        }
        self.flush();
        Some(self.tokens)
    }
}

fn tokenise(command: &str) -> Option<Vec<Token>> {
    let lexer = Lexer {
        chars: command.chars().collect(),
        ..Lexer::default()
    };
    lexer.run()
}

/// The here-document delimiters a line opens. A digit-led word is not one, so a
/// shift such as `$(( 1 << 2 ))` swallows nothing.
fn heredoc_delimiters(line: &str) -> Vec<String> {
    let chars: Vec<char> = line.chars().collect();
    let mut found = Vec::new();
    let mut i = 0;
    while i + 1 < chars.len() {
        if chars[i] != '<' || chars[i + 1] != '<' {
            i += 1;
            continue;
        }
        if chars.get(i + 2) == Some(&'<') {
            i += 3;
            continue;
        }
        let mut j = i + 2;
        if chars.get(j) == Some(&'-') {
            j += 1;
        }
        while matches!(chars.get(j), Some(' ' | '\t' | '\\' | '\'' | '"')) {
            j += 1;
        }
        let start = j;
        while chars
            .get(j)
            .is_some_and(|c| c.is_ascii_alphanumeric() || *c == '_')
        {
            j += 1;
        }
        if j > start && !chars[start].is_ascii_digit() {
            found.push(chars[start..j].iter().collect());
        }
        i = j.max(i + 2);
    }
    found
}

/// The command with here-document bodies removed, and those bodies in order.
fn split_heredocs(command: &str) -> (String, Vec<String>) {
    let lines: Vec<&str> = command.split('\n').collect();
    let mut kept = Vec::new();
    let mut bodies = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        kept.push(lines[i]);
        let delimiters = heredoc_delimiters(lines[i]);
        i += 1;
        for delimiter in delimiters {
            let start = i;
            while i < lines.len() && lines[i].trim() != delimiter {
                i += 1;
            }
            bodies.push(lines[start..i].join("\n"));
            i += 1;
        }
    }
    (kept.join("\n"), bodies)
}

#[derive(Default, Debug)]
pub(super) struct Segment {
    pub(super) words: Vec<String>,
    pub(super) stdin_file: Option<String>,
    pub(super) heredoc: Option<String>,
    pub(super) out_targets: Vec<String>,
    pub(super) upstream: Option<usize>,
    pub(super) cwd: String,
}

impl Segment {
    fn is_filled(&self) -> bool {
        !self.words.is_empty() || !self.out_targets.is_empty() || self.stdin_file.is_some()
    }
}

struct Builder {
    segments: Vec<Segment>,
    current: Segment,
    fd: Option<String>,
    bodies: std::vec::IntoIter<String>,
}

impl Builder {
    fn finish(&mut self, piped: bool) {
        let mut next = Segment::default();
        if self.current.is_filled() {
            self.segments.push(std::mem::take(&mut self.current));
            next.upstream = piped.then(|| self.segments.len() - 1);
        }
        self.current = next;
    }

    /// Applies one operator, and reports whether it used the word after it.
    fn apply(&mut self, op: &str, next_word: Option<String>) -> bool {
        let has_word = next_word.is_some();
        match op {
            "|" | "|&" | "||" | "&&" | ";" | ";;" | "&" | "(" | ")" => {
                self.finish(matches!(op, "|" | "|&"));
                false
            }
            "<<" | "<<-" => {
                self.current.heredoc = self.bodies.next();
                has_word
            }
            "<<<" | "<&" | "<>" => has_word,
            "<" => {
                self.current.stdin_file = next_word.or(self.current.stdin_file.take());
                has_word
            }
            _ => {
                let to_stdout = self.fd.as_deref().is_none_or(|digits| digits == "1");
                if let Some(target) = next_word.filter(|_| to_stdout) {
                    if !target.chars().all(|d| d.is_ascii_digit()) {
                        self.current.out_targets.push(target);
                    }
                }
                has_word
            }
        }
    }
}

/// Each simple command in order. `None` means the text does not lex.
pub(super) fn segments_of(command: &str, cwd: &str) -> Option<Vec<Segment>> {
    let (stripped, bodies) = split_heredocs(command);
    let tokens = tokenise(&stripped)?;
    let mut builder = Builder {
        segments: Vec::new(),
        current: Segment::default(),
        fd: None,
        bodies: bodies.into_iter(),
    };
    let mut i = 0;
    while i < tokens.len() {
        i += 1;
        match &tokens[i - 1] {
            Token::Word(word) => builder.current.words.push(word.clone()),
            Token::Fd(digits) => builder.fd = Some(digits.clone()),
            Token::Op(op) => {
                let next_word = match tokens.get(i) {
                    Some(Token::Word(word)) => Some(word.clone()),
                    _ => None,
                };
                i += usize::from(builder.apply(op, next_word));
                builder.fd = None;
            }
        }
    }
    builder.finish(false);
    assign_directories(&mut builder.segments, cwd);
    Some(builder.segments)
}

/// Each command's working directory, following any `cd` before it.
fn assign_directories(segments: &mut [Segment], cwd: &str) {
    let mut dir = cwd.to_string();
    for segment in segments {
        segment.cwd = dir.clone();
        if segment.words.first().map(String::as_str) == Some("cd") {
            if let Some(target) = segment.words.get(1) {
                dir = resolve(&dir, target).display().to_string();
            }
        }
    }
}
