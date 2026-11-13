// MODE: DEV
// PACKAGE: PROD
//! XML addressing for `read` and `replace`: an element or attribute is named
//! by an XPath subset (`/config/type[@name='X']/arguments/argument[@name='y']`,
//! `.../@xsi:type`) and resolves to the exact bytes of its text or attribute
//! value, so an edit splices only those -- attribute order, quote style,
//! comments, indentation and every untouched element stay byte for byte. Names
//! are matched literally, prefixes included (`xsi:type`). The document must
//! parse again after every edit.

pub type Refusal = (&'static str, String);

#[derive(Debug, Clone)]
pub struct Attr {
    pub name: String,
    /// Where the whitespace before the name begins, so a delete takes it too.
    pub lead_start: usize,
    pub value_start: usize,
    pub value_end: usize,
    pub quote: u8,
}

#[derive(Debug, Clone)]
pub struct Element {
    pub name: String,
    pub start: usize,
    /// Just past the start tag's `>`.
    pub tag_end: usize,
    /// Where `/>` or `>` closes the start tag, for inserting an attribute.
    pub tag_close: usize,
    pub attrs: Vec<Attr>,
    pub children: Vec<Element>,
    pub content_start: usize,
    pub content_end: usize,
    pub end: usize,
    pub self_closing: bool,
}

fn malformed(at: usize, what: &str) -> Refusal {
    ("xml_malformed", format!("{what} at byte {at}"))
}

struct Parser<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl Parser<'_> {
    fn starts(&self, text: &str) -> bool {
        self.bytes[self.pos..].starts_with(text.as_bytes())
    }

    fn skip_past(&mut self, terminator: &str, what: &str) -> Result<(), Refusal> {
        let start = self.pos;
        match self.bytes[self.pos..]
            .windows(terminator.len())
            .position(|window| window == terminator.as_bytes())
        {
            Some(offset) => {
                self.pos += offset + terminator.len();
                Ok(())
            }
            None => Err(malformed(start, &format!("an unterminated {what}"))),
        }
    }

    /// Comments, processing instructions, CDATA and a DOCTYPE: skipped as is.
    fn skip_misc(&mut self) -> Result<bool, Refusal> {
        if self.starts("<!--") {
            self.skip_past("-->", "comment")?;
        } else if self.starts("<![CDATA[") {
            self.skip_past("]]>", "CDATA section")?;
        } else if self.starts("<?") {
            self.skip_past("?>", "processing instruction")?;
        } else if self.starts("<!") {
            let mut depth = 0i32;
            while self.pos < self.bytes.len() {
                match self.bytes[self.pos] {
                    b'[' => depth += 1,
                    b']' => depth -= 1,
                    b'>' if depth <= 0 => {
                        self.pos += 1;
                        return Ok(true);
                    }
                    _ => {}
                }
                self.pos += 1;
            }
            return Err(malformed(self.pos, "an unterminated declaration"));
        } else {
            return Ok(false);
        }
        Ok(true)
    }

    fn name(&mut self) -> String {
        let start = self.pos;
        while self.pos < self.bytes.len()
            && !matches!(
                self.bytes[self.pos],
                b' ' | b'\t' | b'\r' | b'\n' | b'/' | b'>' | b'='
            )
        {
            self.pos += 1;
        }
        String::from_utf8_lossy(&self.bytes[start..self.pos]).into_owned()
    }

    fn skip_ws(&mut self) {
        while self.pos < self.bytes.len()
            && matches!(self.bytes[self.pos], b' ' | b'\t' | b'\r' | b'\n')
        {
            self.pos += 1;
        }
    }

    fn element(&mut self) -> Result<Element, Refusal> {
        let start = self.pos;
        self.pos += 1;
        let name = self.name();
        if name.is_empty() {
            return Err(malformed(start, "an element name was expected"));
        }
        let mut attrs = Vec::new();
        let (tag_close, self_closing) = loop {
            let lead_start = self.pos;
            self.skip_ws();
            if self.starts("/>") {
                break (self.pos, true);
            }
            if self.starts(">") {
                break (self.pos, false);
            }
            if self.pos >= self.bytes.len() {
                return Err(malformed(start, "an unterminated start tag"));
            }
            let attr_name = self.name();
            if attr_name.is_empty() {
                return Err(malformed(self.pos, "an attribute name was expected"));
            }
            self.skip_ws();
            if !self.starts("=") {
                return Err(malformed(
                    self.pos,
                    "'=' was expected after an attribute name",
                ));
            }
            self.pos += 1;
            self.skip_ws();
            let quote = *self
                .bytes
                .get(self.pos)
                .ok_or_else(|| malformed(self.pos, "a quoted value was expected"))?;
            if quote != b'"' && quote != b'\'' {
                return Err(malformed(self.pos, "an attribute value must be quoted"));
            }
            let value_start = self.pos + 1;
            let value_end = self.bytes[value_start..]
                .iter()
                .position(|&b| b == quote)
                .map(|offset| value_start + offset)
                .ok_or_else(|| malformed(value_start, "an unterminated attribute value"))?;
            self.pos = value_end + 1;
            attrs.push(Attr {
                name: attr_name,
                lead_start,
                value_start,
                value_end,
                quote,
            });
        };
        if self_closing {
            self.pos += 2;
            return Ok(Element {
                name,
                start,
                tag_end: self.pos,
                tag_close,
                attrs,
                children: Vec::new(),
                content_start: self.pos,
                content_end: self.pos,
                end: self.pos,
                self_closing: true,
            });
        }
        self.pos += 1;
        let tag_end = self.pos;
        let mut children = Vec::new();
        loop {
            if self.pos >= self.bytes.len() {
                return Err(malformed(start, &format!("<{name}> is never closed")));
            }
            if self.starts("</") {
                let content_end = self.pos;
                self.pos += 2;
                let closing = self.name();
                if closing != name {
                    return Err(malformed(
                        content_end,
                        &format!("</{closing}> closes <{name}>"),
                    ));
                }
                self.skip_ws();
                if !self.starts(">") {
                    return Err(malformed(self.pos, "'>' was expected"));
                }
                self.pos += 1;
                return Ok(Element {
                    name,
                    start,
                    tag_end,
                    tag_close,
                    attrs,
                    children,
                    content_start: tag_end,
                    content_end,
                    end: self.pos,
                    self_closing: false,
                });
            }
            if self.skip_misc()? {
                continue;
            }
            if self.starts("<") {
                children.push(self.element()?);
            } else {
                self.pos += 1;
            }
        }
    }
}

