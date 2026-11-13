// MODE: DEV
// PACKAGE: PROD
//! JSON and JSONL addressing for `read` and `replace`. A value is named by an
//! RFC 6901 pointer or by a `path` whose steps may also match array elements
//! by their fields (`{"id": "abc"}`), and resolves to the exact byte span the
//! value occupies. An edit splices only that span -- key order, spacing,
//! number spelling and every untouched value stay byte for byte -- and the
//! result must parse again or the edit is refused. JSONL is addressed by line
//! first, and only that line is parsed.

use serde_json::Value;

pub type Refusal = (&'static str, String);

/// Files whose own tool owns their JSON; a structured write is refused so it
/// cannot become the way around that tool.
pub const OWNED_FILES: &[(&str, &str)] = &[
    ("BUGS.json", "bugs"),
    ("TODO.json", "todo"),
    ("DECISIONS.json", "decisions"),
];

#[derive(Debug, Clone)]
pub enum Kind {
    Object(Vec<Member>),
    Array(Vec<Node>),
    Scalar,
}

/// A JSON value and its byte span in the document.
#[derive(Debug, Clone)]
pub struct Node {
    pub start: usize,
    pub end: usize,
    pub kind: Kind,
}

#[derive(Debug, Clone)]
pub struct Member {
    pub key: String,
    pub key_start: usize,
    pub key_end: usize,
    pub value: Node,
}

struct Parser<'a> {
    bytes: &'a [u8],
    pos: usize,
    end: usize,
}

fn malformed(at: usize, what: &str) -> Refusal {
    ("json_malformed", format!("{what} at byte {at}"))
}

impl Parser<'_> {
    fn skip_ws(&mut self) {
        while self.pos < self.end && matches!(self.bytes[self.pos], b' ' | b'\t' | b'\r' | b'\n') {
            self.pos += 1;
        }
    }

    fn peek(&self) -> Option<u8> {
        (self.pos < self.end).then(|| self.bytes[self.pos])
    }

    fn string_end(&mut self) -> Result<usize, Refusal> {
        let start = self.pos;
        self.pos += 1;
        while self.pos < self.end {
            match self.bytes[self.pos] {
                b'\\' => self.pos += 2,
                b'"' => {
                    self.pos += 1;
                    return Ok(self.pos);
                }
                _ => self.pos += 1,
            }
        }
        Err(malformed(start, "an unterminated string"))
    }

    fn value(&mut self) -> Result<Node, Refusal> {
        self.skip_ws();
        let start = self.pos;
        match self.peek() {
            None => Err(malformed(start, "a value was expected")),
            Some(b'{') => {
                self.pos += 1;
                let mut members = Vec::new();
                self.skip_ws();
                if self.peek() == Some(b'}') {
                    self.pos += 1;
                } else {
                    loop {
                        self.skip_ws();
                        if self.peek() != Some(b'"') {
                            return Err(malformed(self.pos, "an object key was expected"));
                        }
                        let key_start = self.pos;
                        let key_end = self.string_end()?;
                        let key: String = serde_json::from_slice(&self.bytes[key_start..key_end])
                            .map_err(|_| malformed(key_start, "an invalid key"))?;
                        self.skip_ws();
                        if self.peek() != Some(b':') {
                            return Err(malformed(self.pos, "':' was expected"));
                        }
                        self.pos += 1;
                        let value = self.value()?;
                        members.push(Member {
                            key,
                            key_start,
                            key_end,
                            value,
                        });
                        self.skip_ws();
                        match self.peek() {
                            Some(b',') => self.pos += 1,
                            Some(b'}') => {
                                self.pos += 1;
                                break;
                            }
                            _ => return Err(malformed(self.pos, "',' or '}' was expected")),
                        }
                    }
                }
                Ok(Node {
                    start,
                    end: self.pos,
                    kind: Kind::Object(members),
                })
            }
            Some(b'[') => {
                self.pos += 1;
                let mut items = Vec::new();
                self.skip_ws();
                if self.peek() == Some(b']') {
                    self.pos += 1;
                } else {
                    loop {
                        items.push(self.value()?);
                        self.skip_ws();
                        match self.peek() {
                            Some(b',') => self.pos += 1,
                            Some(b']') => {
                                self.pos += 1;
                                break;
                            }
                            _ => return Err(malformed(self.pos, "',' or ']' was expected")),
                        }
                    }
                }
                Ok(Node {
                    start,
                    end: self.pos,
                    kind: Kind::Array(items),
                })
            }
            Some(b'"') => {
                let end = self.string_end()?;
                Ok(Node {
                    start,
                    end,
                    kind: Kind::Scalar,
                })
            }
            Some(_) => {
                while self.pos < self.end
                    && !matches!(
                        self.bytes[self.pos],
                        b',' | b'}' | b']' | b' ' | b'\t' | b'\r' | b'\n'
                    )
                {
                    self.pos += 1;
                }
                serde_json::from_slice::<Value>(&self.bytes[start..self.pos])
                    .map_err(|_| malformed(start, "an invalid literal"))?;
                Ok(Node {
                    start,
                    end: self.pos,
                    kind: Kind::Scalar,
                })
            }
        }
    }
}

