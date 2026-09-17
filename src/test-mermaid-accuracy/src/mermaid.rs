// MODE: DEV
// PACKAGE: PROD

//! Reproduces mermaid-lint.awk's exact state machine (embedded as a heredoc
//! in planning/tests/test-mermaid-accuracy.sh, since CODE-STYLE.md section 3
//! caps an inline awk program at 15 lines). One pass per document: tracks
//! per-block quote/bracket balance, diagram kind, statement counts, and
//! node-id definition/reference sets, while also emitting every
//! filename-shaped token seen inside a block (consumed by references.rs).

use regex::Regex;
use std::collections::HashSet;
use std::sync::LazyLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Fail,
    Warn,
}

#[derive(Debug, Clone)]
pub struct Finding {
    pub severity: Severity,
    pub line: usize,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub line: usize,
    pub text: String,
}

pub struct ParseResult {
    pub findings: Vec<Finding>,
    pub tokens: Vec<Token>,
    pub blocks: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    None,
    Flow,
    Seq,
    State,
    Unknown,
}

struct Block {
    inq: bool,
    sb: i32,
    cb: i32,
    pr: i32,
    kind: Kind,
    stmts: u32,
    innote: bool,
    def: HashSet<String>,
    refs: Vec<(String, usize)>,
}

impl Block {
    fn new() -> Self {
        Block {
            inq: false,
            sb: 0,
            cb: 0,
            pr: 0,
            kind: Kind::None,
            stmts: 0,
            innote: false,
            def: HashSet::new(),
            refs: Vec::new(),
        }
    }

    fn add_ref(&mut self, id: &str, line: usize) {
        let id = id.trim();
        if id.is_empty() || id == "[*]" {
            return;
        }
        if self.refs.iter().any(|(existing, _)| existing == id) {
            return;
        }
        self.refs.push((id.to_string(), line));
    }

    fn ref_list(&mut self, ids: &str, line: usize) {
        for part in ids.split(',') {
            self.add_ref(part.trim(), line);
        }
    }
}

fn has_alnum(s: &str) -> bool {
    s.chars().any(|c| c.is_ascii_alphanumeric())
}

/// `sub(/[[({].*$/, "", id)`: drop from the first `[`, `(`, or `{` onward.
fn strip_bracket_suffix(s: &str) -> String {
    match s.find(['[', '(', '{']) {
        Some(pos) => s[..pos].to_string(),
        None => s.to_string(),
    }
}

macro_rules! lazy_re {
    ($name:ident, $pat:expr) => {
        static $name: LazyLock<Regex> = LazyLock::new(|| Regex::new($pat).unwrap());
    };
}

lazy_re!(FENCE_OPEN_RE, r"^```mermaid[ \t]*$");
lazy_re!(TOKEN_RE, r"[A-Za-z0-9_/*.-]+\.(sh|md|json|tsv|txt|jsonl)");
lazy_re!(FLOW_HEADER_RE, r"^(flowchart|graph)[ \t]+(TD|TB|BT|LR|RL)$");
lazy_re!(STATE_HEADER_RE, r"^stateDiagram(-v2)?$");
lazy_re!(
    FLOW_SKIP_RE,
    r"^(classDef|style|linkStyle|click|direction|end)([ \t]|$)"
);
lazy_re!(FLOW_CLASS_RE, r"^class([ \t]|$)");
lazy_re!(FLOW_SUBGRAPH_RE, r"^subgraph([ \t]|$)");
lazy_re!(EDGE_LABEL_RE, r"\|[^|]*\|");
lazy_re!(DOTTED_LABELED_RE, r"-\.[^.]*\.->");
lazy_re!(THICK_LABELED_RE, r"--[^->]*-->");
lazy_re!(FLOW_ARROW_RE, r"-\.->|-\.-|==>|===|-->|---|--x|--o|--");
lazy_re!(DEF_WITH_BRACKET_RE, r"^[A-Za-z0-9_]+[\[({]");
lazy_re!(BARE_ID_RE, r"^[A-Za-z0-9_]+$");
lazy_re!(
    SEQ_SKIP_RE,
    r"^(autonumber|end|else|and|activate|deactivate|loop|alt|opt|par|rect|box|critical|break|link|links)([ \t]|$)"
);
lazy_re!(SEQ_PARTICIPANT_RE, r"^(participant|actor)[ \t]");
lazy_re!(SEQ_NOTE_RE, r"^Note([ \t]|$)");
lazy_re!(
    SEQ_NOTE_PREFIX_RE,
    r"^Note[ \t]+(over|(right|left)[ \t]+of)[ \t]+"
);
lazy_re!(SEQ_ARROW_RE, r"-->>|->>|-->|->|--x|-x|--o|-o");
lazy_re!(STATE_END_NOTE_RE, r"^end note([ \t]|$)");
lazy_re!(STATE_NOTE_RE, r"^note[ \t]");
lazy_re!(
    STATE_NOTE_PREFIX_RE,
    r"^note[ \t]+(right|left)[ \t]+of[ \t]+"
);