/// Parses a whole document: prolog, exactly one root element, epilog.
pub fn parse(bytes: &[u8]) -> Result<Element, Refusal> {
    let mut parser = Parser { bytes, pos: 0 };
    if parser.starts("\u{feff}") {
        parser.pos += 3;
    }
    let mut root = None;
    loop {
        parser.skip_ws();
        if parser.pos >= bytes.len() {
            break;
        }
        if parser.skip_misc()? {
            continue;
        }
        if parser.starts("<") {
            if root.is_some() {
                return Err(malformed(parser.pos, "a second root element"));
            }
            root = Some(parser.element()?);
        } else {
            return Err(malformed(parser.pos, "text outside the root element"));
        }
    }
    root.ok_or(("xml_malformed", "the document has no root element".into()))
}

/// Decodes the predefined and numeric entities; CDATA is kept as its text.
pub fn decode(raw: &[u8]) -> String {
    let text = String::from_utf8_lossy(raw);
    let mut out = String::new();
    let mut rest: &str = &text;
    while let Some(index) = rest.find(['&', '<']) {
        out.push_str(&rest[..index]);
        rest = &rest[index..];
        if let Some(cdata) = rest.strip_prefix("<![CDATA[") {
            let end = cdata.find("]]>").unwrap_or(cdata.len());
            out.push_str(&cdata[..end]);
            rest = cdata.get(end + 3..).unwrap_or("");
            continue;
        }
        if rest.starts_with('<') {
            out.push('<');
            rest = &rest[1..];
            continue;
        }
        let Some(semi) = rest.find(';') else {
            out.push_str(rest);
            rest = "";
            break;
        };
        let entity = &rest[1..semi];
        let decoded = match entity {
            "lt" => Some('<'),
            "gt" => Some('>'),
            "amp" => Some('&'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            _ => entity
                .strip_prefix("#x")
                .and_then(|hex| u32::from_str_radix(hex, 16).ok())
                .or_else(|| entity.strip_prefix('#').and_then(|dec| dec.parse().ok()))
                .and_then(char::from_u32),
        };
        match decoded {
            Some(character) => out.push(character),
            None => out.push_str(&rest[..=semi]),
        }
        rest = &rest[semi + 1..];
    }
    out.push_str(rest);
    out
}

fn escape_text(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn escape_attr(value: &str, quote: u8) -> String {
    let escaped = value.replace('&', "&amp;").replace('<', "&lt;");
    if quote == b'"' {
        escaped.replace('"', "&quot;")
    } else {
        escaped.replace('\'', "&apos;")
    }
}

#[derive(Debug, Clone)]
enum Predicate {
    Equals(String, String),
    Has(String),
    Position(usize),
}

#[derive(Debug, Clone)]
struct Step {
    name: String,
    predicates: Vec<Predicate>,
}

/// Parses `/a/b[@k='v'][2]/c` (and a trailing `/@attr`) into steps.
fn parse_path(path: &str) -> Result<(Vec<Step>, Option<String>), Refusal> {
    let invalid = |why: &str| ("xml_address_invalid", format!("{why}: {path}"));
    let body = path
        .strip_prefix('/')
        .ok_or_else(|| invalid("an XML path starts with '/'"))?;
    let mut raw_steps = Vec::new();
    let (mut current, mut quote, mut depth) = (String::new(), None::<char>, 0);
    for character in body.chars() {
        match (character, quote) {
            ('\'' | '"', None) if depth > 0 => {
                quote = Some(character);
                current.push(character);
            }
            (c, Some(open)) if c == open => {
                quote = None;
                current.push(c);
            }
            ('[', None) => {
                depth += 1;
                current.push('[');
            }
            (']', None) => {
                depth -= 1;
                current.push(']');
            }
            ('/', None) if depth == 0 => raw_steps.push(std::mem::take(&mut current)),
            (c, _) => current.push(c),
        }
    }
    raw_steps.push(current);
    let attribute = match raw_steps.last() {
        Some(last) if last.starts_with('@') => Some(raw_steps.pop().unwrap()[1..].to_owned()),
        _ => None,
    };
    let mut steps = Vec::new();
    for raw in raw_steps {
        let (name, mut rest) = match raw.find('[') {
            Some(index) => (raw[..index].to_owned(), &raw[index..]),
            None => (raw.clone(), ""),
        };
        if name.is_empty() {
            return Err(invalid("an empty step ('//' is not supported)"));
        }
        let mut predicates = Vec::new();
        while let Some(inner) = rest.strip_prefix('[') {
            let close = inner.find(']').ok_or_else(|| invalid("an unclosed '['"))?;
            let predicate = inner[..close].trim();
            rest = &inner[close + 1..];
            if let Ok(position) = predicate.parse::<usize>() {
                if position == 0 {
                    return Err(invalid("a position is 1-based"));
                }
                predicates.push(Predicate::Position(position));
            } else if let Some(attr) = predicate.strip_prefix('@') {
                match attr.split_once('=') {
                    Some((key, value)) => {
                        let value = value.trim();
                        let unquoted = value
                            .strip_prefix('\'')
                            .and_then(|v| v.strip_suffix('\''))
                            .or_else(|| value.strip_prefix('"').and_then(|v| v.strip_suffix('"')))
                            .ok_or_else(|| invalid("a predicate value is quoted"))?;
                        predicates.push(Predicate::Equals(
                            key.trim().to_owned(),
                            unquoted.to_owned(),
                        ));
                    }
                    None => predicates.push(Predicate::Has(attr.trim().to_owned())),
                }
            } else {
                return Err(invalid("a predicate is [@name='value'], [@name] or [n]"));
            }
        }
        steps.push(Step { name, predicates });
    }
    Ok((steps, attribute))
}

fn attr_value(bytes: &[u8], element: &Element, name: &str) -> Option<String> {
    element
        .attrs
        .iter()
        .find(|attr| attr.name == name)
        .map(|attr| decode(&bytes[attr.value_start..attr.value_end]))
}

fn select<'a>(bytes: &[u8], candidates: Vec<&'a Element>, step: &Step) -> Vec<&'a Element> {
    let mut chosen: Vec<&Element> = candidates
        .into_iter()
        .filter(|element| element.name == step.name)
        .collect();
    for predicate in &step.predicates {
        chosen = match predicate {
            Predicate::Equals(key, value) => chosen
                .into_iter()
                .filter(|element| {
                    attr_value(bytes, element, key).as_deref() == Some(value.as_str())
                })
                .collect(),
            Predicate::Has(key) => chosen
                .into_iter()
                .filter(|element| element.attrs.iter().any(|attr| &attr.name == key))
                .collect(),
            Predicate::Position(position) => {
                chosen.get(position - 1).copied().into_iter().collect()
            }
        };
    }
    chosen
}

