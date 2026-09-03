// MODE: DEV
// PACKAGE: PROD
use serde::de::{DeserializeSeed, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;
use std::fmt;
use thiserror::Error;

pub const MAX_SERIALIZED_FRAME_BYTES: usize = 8 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Envelope {
    pub version: u32,
    pub request_id: String,
    pub method: String,
    pub revision: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth_token: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_token: Option<String>,
    pub payload: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DataFrame {
    pub version: u32,
    pub request_id: String,
    pub sequence: u64,
    pub payload: Value,
    pub byte_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CompleteFrame {
    pub version: u32,
    pub request_id: String,
    pub sequence: u64,
    pub result_generation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ErrorFrame {
    pub version: u32,
    pub request_id: String,
    pub code: String,
    pub message: String,
    pub retryable: bool,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ProtocolError {
    #[error("unsupported protocol version {0}")]
    Version(u32),
    #[error("request id is empty")]
    EmptyRequestId,
    #[error("method is empty")]
    EmptyMethod,
    #[error("serialized frame exceeds 8 MiB")]
    FrameTooLarge,
    #[error("frame must end with exactly one LF")]
    InvalidFraming,
    #[error("invalid JSON: {0}")]
    Json(String),
    #[error("JSON object contains duplicate key: {0}")]
    DuplicateKey(String),
}

pub fn canonical_json(value: &Value) -> Vec<u8> {
    let mut out = Vec::new();
    write_canonical(value, &mut out);
    out
}

fn write_canonical(value: &Value, out: &mut Vec<u8>) {
    match value {
        Value::Null => out.extend_from_slice(b"null"),
        Value::Bool(value) => out.extend_from_slice(if *value { b"true" } else { b"false" }),
        Value::Number(value) => out.extend_from_slice(value.to_string().as_bytes()),
        Value::String(value) => {
            out.extend_from_slice(serde_json::to_string(value).unwrap().as_bytes())
        }
        Value::Array(values) => {
            out.push(b'[');
            for (index, value) in values.iter().enumerate() {
                if index != 0 {
                    out.push(b',');
                }
                write_canonical(value, out);
            }
            out.push(b']');
        }
        Value::Object(values) => {
            let mut keys: Vec<&String> = values.keys().collect();
            keys.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));
            out.push(b'{');
            for (index, key) in keys.iter().enumerate() {
                if index != 0 {
                    out.push(b',');
                }
                out.extend_from_slice(serde_json::to_string(*key).unwrap().as_bytes());
                out.push(b':');
                write_canonical(&values[*key], out);
            }
            out.push(b'}');
        }
    }
}

pub fn validate_ndjson(line: &[u8]) -> Result<Value, ProtocolError> {
    if !line.ends_with(b"\n")
        || line.len() < 2
        || line[..line.len() - 1].contains(&b'\n')
        || line[..line.len() - 1].contains(&b'\r')
    {
        return Err(ProtocolError::InvalidFraming);
    }
    if line.len() > MAX_SERIALIZED_FRAME_BYTES {
        return Err(ProtocolError::FrameTooLarge);
    }
    let value = parse_json_no_duplicates(&line[..line.len() - 1])?;
    Ok(value)
}

fn parse_json_no_duplicates(bytes: &[u8]) -> Result<Value, ProtocolError> {
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let value = StrictValue
        .deserialize(&mut deserializer)
        .map_err(|error| {
            let message = error.to_string();
            let prefix = "JSON object contains duplicate key: ";
            if let Some(key) = message.strip_prefix(prefix) {
                ProtocolError::DuplicateKey(key.split(" at line ").next().unwrap_or(key).to_owned())
            } else {
                ProtocolError::Json(message)
            }
        })?;
    deserializer
        .end()
        .map_err(|error| ProtocolError::Json(error.to_string()))?;
    Ok(value)
}

struct StrictValue;

impl<'de> DeserializeSeed<'de> for StrictValue {
    type Value = Value;

    fn deserialize<D>(self, deserializer: D) -> Result<Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(StrictVisitor)
    }
}

struct StrictVisitor;

impl<'de> Visitor<'de> for StrictVisitor {
    type Value = Value;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a JSON value")
    }

    fn visit_bool<E>(self, value: bool) -> Result<Value, E> {
        Ok(Value::Bool(value))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Value, E> {
        Ok(Value::Number(value.into()))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Value, E> {
        Ok(Value::Number(value.into()))
    }

    fn visit_f64<E>(self, value: f64) -> Result<Value, E>
    where
        E: serde::de::Error,
    {
        serde_json::Number::from_f64(value)
            .map(Value::Number)
            .ok_or_else(|| E::custom("non-finite JSON number"))
    }

    fn visit_str<E>(self, value: &str) -> Result<Value, E> {
        Ok(Value::String(value.to_owned()))
    }

    fn visit_string<E>(self, value: String) -> Result<Value, E> {
        Ok(Value::String(value))
    }

    fn visit_unit<E>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }

    fn visit_none<E>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        StrictValue.deserialize(deserializer)
    }

    fn visit_seq<A>(self, mut access: A) -> Result<Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut values = Vec::new();
        while let Some(value) = access.next_element_seed(StrictValue)? {
            values.push(value);
        }
        Ok(Value::Array(values))
    }

    fn visit_map<A>(self, mut access: A) -> Result<Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut values = serde_json::Map::new();
        let mut keys = HashSet::new();
        while let Some(key) = access.next_key::<String>()? {
            if !keys.insert(key.clone()) {
                return Err(serde::de::Error::custom(ProtocolError::DuplicateKey(key)));
            }
            let value = access.next_value_seed(StrictValue)?;
            values.insert(key, value);
        }
        Ok(Value::Object(values))
    }
}

impl Envelope {
    pub fn validate(&self) -> Result<(), ProtocolError> {
        if self.version != crate::PROTOCOL_VERSION {
            return Err(ProtocolError::Version(self.version));
        }
        if self.request_id.is_empty() {
            return Err(ProtocolError::EmptyRequestId);
        }
        if self.method.is_empty() {
            return Err(ProtocolError::EmptyMethod);
        }
        if canonical_json(&serde_json::to_value(self).unwrap()).len() > MAX_SERIALIZED_FRAME_BYTES {
            return Err(ProtocolError::FrameTooLarge);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_crlf_and_embedded_newlines() {
        assert_eq!(
            validate_ndjson(b"{\"ok\":true}\r\n"),
            Err(ProtocolError::InvalidFraming)
        );
        assert_eq!(
            validate_ndjson(b"{\"ok\":true}\n\n"),
            Err(ProtocolError::InvalidFraming)
        );
    }

    #[test]
    fn rejects_duplicate_keys_at_any_depth() {
        assert_eq!(
            validate_ndjson(b"{\"a\":1,\"a\":2}\n"),
            Err(ProtocolError::DuplicateKey("a".into()))
        );
        assert_eq!(
            validate_ndjson(b"{\"outer\":{\"a\":1,\"a\":2}}\n"),
            Err(ProtocolError::DuplicateKey("a".into()))
        );
    }

    #[test]
    fn canonical_json_sorts_object_keys_without_whitespace() {
        let value: Value = serde_json::json!({"z": 1, "a": [true, null]});
        assert_eq!(canonical_json(&value), br#"{"a":[true,null],"z":1}"#);
    }
}