fn flowchart(block: &mut Block, findings: &mut Vec<Finding>, s: &str, line: usize) {
    if FLOW_SKIP_RE.is_match(s) || s.starts_with("%%") {
        return;
    }
    if FLOW_CLASS_RE.is_match(s) {
        let fields: Vec<&str> = s.split_whitespace().collect();
        if fields.len() > 1 {
            block.ref_list(fields[1], line);
        }
        return;
    }
    if FLOW_SUBGRAPH_RE.is_match(s) {
        let fields: Vec<&str> = s.split_whitespace().collect();
        if fields.len() > 1 {
            let id = strip_bracket_suffix(fields[1]);
            if !id.is_empty() {
                block.def.insert(id);
            }
        }
        return;
    }
    let mut t = EDGE_LABEL_RE.replace_all(s, "").to_string();
    t = DOTTED_LABELED_RE.replace_all(&t, ";").to_string();
    t = THICK_LABELED_RE.replace_all(&t, ";").to_string();
    t = FLOW_ARROW_RE.replace_all(&t, ";").to_string();
    for part in t.split(';') {
        let p = part.trim();
        if p.is_empty() || !has_alnum(p) {
            continue;
        }
        block.stmts += 1;
        if DEF_WITH_BRACKET_RE.is_match(p) {
            let id = strip_bracket_suffix(p);
            block.def.insert(id);
        } else if BARE_ID_RE.is_match(p) {
            block.add_ref(p, line);
        } else {
            findings.push(Finding {
                severity: Severity::Warn,
                line,
                message: format!("unparsed flowchart fragment: {p}"),
            });
        }
    }
}

fn sequence(block: &mut Block, findings: &mut Vec<Finding>, s: &str, line: usize) {
    if SEQ_SKIP_RE.is_match(s) {
        return;
    }
    if SEQ_PARTICIPANT_RE.is_match(s) {
        let fields: Vec<&str> = s.split_whitespace().collect();
        if fields.len() > 1 {
            block.def.insert(fields[1].to_string());
        }
        return;
    }
    if SEQ_NOTE_RE.is_match(s) {
        let mut t = s.to_string();
        if let Some(pos) = t.find(':') {
            t.truncate(pos);
        }
        let t = SEQ_NOTE_PREFIX_RE.replace(&t, "").to_string();
        block.ref_list(t.trim(), line);
        return;
    }
    let mut t = s.to_string();
    if let Some(pos) = t.find(':') {
        t.truncate(pos);
    }
    let t = SEQ_ARROW_RE.replace_all(&t, ";").to_string();
    for part in t.split(';') {
        let p = part.trim();
        if p.is_empty() || !has_alnum(p) {
            continue;
        }
        block.stmts += 1;
        if BARE_ID_RE.is_match(p) {
            block.add_ref(p, line);
        } else {
            findings.push(Finding {
                severity: Severity::Warn,
                line,
                message: format!("unparsed sequence fragment: {p}"),
            });
        }
    }
}