/// The element a path names, exactly one, with its concrete path.
fn locate<'a>(
    bytes: &[u8],
    root: &'a Element,
    steps: &[Step],
) -> Result<(&'a Element, String), Refusal> {
    let mut current: Vec<&Element> = vec![root];
    let mut concrete = String::new();
    for (index, step) in steps.iter().enumerate() {
        let candidates: Vec<&Element> = if index == 0 {
            current.clone()
        } else {
            current[0].children.iter().collect()
        };
        let siblings_named = candidates.iter().filter(|e| e.name == step.name).count();
        let chosen = select(bytes, candidates.clone(), step);
        match chosen.len() {
            1 => {
                let position = candidates
                    .iter()
                    .filter(|e| e.name == step.name)
                    .position(|e| std::ptr::eq(*e, chosen[0]))
                    .unwrap_or(0)
                    + 1;
                concrete.push('/');
                concrete.push_str(&step.name);
                if siblings_named > 1 {
                    concrete.push_str(&format!("[{position}]"));
                }
                current = chosen;
            }
            0 => {
                return Err((
                    "xml_path_not_found",
                    format!("no <{}> matches step {} under {}", step.name, index + 1, if concrete.is_empty() { "the document" } else { &concrete }),
                ))
            }
            count => {
                return Err((
                    "xml_path_ambiguous",
                    format!("{count} <{}> elements match step {} under {}; add [@attr='value'] or [n] to pick one", step.name, index + 1, if concrete.is_empty() { "the document" } else { &concrete }),
                ))
            }
        }
    }
    Ok((current[0], concrete))
}