/// Parses `bytes[start..end]` as exactly one JSON value (whitespace around
/// it allowed), keeping every value's span in document coordinates.
pub fn parse(bytes: &[u8], start: usize, end: usize) -> Result<Node, Refusal> {
    let mut parser = Parser {
        bytes,
        pos: start,
        end,
    };
    let node = parser.value()?;
    parser.skip_ws();
    if parser.pos != end {
        return Err(malformed(parser.pos, "trailing content after the value"));
    }
    Ok(node)
}

/// One step of a path: an object key, an array index (or `-`, one past the
/// end), or a match on an array element's fields.
#[derive(Debug, Clone)]
enum Step {
    Key(String),
    Index(usize),
    End,
    Where(serde_json::Map<String, Value>),
}

fn steps(address: &serde_json::Map<String, Value>) -> Result<Vec<Step>, Refusal> {
    match (address.get("pointer"), address.get("path")) {
        (Some(_), Some(_)) => Err((
            "json_address_invalid",
            "name the value by pointer or by path, not both".into(),
        )),
        (Some(Value::String(pointer)), None) => {
            if pointer.is_empty() {
                return Ok(Vec::new());
            }
            if !pointer.starts_with('/') {
                return Err((
                    "json_address_invalid",
                    format!("a JSON pointer starts with '/', e.g. /orders/3/qty; got {pointer:?}"),
                ));
            }
            Ok(pointer[1..]
                .split('/')
                .map(|token| {
                    let token = token.replace("~1", "/").replace("~0", "~");
                    if token == "-" {
                        Step::End
                    } else {
                        Step::Key(token)
                    }
                })
                .collect())
        }
        (None, Some(Value::Array(path))) => path
            .iter()
            .map(|step| match step {
                Value::String(key) if key == "-" => Ok(Step::End),
                Value::String(key) => Ok(Step::Key(key.clone())),
                Value::Number(number) => number
                    .as_u64()
                    .map(|index| Step::Index(index as usize))
                    .ok_or((
                        "json_address_invalid",
                        "an array index is a whole number from 0".to_owned(),
                    )),
                Value::Object(conditions) if !conditions.is_empty() => {
                    Ok(Step::Where(conditions.clone()))
                }
                _ => Err((
                    "json_address_invalid",
                    "a path step is a key, an index, \"-\", or a {field: value} match".to_owned(),
                )),
            })
            .collect(),
        (None, None) => Ok(Vec::new()),
        _ => Err((
            "json_address_invalid",
            "pointer is a string such as /orders/3/qty; path is a list of steps".into(),
        )),
    }
}

fn escape_token(key: &str) -> String {
    key.replace('~', "~0").replace('/', "~1")
}

fn node_value(bytes: &[u8], node: &Node) -> Value {
    serde_json::from_slice(&bytes[node.start..node.end]).unwrap_or(Value::Null)
}

fn matches_where(bytes: &[u8], node: &Node, conditions: &serde_json::Map<String, Value>) -> bool {
    let Kind::Object(members) = &node.kind else {
        return false;
    };
    conditions.iter().all(|(field, wanted)| {
        members.iter().any(|member| {
            member.key == *field && {
                let actual = node_value(bytes, &member.value);
                actual == *wanted
                    || matches!((&actual, wanted), (Value::Number(n), Value::String(s)) if n.to_string() == *s)
            }
        })
    })
}