fn state(block: &mut Block, findings: &mut Vec<Finding>, s: &str, line: usize) {
    if STATE_END_NOTE_RE.is_match(s) {
        block.innote = false;
        return;
    }
    if STATE_NOTE_RE.is_match(s) {
        block.innote = true;
        let mut t = s.to_string();
        if let Some(pos) = t.find(':') {
            t.truncate(pos);
        }
        let t = STATE_NOTE_PREFIX_RE.replace(&t, "").to_string();
        block.ref_list(t.trim(), line);
        return;
    }
    if block.innote || !s.contains("-->") {
        return;
    }
    for part in s.split("-->") {
        let mut p = part.trim().to_string();
        if let Some(pos) = p.find(':') {
            p.truncate(pos);
        }
        let p = p.trim();
        if p.is_empty() || p == "[*]" {
            continue;
        }
        block.stmts += 1;
        if BARE_ID_RE.is_match(p) {
            block.def.insert(p.to_string());
        } else {
            findings.push(Finding {
                severity: Severity::Warn,
                line,
                message: format!("unparsed state fragment: {p}"),
            });
        }
    }
}

fn finish(block: &Block, findings: &mut Vec<Finding>, start_line: usize) {
    if block.kind == Kind::None {
        findings.push(Finding {
            severity: Severity::Fail,
            line: start_line,
            message: "mermaid block declares no diagram type".into(),
        });
        return;
    }
    if block.stmts == 0 {
        findings.push(Finding {
            severity: Severity::Fail,
            line: start_line,
            message: "mermaid block has a header but no statements".into(),
        });
    }
    if block.inq {
        findings.push(Finding {
            severity: Severity::Fail,
            line: start_line,
            message: "unbalanced double quote in mermaid block".into(),
        });
    }
    if block.sb != 0 {
        findings.push(Finding {
            severity: Severity::Fail,
            line: start_line,
            message: format!("unbalanced [] in mermaid block (delta {})", block.sb),
        });
    }
    if block.cb != 0 {
        findings.push(Finding {
            severity: Severity::Fail,
            line: start_line,
            message: format!("unbalanced {{}} in mermaid block (delta {})", block.cb),
        });
    }
    if block.pr != 0 {
        findings.push(Finding {
            severity: Severity::Fail,
            line: start_line,
            message: format!("unbalanced () in mermaid block (delta {})", block.pr),
        });
    }
    for (id, refline) in &block.refs {
        if !block.def.contains(id) {
            findings.push(Finding {
                severity: Severity::Fail,
                line: *refline,
                message: format!("node id never defined: {id}"),
            });
        }
    }
}

/// Drops quoted spans (carrying the open-quote state across lines, since a
/// mermaid label may span several), and counts bracket/brace/paren balance on
/// what remains -- brackets themselves stay in the output, only quote
/// delimiters and their contents are removed.
fn strip(line: &str, block: &mut Block) -> String {
    let mut out = String::with_capacity(line.len());
    for c in line.chars() {
        if c == '"' {
            block.inq = !block.inq;
            continue;
        }
        if block.inq {
            continue;
        }
        match c {
            '[' => block.sb += 1,
            ']' => block.sb -= 1,
            '{' => block.cb += 1,
            '}' => block.cb -= 1,
            '(' => block.pr += 1,
            ')' => block.pr -= 1,
            _ => {}
        }
        out.push(c);
    }
    out
}

fn extract_tokens(line: &str, lineno: usize, tokens: &mut Vec<Token>) {
    for m in TOKEN_RE.find_iter(line) {
        tokens.push(Token {
            line: lineno,
            text: m.as_str().to_string(),
        });
    }
}