/// What an xml address reads: an attribute's value, an element's decoded
/// text, or (for an element with child elements) its raw XML.
#[derive(Debug)]
pub struct Read {
    pub path: String,
    pub value: String,
    pub start: usize,
    pub end: usize,
}

fn address_path(address: &serde_json::Value) -> Result<&str, Refusal> {
    address
        .get("path")
        .and_then(serde_json::Value::as_str)
        .ok_or((
            "xml_address_invalid",
            "xml is {path: \"/root/child[@name='x']\"}, optionally ending in /@attribute"
                .to_owned(),
        ))
}

pub fn read(bytes: &[u8], address: &serde_json::Value) -> Result<Read, Refusal> {
    let (steps, attribute) = parse_path(address_path(address)?)?;
    let root = parse(bytes)?;
    let (element, path) = locate(bytes, &root, &steps)?;
    match attribute {
        Some(name) => {
            let attr = element.attrs.iter().find(|attr| attr.name == name).ok_or((
                "xml_path_not_found",
                format!("{path} has no attribute {name}"),
            ))?;
            Ok(Read {
                path: format!("{path}/@{name}"),
                value: decode(&bytes[attr.value_start..attr.value_end]),
                start: attr.value_start,
                end: attr.value_end,
            })
        }
        None if element.children.is_empty() => Ok(Read {
            path,
            value: decode(&bytes[element.content_start..element.content_end]),
            start: element.content_start,
            end: element.content_end,
        }),
        None => Ok(Read {
            path,
            value: String::from_utf8_lossy(&bytes[element.start..element.end]).into_owned(),
            start: element.start,
            end: element.end,
        }),
    }
}

#[derive(Debug, Clone)]
pub struct Edit {
    pub start: usize,
    pub end: usize,
    pub replacement: Vec<u8>,
    pub path: String,
    pub old: Option<String>,
    pub new: Option<String>,
}