/// Where a step leads: an existing value, or (for insert) a position in a
/// container that does not hold a value yet.
enum Found<'a> {
    Value(&'a Node, Option<(&'a Node, usize)>),
    Slot(&'a Node, Step),
}

fn walk<'a>(bytes: &[u8], root: &'a Node, path: &[Step]) -> Result<(Found<'a>, String), Refusal> {
    let mut node = root;
    let mut parent: Option<(&Node, usize)> = None;
    let mut pointer = String::new();
    for (position, step) in path.iter().enumerate() {
        let last = position + 1 == path.len();
        let next = match (&node.kind, step) {
            (Kind::Object(members), Step::Key(key)) => members
                .iter()
                .position(|member| &member.key == key)
                .map(|index| (&members[index].value, index)),
            (Kind::Array(items), Step::Key(key)) => key
                .parse::<usize>()
                .ok()
                .and_then(|i| items.get(i).map(|n| (n, i))),
            (Kind::Array(items), Step::Index(index)) => items.get(*index).map(|n| (n, *index)),
            (Kind::Array(items), Step::Where(conditions)) => {
                let hits: Vec<usize> = items
                    .iter()
                    .enumerate()
                    .filter(|(_, item)| matches_where(bytes, item, conditions))
                    .map(|(index, _)| index)
                    .collect();
                match hits.len() {
                    1 => Some((&items[hits[0]], hits[0])),
                    0 => {
                        return Err((
                            "json_where_no_match",
                            format!("no element of {} matches {}", pointer_or_root(&pointer), Value::Object(conditions.clone())),
                        ))
                    }
                    n => {
                        return Err((
                            "json_where_ambiguous",
                            format!("{n} elements of {} match {} (indexes {hits:?}); add fields to narrow it to one", pointer_or_root(&pointer), Value::Object(conditions.clone())),
                        ))
                    }
                }
            }
            _ => None,
        };
        match next {
            Some((child, index)) => {
                pointer.push('/');
                pointer.push_str(&match &node.kind {
                    Kind::Object(members) => escape_token(&members[index].key),
                    _ => index.to_string(),
                });
                parent = Some((node, index));
                node = child;
            }
            None if last && matches!(node.kind, Kind::Object(_) | Kind::Array(_)) => {
                let slot = match step {
                    Step::Key(key) if matches!(node.kind, Kind::Array(_)) => {
                        match key.parse::<usize>() {
                            Ok(index) => Step::Index(index),
                            Err(_) => step.clone(),
                        }
                    }
                    other => other.clone(),
                };
                return Ok((Found::Slot(node, slot), pointer));
            }
            None => {
                return Err((
                    "json_path_not_found",
                    format!(
                        "{} has nothing at step {}",
                        pointer_or_root(&pointer),
                        position + 1
                    ),
                ))
            }
        }
    }
    Ok((Found::Value(node, parent), pointer))
}

fn pointer_or_root(pointer: &str) -> &str {
    if pointer.is_empty() {
        "the document root"
    } else {
        pointer
    }
}

/// The span of one item of a container (an object member includes its key).
fn item_span(container: &Node, index: usize) -> (usize, usize) {
    match &container.kind {
        Kind::Object(members) => (members[index].key_start, members[index].value.end),
        Kind::Array(items) => (items[index].start, items[index].end),
        Kind::Scalar => unreachable!("a scalar has no items"),
    }
}

fn item_count(container: &Node) -> usize {
    match &container.kind {
        Kind::Object(members) => members.len(),
        Kind::Array(items) => items.len(),
        Kind::Scalar => 0,
    }
}

/// The separator the file already uses between items of this container,
/// so an inserted item looks like its neighbours.
fn separator(bytes: &[u8], container: &Node) -> String {
    if item_count(container) >= 2 {
        let (_, first_end) = item_span(container, 0);
        let (second_start, _) = item_span(container, 1);
        return String::from_utf8_lossy(&bytes[first_end..second_start]).into_owned();
    }
    if item_count(container) == 1 {
        let (first_start, _) = item_span(container, 0);
        let lead = &bytes[container.start + 1..first_start];
        if lead.contains(&b'\n') {
            return format!(",{}", String::from_utf8_lossy(lead));
        }
    }
    ", ".into()
}