pub fn parse_document(text: &str) -> ParseResult {
    let mut findings = Vec::new();
    let mut tokens = Vec::new();
    let mut blocks = 0usize;
    let mut inblock = false;
    let mut start_line = 0usize;
    let mut block = Block::new();

    for (idx, line) in text.lines().enumerate() {
        let lineno = idx + 1;
        if !inblock && FENCE_OPEN_RE.is_match(line) {
            inblock = true;
            start_line = lineno;
            block = Block::new();
            continue;
        }
        if inblock && line.starts_with("```") {
            finish(&block, &mut findings, start_line);
            inblock = false;
            blocks += 1;
            continue;
        }
        if inblock {
            extract_tokens(line, lineno, &mut tokens);
            let stripped = strip(line, &mut block);
            let s = stripped.trim();
            if s.is_empty() {
                continue;
            }
            if block.kind == Kind::None {
                if FLOW_HEADER_RE.is_match(s) {
                    block.kind = Kind::Flow;
                } else if s == "sequenceDiagram" {
                    block.kind = Kind::Seq;
                } else if STATE_HEADER_RE.is_match(s) {
                    block.kind = Kind::State;
                } else {
                    findings.push(Finding {
                        severity: Severity::Fail,
                        line: lineno,
                        message: format!("unrecognised diagram header: {s}"),
                    });
                    block.kind = Kind::Unknown;
                }
                continue;
            }
            match block.kind {
                Kind::Flow => flowchart(&mut block, &mut findings, s, lineno),
                Kind::Seq => sequence(&mut block, &mut findings, s, lineno),
                Kind::State => state(&mut block, &mut findings, s, lineno),
                Kind::None | Kind::Unknown => {}
            }
        }
    }
    if inblock {
        findings.push(Finding {
            severity: Severity::Fail,
            line: start_line,
            message: "unterminated mermaid fence".into(),
        });
    }
    ParseResult {
        findings,
        tokens,
        blocks,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fails(result: &ParseResult) -> Vec<&str> {
        result
            .findings
            .iter()
            .filter(|f| f.severity == Severity::Fail)
            .map(|f| f.message.as_str())
            .collect()
    }

    fn warns(result: &ParseResult) -> Vec<&str> {
        result
            .findings
            .iter()
            .filter(|f| f.severity == Severity::Warn)
            .map(|f| f.message.as_str())
            .collect()
    }

    #[test]
    fn a_clean_flowchart_parses_with_no_findings() {
        // Bare mentions register only as references, never definitions
        // (matching the awk original exactly): a labeled form is required
        // somewhere for a node id to count as defined.
        let doc = "```mermaid\nflowchart TD\n    A[Start]-->B[End]\n```\n";
        let result = parse_document(doc);
        assert!(result.findings.is_empty(), "{:?}", result.findings);
        assert_eq!(result.blocks, 1);
    }

    #[test]
    fn a_clean_sequence_diagram_parses_with_no_findings() {
        let doc = "```mermaid\nsequenceDiagram\n    participant A\n    participant B\n    A->>B: hi\n```\n";
        let result = parse_document(doc);
        assert!(result.findings.is_empty(), "{:?}", result.findings);
    }

    #[test]
    fn a_clean_state_diagram_parses_with_no_findings() {
        let doc = "```mermaid\nstateDiagram-v2\n    A --> B\n```\n";
        let result = parse_document(doc);
        assert!(result.findings.is_empty(), "{:?}", result.findings);
    }

    #[test]
    fn unrecognised_header_fails() {
        let doc = "```mermaid\nbogusDiagram\n    A-->B\n```\n";
        let result = parse_document(doc);
        assert!(fails(&result)
            .iter()
            .any(|m| m.contains("unrecognised diagram header")));
    }

    #[test]
    fn unbalanced_bracket_fails_with_signed_delta() {
        let doc = "```mermaid\nflowchart TD\n    A[label\n```\n";
        let result = parse_document(doc);
        assert!(fails(&result)
            .iter()
            .any(|m| m.contains("unbalanced [] in mermaid block (delta 1)")));
    }

    #[test]
    fn an_undefined_referenced_node_fails() {
        let doc = "```mermaid\nflowchart TD\n    A-->B\n```\n";
        let result = parse_document(doc);
        assert!(fails(&result)
            .iter()
            .any(|m| m.contains("node id never defined: A")));
        assert!(fails(&result)
            .iter()
            .any(|m| m.contains("node id never defined: B")));
    }

    #[test]
    fn a_defined_node_is_not_flagged() {
        let doc = "```mermaid\nflowchart TD\n    A[Start]-->B[End]\n```\n";
        let result = parse_document(doc);
        assert!(result.findings.is_empty(), "{:?}", result.findings);
    }

    #[test]
    fn blocks_cross_check_catches_a_silently_skipped_block() {
        // A block with no recognisable content at all (blank body) still
        // closes and counts toward `blocks`, matching the bash original.
        let doc = "```mermaid\n```\n";
        let result = parse_document(doc);
        assert_eq!(result.blocks, 1);
        assert!(fails(&result)
            .iter()
            .any(|m| m.contains("declares no diagram type")));
    }

    #[test]
    fn an_unterminated_fence_fails() {
        let doc = "```mermaid\nflowchart TD\n    A-->B\n";
        let result = parse_document(doc);
        assert!(fails(&result)
            .iter()
            .any(|m| m.contains("unterminated mermaid fence")));
        assert_eq!(result.blocks, 0);
    }

    #[test]
    fn an_unparsed_flowchart_fragment_warns() {
        let doc = "```mermaid\nflowchart TD\n    !!!not-a-node!!!\n```\n";
        let result = parse_document(doc);
        assert!(warns(&result)
            .iter()
            .any(|m| m.starts_with("unparsed flowchart fragment:")));
    }

    #[test]
    fn an_unparsed_sequence_fragment_warns() {
        let doc = "```mermaid\nsequenceDiagram\n    !!!bad!!!\n```\n";
        let result = parse_document(doc);
        assert!(warns(&result)
            .iter()
            .any(|m| m.starts_with("unparsed sequence fragment:")));
    }

    #[test]
    fn an_unparsed_state_fragment_warns() {
        let doc = "```mermaid\nstateDiagram-v2\n    A --> !!!bad!!!\n```\n";
        let result = parse_document(doc);
        assert!(warns(&result)
            .iter()
            .any(|m| m.starts_with("unparsed state fragment:")));
    }

    #[test]
    fn sequence_note_lines_are_absorbed_without_incrementing_statements() {
        let doc = "```mermaid\nsequenceDiagram\n    participant A\n    participant B\n    A->>B: hi\n    Note over A,B: a note\n```\n";
        let result = parse_document(doc);
        assert!(result.findings.is_empty(), "{:?}", result.findings);
    }

    #[test]
    fn state_note_blocks_are_skipped_until_end_note() {
        let doc = "```mermaid\nstateDiagram-v2\n    A --> B\n    note right of B\n    anything at all, not a transition\n    end note\n```\n";
        let result = parse_document(doc);
        assert!(result.findings.is_empty(), "{:?}", result.findings);
    }

    #[test]
    fn a_hyphen_led_token_is_extracted() {
        let doc = "```mermaid\nflowchart TD\n    A[-testing.md]\n```\n";
        let result = parse_document(doc);
        assert!(result.tokens.iter().any(|t| t.text == "-testing.md"));
    }

    #[test]
    fn cumulative_finish_checks_can_all_fire_on_one_block() {
        let doc = "```mermaid\nbogusHeader\n    A[label\n```\n";
        let result = parse_document(doc);
        let messages = fails(&result);
        assert!(messages
            .iter()
            .any(|m| m.contains("unrecognised diagram header")));
        assert!(messages
            .iter()
            .any(|m| m.contains("unbalanced [] in mermaid block")));
    }
}