/// Resolves a `replace` with an xml address. `op` is `set` (default: an
/// element's text, or an attribute's value -- added in the file's quote style
/// if the element lacks it), `delete` (the attribute, or the element with its
/// line), or `append` (a raw XML child, given as `value_text`, after the last
/// child at its indentation). `value` is text and is escaped; `expected` is
/// compared with the decoded current value.
pub fn edit(
    bytes: &[u8],
    address: &serde_json::Value,
    op: &str,
    value: Option<&str>,
    value_text: Option<&str>,
    expected: Option<&str>,
) -> Result<Edit, Refusal> {
    let (steps, attribute) = parse_path(address_path(address)?)?;
    let root = parse(bytes)?;
    let (element, path) = locate(bytes, &root, &steps)?;
    let check = |current: Option<&str>| match (expected, current) {
        (Some(wanted), Some(actual)) if wanted != actual => Err((
            "xml_expected_value_mismatch",
            format!("{path} holds {actual:?}, not the expected {wanted:?}; nothing was written"),
        )),
        (Some(_), None) => Err((
            "xml_expected_value_mismatch",
            format!("{path} holds nothing yet, so expected_value cannot match"),
        )),
        _ => Ok(()),
    };
    let needs_value = || {
        value.ok_or((
            "xml_value_missing",
            format!("op {op} needs value, the text to write"),
        ))
    };
    let (start, end, replacement, full_path, old, new) = match (op, &attribute) {
        ("set", Some(name)) => {
            let new = needs_value()?.to_owned();
            match element.attrs.iter().find(|attr| &attr.name == name) {
                Some(attr) => {
                    let old = decode(&bytes[attr.value_start..attr.value_end]);
                    check(Some(&old))?;
                    (
                        attr.value_start,
                        attr.value_end,
                        escape_attr(&new, attr.quote),
                        format!("{path}/@{name}"),
                        Some(old),
                        Some(new),
                    )
                }
                None => {
                    check(None)?;
                    let quote = element.attrs.first().map_or(b'"', |attr| attr.quote);
                    let text = format!(
                        " {name}={q}{}{q}",
                        escape_attr(&new, quote),
                        q = quote as char
                    );
                    let at = element
                        .attrs
                        .last()
                        .map_or(element.start + 1 + element.name.len(), |attr| {
                            attr.value_end + 1
                        });
                    (at, at, text, format!("{path}/@{name}"), None, Some(new))
                }
            }
        }
        ("delete", Some(name)) => {
            let attr = element
                .attrs
                .iter()
                .find(|attr| &attr.name == name)
                .ok_or((
                    "xml_path_not_found",
                    format!("{path} has no attribute {name}"),
                ))?;
            let old = decode(&bytes[attr.value_start..attr.value_end]);
            check(Some(&old))?;
            (
                attr.lead_start,
                attr.value_end + 1,
                String::new(),
                format!("{path}/@{name}"),
                Some(old),
                None,
            )
        }
        ("set", None) => {
            if !element.children.is_empty() {
                return Err((
                    "xml_mixed_content",
                    format!("{path} holds child elements; set its text only on an element without them, or address a child"),
                ));
            }
            let new = needs_value()?.to_owned();
            let old = decode(&bytes[element.content_start..element.content_end]);
            check(Some(&old))?;
            if element.self_closing {
                let text = format!(">{}</{}>", escape_text(&new), element.name);
                (
                    element.tag_close,
                    element.end,
                    text,
                    path.clone(),
                    Some(old),
                    Some(new),
                )
            } else {
                (
                    element.content_start,
                    element.content_end,
                    escape_text(&new),
                    path.clone(),
                    Some(old),
                    Some(new),
                )
            }
        }
        ("delete", None) => {
            if std::ptr::eq(element, &root) {
                return Err((
                    "xml_op_invalid",
                    "the root element cannot be deleted".into(),
                ));
            }
            let old = String::from_utf8_lossy(&bytes[element.start..element.end]).into_owned();
            let mut start = element.start;
            while start > 0 && matches!(bytes[start - 1], b' ' | b'\t') {
                start -= 1;
            }
            if start > 0 && bytes[start - 1] == b'\n' {
                start -= 1;
                if start > 0 && bytes[start - 1] == b'\r' {
                    start -= 1;
                }
            } else {
                start = element.start;
            }
            (
                start,
                element.end,
                String::new(),
                path.clone(),
                Some(old),
                None,
            )
        }
        ("append", None) => {
            let fragment = value_text
                .ok_or((
                    "xml_value_missing",
                    "append takes value_text, the child as raw XML".to_owned(),
                ))?
                .trim()
                .to_owned();
            let (at, text) = match (element.children.last(), element.self_closing) {
                (Some(last), _) => {
                    let mut indent_start = last.start;
                    while indent_start > 0 && matches!(bytes[indent_start - 1], b' ' | b'\t') {
                        indent_start -= 1;
                    }
                    let lead = if indent_start > 0 && bytes[indent_start - 1] == b'\n' {
                        let newline_start = if indent_start > 1 && bytes[indent_start - 2] == b'\r'
                        {
                            indent_start - 2
                        } else {
                            indent_start - 1
                        };
                        String::from_utf8_lossy(&bytes[newline_start..last.start]).into_owned()
                    } else {
                        String::new()
                    };
                    (last.end, format!("{lead}{fragment}"))
                }
                (None, true) => (element.tag_close, format!(">{fragment}</{}>", element.name)),
                (None, false) => (element.content_start, fragment.clone()),
            };
            let end = if element.children.is_empty() && element.self_closing {
                element.end
            } else {
                at
            };
            (at, end, text, path.clone(), None, Some(fragment))
        }
        ("append", Some(_)) => {
            return Err((
                "xml_op_invalid",
                "append adds a child element; address the element, not an attribute".into(),
            ))
        }
        (other, _) => {
            return Err((
                "xml_op_invalid",
                format!("op {other:?} is not one of set, delete, append"),
            ))
        }
    };
    let mut after = bytes[..start].to_vec();
    after.extend_from_slice(replacement.as_bytes());
    after.extend_from_slice(&bytes[end..]);
    parse(&after).map_err(|(_, why)| {
        (
            "xml_invalid_after_edit",
            format!("the edit would leave malformed XML ({why}); nothing was written"),
        )
    })?;
    Ok(Edit {
        start,
        end,
        replacement: replacement.into_bytes(),
        path: full_path,
        old,
        new,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const DI: &str = "<?xml version=\"1.0\"?>\n<!-- di -->\n<config xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\">\n    <type name=\"A\">\n        <arguments>\n            <argument name=\"limit\" xsi:type=\"number\">10</argument>\n            <argument name=\"mode\" xsi:type=\"string\">fast &amp; loose</argument>\n        </arguments>\n    </type>\n    <type name='B'/>\n</config>\n";

    fn apply(bytes: &[u8], edit: &Edit) -> String {
        let mut after = bytes[..edit.start].to_vec();
        after.extend_from_slice(&edit.replacement);
        after.extend_from_slice(&bytes[edit.end..]);
        String::from_utf8(after).unwrap()
    }

    #[test]
    fn a_predicate_path_reads_decoded_text_and_a_prefixed_attribute() {
        let bytes = DI.as_bytes();
        let text = read(
            bytes,
            &json!({"path": "/config/type[@name='A']/arguments/argument[@name='mode']"}),
        )
        .unwrap();
        assert_eq!(text.value, "fast & loose");
        let attr = read(
            bytes,
            &json!({"path": "/config/type[@name='A']/arguments/argument[@name='limit']/@xsi:type"}),
        )
        .unwrap();
        assert_eq!(attr.value, "number");
    }

    #[test]
    fn set_text_and_attribute_splice_only_their_bytes() {
        let bytes = DI.as_bytes();
        let text = edit(
            bytes,
            &json!({"path": "/config/type[@name='A']/arguments/argument[@name='limit']"}),
            "set",
            Some("20 < 30"),
            None,
            Some("10"),
        )
        .unwrap();
        assert_eq!(apply(bytes, &text), DI.replace(">10<", ">20 &lt; 30<"));
        let added = edit(
            bytes,
            &json!({"path": "/config/type[@name='B']/@shared"}),
            "set",
            Some("false"),
            None,
            None,
        )
        .unwrap();
        assert_eq!(
            apply(bytes, &added),
            DI.replace("<type name='B'/>", "<type name='B' shared='false'/>")
        );
    }

    #[test]
    fn append_copies_the_siblings_indentation_and_delete_takes_the_line() {
        let bytes = DI.as_bytes();
        let appended = edit(
            bytes,
            &json!({"path": "/config/type[@name='A']/arguments"}),
            "append",
            None,
            Some("<argument name=\"x\">1</argument>"),
            None,
        )
        .unwrap();
        assert!(apply(bytes, &appended).contains("loose</argument>\n            <argument name=\"x\">1</argument>\n        </arguments>"), "{}", apply(bytes, &appended));
        let deleted = edit(
            bytes,
            &json!({"path": "/config/type[@name='B']"}),
            "delete",
            None,
            None,
            None,
        )
        .unwrap();
        assert_eq!(
            apply(bytes, &deleted),
            DI.replace("\n    <type name='B'/>", "")
        );
    }

    #[test]
    fn an_ambiguous_step_and_malformed_append_are_refused() {
        let bytes = DI.as_bytes();
        assert_eq!(
            read(bytes, &json!({"path": "/config/type"})).unwrap_err().0,
            "xml_path_ambiguous"
        );
        let broken = edit(
            bytes,
            &json!({"path": "/config/type[@name='A']/arguments"}),
            "append",
            None,
            Some("<argument>"),
            None,
        )
        .unwrap_err();
        assert_eq!(broken.0, "xml_invalid_after_edit");
    }
}