fn colon(bytes: &[u8], container: &Node) -> String {
    match &container.kind {
        Kind::Object(members) if !members.is_empty() => {
            String::from_utf8_lossy(&bytes[members[0].key_end..members[0].value.start]).into_owned()
        }
        _ => ": ".into(),
    }
}

/// One resolved edit: the byte span to replace, its replacement, and what it
/// reports (the concrete pointer, the old value, the new one).
#[derive(Debug, Clone)]
pub struct Edit {
    pub start: usize,
    pub end: usize,
    pub replacement: Vec<u8>,
    pub pointer: String,
    pub old: Option<Value>,
    pub new: Option<Value>,
}

/// The value at an address, its concrete pointer and its span.
#[derive(Debug)]
pub struct Read {
    pub pointer: String,
    pub value: Value,
    pub start: usize,
    pub end: usize,
}

/// The span the address works in: the whole document for JSON, or one line
/// (1-based `line`) for JSONL, located without parsing any other line.
fn scope(
    bytes: &[u8],
    address: &serde_json::Map<String, Value>,
) -> Result<(usize, usize, Option<usize>), Refusal> {
    let Some(line) = address.get("line") else {
        return Ok((0, bytes.len(), None));
    };
    let line = line.as_u64().filter(|line| *line > 0).ok_or((
        "json_address_invalid",
        "line is a 1-based JSONL line number".to_owned(),
    ))? as usize;
    let mut start = 0;
    for _ in 1..line {
        match bytes[start..].iter().position(|&b| b == b'\n') {
            Some(offset) => start += offset + 1,
            None => {
                return Err((
                    "json_line_out_of_range",
                    format!("line {line} does not exist"),
                ));
            }
        }
    }
    if start >= bytes.len() {
        return Err((
            "json_line_out_of_range",
            format!("line {line} does not exist"),
        ));
    }
    let mut end = bytes[start..]
        .iter()
        .position(|&b| b == b'\n')
        .map_or(bytes.len(), |offset| start + offset);
    if end > start && bytes[end - 1] == b'\r' {
        end -= 1;
    }
    Ok((start, end, Some(line)))
}

fn address_object(address: &Value) -> Result<&serde_json::Map<String, Value>, Refusal> {
    address.as_object().ok_or((
        "json_address_invalid",
        "json is an object: {pointer | path, line (JSONL)}".to_owned(),
    ))
}

/// Results `find` returns at most; past this it reports `truncated`.
pub const FIND_LIMIT: usize = 200;

fn collect_matches(
    bytes: &[u8],
    node: &Node,
    pointer: &str,
    conditions: &serde_json::Map<String, Value>,
    found: &mut Vec<Read>,
    total: &mut usize,
) {
    if matches_where(bytes, node, conditions) {
        *total += 1;
        if found.len() < FIND_LIMIT {
            found.push(Read {
                pointer: pointer.to_owned(),
                value: node_value(bytes, node),
                start: node.start,
                end: node.end,
            });
        }
    }
    match &node.kind {
        Kind::Object(members) => {
            for member in members {
                let child = format!("{pointer}/{}", escape_token(&member.key));
                collect_matches(bytes, &member.value, &child, conditions, found, total);
            }
        }
        Kind::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                let child = format!("{pointer}/{index}");
                collect_matches(bytes, item, &child, conditions, found, total);
            }
        }
        Kind::Scalar => {}
    }
}

