// MODE: DEV
// PACKAGE: PROD
//! Ordered, string-keyed maps used by planning command implementations.
//!
//! The shell helper cannot use Bash associative arrays on the supported Bash
//! 3.2 floor.  It therefore stores an encoded key list alongside flattened
//! variables.  This crate keeps the same observable contract: insertion order,
//! replacement without reordering, empty values distinct from missing values,
//! and byte-reversible keys.

use std::collections::HashMap;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PlanningMap {
    values: HashMap<String, String>,
    order: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MapNameError {
    Empty,
    Invalid(String),
}

impl std::fmt::Display for MapNameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => write!(f, "Map name must be [A-Za-z0-9_]+: "),
            Self::Invalid(name) => write!(f, "Map name must be [A-Za-z0-9_]+: {name}"),
        }
    }
}

impl std::error::Error for MapNameError {}

/// Encode a map key exactly as `plan_map_encode_into` does.
pub fn encode_key(key: &str) -> String {
    if key.bytes().all(|byte| byte.is_ascii_alphanumeric()) {
        return key.to_owned();
    }

    let mut encoded = String::with_capacity(key.len());
    for byte in key.as_bytes() {
        if byte.is_ascii_alphanumeric() {
            encoded.push(*byte as char);
        } else {
            encoded.push('_');
            encoded.push(char::from_digit(u32::from(byte >> 4), 16).unwrap());
            encoded.push(char::from_digit(u32::from(byte & 0x0f), 16).unwrap());
        }
    }
    encoded
}

/// Decode an encoded key. Invalid runs are retained as literal text, matching
/// the shell `%b` decoder's useful property of never discarding input.
pub fn decode_key(encoded: &str) -> String {
    let bytes = encoded.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'_' && index + 2 < bytes.len() {
            if let (Some(high), Some(low)) = (hex(bytes[index + 1]), hex(bytes[index + 2])) {
                decoded.push((high << 4) | low);
                index += 3;
                continue;
            }
        }
        decoded.push(bytes[index]);
        index += 1;
    }
    String::from_utf8_lossy(&decoded).into_owned()
}

fn hex(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

pub fn validate_map_name(name: &str) -> Result<(), MapNameError> {
    if name.is_empty() {
        return Err(MapNameError::Empty);
    }
    if name
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
    {
        Ok(())
    } else {
        Err(MapNameError::Invalid(name.to_owned()))
    }
}

impl PlanningMap {
    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) {
        let key = key.into();
        if !self.values.contains_key(&key) {
            self.order.push(key.clone());
        }
        self.values.insert(key, value.into());
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.values.get(key).map(String::as_str)
    }

    pub fn has(&self, key: &str) -> bool {
        self.values.contains_key(key)
    }

    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.order.iter().map(String::as_str)
    }

    pub fn count(&self) -> usize {
        self.order.len()
    }

    pub fn clear(&mut self) {
        self.values.clear();
        self.order.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::{decode_key, encode_key, validate_map_name, PlanningMap};

    #[test]
    fn preserves_order_and_empty_values() {
        let mut map = PlanningMap::default();
        map.set("AR-01", "first");
        map.set("empty", "");
        map.set("AR-01", "replaced");
        assert_eq!(map.keys().collect::<Vec<_>>(), vec!["AR-01", "empty"]);
        assert_eq!(map.get("AR-01"), Some("replaced"));
        assert_eq!(map.get("empty"), Some(""));
        assert!(!map.has("missing"));
        assert_eq!(map.count(), 2);
    }

    #[test]
    fn encoding_is_byte_reversible_and_distinguishes_underscore() {
        for key in ["AR-01", "AR_01", "a b", "ümlaut", ""] {
            assert_eq!(decode_key(&encode_key(key)), key);
        }
        assert_ne!(encode_key("AR-01"), encode_key("AR_01"));
        assert_eq!(encode_key("W37"), "W37");
    }

    #[test]
    fn map_names_match_shell_guard() {
        assert!(validate_map_name("unit_goal").is_ok());
        assert!(validate_map_name("").is_err());
        assert!(validate_map_name("unit-goal").is_err());
    }
}