/// `read` with `{find: {field: value, ...}}`: every object whose fields all
/// match, anywhere under the address (`pointer`/`path` narrows where to look;
/// none means the whole document), each with its concrete pointer -- the
/// search that answers "which index is order abc" before an edit. A JSONL
/// file (or one `line` of it) is searched record by record, pointers
/// prefixed `line N`. Returns the matches (at most FIND_LIMIT) and the total.
pub fn find(bytes: &[u8], address: &Value) -> Result<(Vec<Read>, usize), Refusal> {
    let object = address_object(address)?;
    let conditions = match object.get("find") {
        Some(Value::Object(conditions)) if !conditions.is_empty() => conditions.clone(),
        _ => {
            return Err((
                "json_address_invalid",
                "find is an object of field: value pairs an object must all hold".into(),
            ))
        }
    };
    let mut found = Vec::new();
    let mut total = 0;
    let records: Vec<(usize, usize, Option<usize>)> = match scope(bytes, object)? {
        (start, end, Some(line)) => vec![(start, end, Some(line))],
        (start, end, None) => match parse(bytes, start, end) {
            Ok(_) => vec![(start, end, None)],
            // Not one JSON document: read it as JSONL, a record per line.
            Err(_) => {
                let mut lines = Vec::new();
                let mut line_start = 0;
                for (number, line) in bytes.split(|&b| b == b'\n').enumerate() {
                    let mut line_end = line_start + line.len();
                    if line_end > line_start && bytes[line_end - 1] == b'\r' {
                        line_end -= 1;
                    }
                    if bytes[line_start..line_end]
                        .iter()
                        .any(|b| !b.is_ascii_whitespace())
                    {
                        lines.push((line_start, line_end, Some(number + 1)));
                    }
                    line_start += line.len() + 1;
                }
                lines
            }
        },
    };
    for (start, end, line) in records {
        let root = parse(bytes, start, end)?;
        let (base, prefix) = match walk(bytes, &root, &steps(object)?)? {
            (Found::Value(node, _), pointer) => (node.clone(), pointer),
            (Found::Slot(..), pointer) => {
                return Err((
                    "json_path_not_found",
                    format!(
                        "{} has no such member or element",
                        pointer_or_root(&pointer)
                    ),
                ))
            }
        };
        let prefix = match line {
            Some(line) => format!("line {line}{prefix}"),
            None => prefix,
        };
        collect_matches(bytes, &base, &prefix, &conditions, &mut found, &mut total);
    }
    Ok((found, total))
}

/// `read` with a json address.
pub fn read(bytes: &[u8], address: &Value) -> Result<Read, Refusal> {
    let address = address_object(address)?;
    let (start, end, _) = scope(bytes, address)?;
    let root = parse(bytes, start, end)?;
    match walk(bytes, &root, &steps(address)?)? {
        (Found::Value(node, _), pointer) => Ok(Read {
            pointer,
            value: node_value(bytes, node),
            start: node.start,
            end: node.end,
        }),
        (Found::Slot(..), pointer) => Err((
            "json_path_not_found",
            format!(
                "{} has no such member or element",
                pointer_or_root(&pointer)
            ),
        )),
    }
}

/// Resolves a `replace` with a json address and an `op` into one byte edit.
/// `op` is `set` (default; the value must exist), `insert` (a new object key,
/// or an array element before an index, or `-` for the end), `append` (to the
/// array the address names), `delete`, or `append_line` (JSONL: a new record
/// after the last line, never touching the others). The result is parsed
/// again -- the whole document for JSON, the line for JSONL -- before it is
/// returned, so a malformed edit is refused rather than written.
pub fn edit(
    bytes: &[u8],
    address: &Value,
    op: &str,
    value: Option<&Value>,
    value_text: Option<&str>,
    expected: Option<&Value>,
) -> Result<Edit, Refusal> {
    let address = address_object(address)?;
    // `value_text` is the caller's own JSON text, written verbatim once it
    // parses, so a new value keeps its key order and number spelling;
    // `value` is a parsed JSON value and is written compactly.
    let parsed_text = match value_text {
        Some(text) => Some(serde_json::from_str::<Value>(text).map_err(|error| {
            (
                "json_value_invalid",
                format!("value_text is not valid JSON: {error}"),
            )
        })?),
        None => None,
    };
    let needs_value = |value: Option<&Value>| {
        parsed_text.clone().or_else(|| value.cloned()).ok_or((
            "json_value_missing",
            format!("op {op} needs value (a JSON value: \"3\" is a string, 3 a number) or value_text (JSON text, written as typed)"),
        ))
    };
    let compact = |value: &Value| serde_json::to_string(value).expect("a Value serialises");
    let render = |value: &Value| match value_text {
        Some(text) => text.trim().to_owned(),
        None => compact(value),
    };
    if op == "append_line" {
        let new = needs_value(value)?;
        let ending = if bytes.windows(2).any(|pair| pair == b"\r\n") {
            "\r\n"
        } else {
            "\n"
        };
        let mut replacement = Vec::new();
        if !bytes.is_empty() && !bytes.ends_with(b"\n") {
            replacement.extend_from_slice(ending.as_bytes());
        }
        replacement.extend_from_slice(render(&new).as_bytes());
        replacement.extend_from_slice(ending.as_bytes());
        let line = bytes.iter().filter(|&&b| b == b'\n').count()
            + usize::from(!bytes.is_empty() && !bytes.ends_with(b"\n"))
            + 1;
        return Ok(Edit {
            start: bytes.len(),
            end: bytes.len(),
            replacement,
            pointer: format!("line {line}"),
            old: None,
            new: Some(new),
        });
    }
    let (scope_start, scope_end, line) = scope(bytes, address)?;
    let root = parse(bytes, scope_start, scope_end)?;
    let (found, pointer) = walk(bytes, &root, &steps(address)?)?;
    let check_expected = |current: Option<&Value>| -> Result<(), Refusal> {
        match (expected, current) {
            (Some(wanted), Some(actual)) if wanted != actual => Err((
                "json_expected_value_mismatch",
                format!(
                    "{} holds {actual}, not the expected {wanted}; nothing was written",
                    pointer_or_root(&pointer)
                ),
            )),
            (Some(_), None) => Err((
                "json_expected_value_mismatch",
                format!(
                    "{} holds nothing yet, so expected_value cannot match",
                    pointer_or_root(&pointer)
                ),
            )),
            _ => Ok(()),
        }
    };
    let (start, end, replacement, old, new) = match (op, found) {
        ("set", Found::Value(node, _)) => {
            let current = node_value(bytes, node);
            check_expected(Some(&current))?;
            let new = needs_value(value)?;
            (
                node.start,
                node.end,
                render(&new).into_bytes(),
                Some(current),
                Some(new),
            )
        }
        ("delete", Found::Value(node, Some((container, index)))) => {
            let current = node_value(bytes, node);
            check_expected(Some(&current))?;
            let count = item_count(container);
            let (item_start, item_end) = item_span(container, index);
            let (start, end) = if count == 1 {
                (item_start, item_end)
            } else if index > 0 {
                (item_span(container, index - 1).1, item_end)
            } else {
                (item_start, item_span(container, 1).0)
            };
            (start, end, Vec::new(), Some(current), None)
        }
        ("delete", Found::Value(_, None)) => {
            return Err((
                "json_op_invalid",
                "the document root cannot be deleted".into(),
            ))
        }
        ("append", Found::Value(node, _)) => {
            check_expected(expected.map(|_| node_value(bytes, node)).as_ref())?;
            let Kind::Array(items) = &node.kind else {
                return Err((
                    "json_op_invalid",
                    format!(
                        "append needs an array, and {} is not one",
                        pointer_or_root(&pointer)
                    ),
                ));
            };
            let new = needs_value(value)?;
            let text = render(&new);
            let (at, replacement) = match items.last() {
                None => (node.start + 1, text),
                Some(last) => (last.end, format!("{}{text}", separator(bytes, node))),
            };
            (at, at, replacement.into_bytes(), None, Some(new))
        }
        ("insert", Found::Slot(container, step)) => {
            // Nothing is there yet, so an expected_value can only mismatch.
            check_expected(None)?;
            let new = needs_value(value)?;
            let text = render(&new);
            let count = item_count(container);
            let (at, replacement) = match (&container.kind, step) {
                (Kind::Object(_), Step::Key(key)) => {
                    let member = format!("{}{}{text}", compact(&Value::String(key)), colon(bytes, container));
                    if count == 0 {
                        (container.start + 1, member)
                    } else {
                        (item_span(container, count - 1).1, format!("{}{member}", separator(bytes, container)))
                    }
                }
                (Kind::Array(_), Step::Index(index)) if index == count => match count {
                    0 => (container.start + 1, text),
                    _ => (item_span(container, count - 1).1, format!("{}{text}", separator(bytes, container))),
                },
                (Kind::Array(_), Step::End) => match count {
                    0 => (container.start + 1, text),
                    _ => (item_span(container, count - 1).1, format!("{}{text}", separator(bytes, container))),
                },
                _ => {
                    return Err((
                        "json_path_not_found",
                        format!("{} has no slot for that step; insert names a new key, an index up to the length, or -", pointer_or_root(&pointer)),
                    ))
                }
            };
            (at, at, replacement.into_bytes(), None, Some(new))
        }
        // Before an existing element: the new one takes its place and the
        // file's own separator follows it.
        ("insert", Found::Value(_, Some((container, index))))
            if matches!(container.kind, Kind::Array(_)) =>
        {
            let new = needs_value(value)?;
            let at = item_span(container, index).0;
            let replacement = format!("{}{}", render(&new), separator(bytes, container));
            (at, at, replacement.into_bytes(), None, Some(new))
        }
        ("insert", Found::Value(..)) => {
            return Err((
                "json_key_exists",
                format!(
                    "{} already exists; use op set to change it",
                    pointer_or_root(&pointer)
                ),
            ))
        }
        ("set" | "delete" | "append", Found::Slot(..)) => {
            return Err((
                "json_path_not_found",
                format!(
                    "{} has no such member or element",
                    pointer_or_root(&pointer)
                ),
            ))
        }
        (other, _) => {
            return Err((
                "json_op_invalid",
                format!("op {other:?} is not one of set, insert, append, delete, append_line"),
            ))
        }
    };
    let mut after = bytes[..start].to_vec();
    after.extend_from_slice(&replacement);
    after.extend_from_slice(&bytes[end..]);
    let new_scope_end = scope_end + replacement.len() - (end - start);
    if line.is_some() && replacement.contains(&b'\n') {
        return Err((
            "json_record_split",
            "a JSONL record must stay on one line; the edit would add a line break".into(),
        ));
    }
    if serde_json::from_slice::<Value>(&after[scope_start..new_scope_end]).is_err() {
        return Err((
            "json_invalid_after_edit",
            "the edit would leave invalid JSON; nothing was written".into(),
        ));
    }
    let pointer = match line {
        Some(line) => format!("line {line}{pointer}"),
        None => pointer,
    };
    Ok(Edit {
        start,
        end,
        replacement,
        pointer,
        old,
        new,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// The structured-`value` form every test below uses; `value_text` has
    /// its own test.
    fn edit(
        bytes: &[u8],
        address: &Value,
        op: &str,
        value: Option<&Value>,
        expected: Option<&Value>,
    ) -> Result<Edit, Refusal> {
        super::edit(bytes, address, op, value, None, expected)
    }

    #[test]
    fn value_text_is_written_as_typed_keeping_key_order_and_spelling() {
        let bytes = b"{\"run\":1}\n";
        let appended = super::edit(
            bytes,
            &json!({}),
            "append_line",
            None,
            Some("{\"run\":2,\"meter\":2.17e-06}"),
            None,
        )
        .unwrap();
        assert_eq!(
            apply(bytes, &appended),
            "{\"run\":1}\n{\"run\":2,\"meter\":2.17e-06}\n"
        );
        let invalid =
            super::edit(bytes, &json!({}), "append_line", None, Some("{oops"), None).unwrap_err();
        assert_eq!(invalid.0, "json_value_invalid");
    }

    fn apply(bytes: &[u8], edit: &Edit) -> String {
        let mut after = bytes[..edit.start].to_vec();
        after.extend_from_slice(&edit.replacement);
        after.extend_from_slice(&bytes[edit.end..]);
        String::from_utf8(after).unwrap()
    }

    const BOARD: &str = "{\n  \"tasks\": [\n    {\"id\": \"a\", \"title\": \"One\", \"score\": 2.17e-06},\n    {\"id\": \"b\", \"title\": \"Two\"}\n  ]\n}\n";

    #[test]
    fn a_where_step_sets_one_value_and_leaves_every_other_byte() {
        let bytes = BOARD.as_bytes();
        let edit = edit(
            bytes,
            &json!({"path": ["tasks", {"id": "b"}, "title"]}),
            "set",
            Some(&json!("Deux")),
            None,
        )
        .unwrap();
        assert_eq!(edit.pointer, "/tasks/1/title");
        assert_eq!(apply(bytes, &edit), BOARD.replace("\"Two\"", "\"Deux\""));
    }

    #[test]
    fn append_copies_the_files_own_separator() {
        let bytes = BOARD.as_bytes();
        let edit = edit(
            bytes,
            &json!({"pointer": "/tasks"}),
            "append",
            Some(&json!({"id": "c"})),
            None,
        )
        .unwrap();
        assert!(
            apply(bytes, &edit).contains("\"Two\"},\n    {\"id\":\"c\"}\n  ]"),
            "{}",
            apply(bytes, &edit)
        );
    }

    #[test]
    fn delete_removes_the_item_and_its_separator() {
        let bytes = br#"{"a": [1, 2, 3]}"#;
        let first = edit(bytes, &json!({"pointer": "/a/0"}), "delete", None, None).unwrap();
        assert_eq!(apply(bytes, &first), r#"{"a": [2, 3]}"#);
        let last = edit(bytes, &json!({"pointer": "/a/2"}), "delete", None, None).unwrap();
        assert_eq!(apply(bytes, &last), r#"{"a": [1, 2]}"#);
    }

    #[test]
    fn insert_adds_a_new_key_in_the_files_colon_style() {
        let bytes = br#"{"a": 1, "b": 2}"#;
        let added = edit(
            bytes,
            &json!({"pointer": "/c"}),
            "insert",
            Some(&json!(3)),
            None,
        )
        .unwrap();
        assert_eq!(apply(bytes, &added), r#"{"a": 1, "b": 2, "c": 3}"#);
        let refused = edit(
            bytes,
            &json!({"pointer": "/a"}),
            "insert",
            Some(&json!(3)),
            None,
        )
        .unwrap_err();
        assert_eq!(refused.0, "json_key_exists");
    }

    #[test]
    fn expected_value_is_compared_as_json_not_text() {
        let bytes = br#"{"qty": 3}"#;
        let refused = edit(
            bytes,
            &json!({"pointer": "/qty"}),
            "set",
            Some(&json!(4)),
            Some(&json!("3")),
        )
        .unwrap_err();
        assert_eq!(refused.0, "json_expected_value_mismatch");
        assert!(edit(
            bytes,
            &json!({"pointer": "/qty"}),
            "set",
            Some(&json!(4)),
            Some(&json!(3))
        )
        .is_ok());
    }

    #[test]
    fn jsonl_edits_one_line_and_appends_without_touching_the_rest() {
        let bytes = b"{\"run\":1,\"meter\":0.5}\n{\"run\":2,\"meter\":0.7}\n";
        let set = edit(
            bytes,
            &json!({"line": 2, "pointer": "/meter"}),
            "set",
            Some(&json!(0.9)),
            None,
        )
        .unwrap();
        assert_eq!(set.pointer, "line 2/meter");
        assert_eq!(
            apply(bytes, &set),
            "{\"run\":1,\"meter\":0.5}\n{\"run\":2,\"meter\":0.9}\n"
        );
        let appended = edit(
            bytes,
            &json!({}),
            "append_line",
            Some(&json!({"run": 3})),
            None,
        )
        .unwrap();
        assert_eq!(
            apply(bytes, &appended),
            "{\"run\":1,\"meter\":0.5}\n{\"run\":2,\"meter\":0.7}\n{\"run\":3}\n"
        );
    }

    #[test]
    fn find_returns_every_matching_object_with_its_pointer() {
        let bytes = br#"{"board": {"tasks": [{"id": "a", "done": false}, {"id": "b", "done": true}], "archive": [{"id": "b", "done": true}]}}"#;
        let (found, total) = find(bytes, &json!({"find": {"id": "b"}})).unwrap();
        assert_eq!(total, 2);
        let pointers: Vec<&str> = found.iter().map(|hit| hit.pointer.as_str()).collect();
        assert_eq!(pointers, ["/board/tasks/1", "/board/archive/0"]);
        let (narrowed, _) = find(
            bytes,
            &json!({"find": {"id": "b"}, "pointer": "/board/archive"}),
        )
        .unwrap();
        assert_eq!(narrowed[0].pointer, "/board/archive/0");
    }

    #[test]
    fn find_searches_a_jsonl_file_record_by_record() {
        let bytes =
            b"{\"run\":1,\"ok\":true}\n{\"run\":2,\"ok\":false}\n{\"run\":3,\"ok\":false}\n";
        let (found, total) = find(bytes, &json!({"find": {"ok": false}})).unwrap();
        assert_eq!(total, 2);
        assert_eq!(found[0].pointer, "line 2");
        assert_eq!(found[1].pointer, "line 3");
    }

    #[test]
    fn an_ambiguous_where_is_refused() {
        let bytes = br#"[{"k": 1}, {"k": 1}]"#;
        let refused = read(bytes, &json!({"path": [{"k": 1}]})).unwrap_err();
        assert_eq!(refused.0, "json_where_ambiguous");
    }
}
