// MODE: DEV
// PACKAGE: PROD
use crate::protocol::canonical_json;
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct JournalRecord {
    pub tx_id: String,
    pub seq: u64,
    pub kind: String,
    pub payload: Value,
}

#[derive(Debug)]
pub struct Journal {
    path: PathBuf,
    file: File,
}

impl Journal {
    pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .read(true)
            .open(&path)?;
        Ok(Self { path, file })
    }

    pub fn append(&mut self, record: &JournalRecord) -> io::Result<usize> {
        let mut record_len = 0usize;
        loop {
            let mut preimage = base_object(record, None);
            preimage.insert("record_len".into(), Value::Number(record_len.into()));
            let digest = hex::encode(Sha256::digest(canonical_json(&Value::Object(preimage))));
            let mut final_object = base_object(record, Some(digest));
            final_object.insert("record_len".into(), Value::Number(record_len.into()));
            let candidate = canonical_json(&Value::Object(final_object));
            if candidate.len() == record_len {
                let mut output = candidate;
                output.push(b'\n');
                self.file.write_all(&output)?;
                self.file.sync_data()?;
                return Ok(output.len());
            }
            record_len = candidate.len();
            if record_len > crate::protocol::MAX_SERIALIZED_FRAME_BYTES {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "journal record exceeds protocol frame limit",
                ));
            }
        }
    }

    pub fn replay(&mut self) -> io::Result<Vec<JournalRecord>> {
        self.file.seek(SeekFrom::Start(0))?;
        let mut reader = BufReader::new(self.file.try_clone()?);
        let mut records = Vec::new();
        let mut line = Vec::new();
        loop {
            line.clear();
            let read = reader.read_until(b'\n', &mut line)?;
            if read == 0 {
                break;
            }
            if !line.ends_with(b"\n")
                || line[..line.len() - 1].contains(&b'\n')
                || line[..line.len() - 1].contains(&b'\r')
            {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "journal has an invalid line terminator",
                ));
            }
            let value = crate::protocol::validate_ndjson(&line)
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.to_string()))?;
            validate_record(&value)?;
            records.push(JournalRecord {
                tx_id: value["tx_id"].as_str().unwrap().to_owned(),
                seq: value["seq"].as_u64().unwrap(),
                kind: value["kind"].as_str().unwrap().to_owned(),
                payload: value["payload"].clone(),
            });
        }
        self.file.seek(SeekFrom::End(0))?;
        Ok(records)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn rotate(&mut self) -> io::Result<()> {
        self.file.sync_data()?;
        let rotated = self.path.with_extension("ndjson.rotated");
        std::fs::rename(&self.path, rotated)?;
        self.file = OpenOptions::new()
            .create(true)
            .append(true)
            .read(true)
            .open(&self.path)?;
        self.file.sync_data()
    }

    pub fn cleanup(&mut self) -> io::Result<()> {
        self.file.sync_data()?;
        let _ = std::fs::remove_file(&self.path);
        Ok(())
    }
}

fn base_object(record: &JournalRecord, digest: Option<String>) -> Map<String, Value> {
    let mut object = Map::new();
    object.insert("kind".into(), Value::String(record.kind.clone()));
    object.insert("magic".into(), Value::String("TSED".into()));
    object.insert("payload".into(), record.payload.clone());
    object.insert("record_len".into(), Value::Number(0.into()));
    object.insert("seq".into(), Value::Number(record.seq.into()));
    if let Some(digest) = digest {
        object.insert("sha256".into(), Value::String(digest));
    }
    object.insert("tx_id".into(), Value::String(record.tx_id.clone()));
    object.insert("version".into(), Value::Number(1.into()));
    object
}

fn validate_record(value: &Value) -> io::Result<()> {
    let object = value
        .as_object()
        .ok_or_else(|| invalid("journal record must be an object"))?;
    if object.get("magic").and_then(Value::as_str) != Some("TSED")
        || object.get("version").and_then(Value::as_u64) != Some(1)
    {
        return Err(invalid(
            "journal record has an unsupported magic or version",
        ));
    }
    for key in ["tx_id", "seq", "kind", "payload", "record_len", "sha256"] {
        if !object.contains_key(key) {
            return Err(invalid("journal record is missing a required field"));
        }
    }
    let declared_len = object["record_len"]
        .as_u64()
        .ok_or_else(|| invalid("journal record_len is not an integer"))?
        as usize;
    let without_digest = {
        let mut preimage = object.clone();
        preimage.remove("sha256");
        canonical_json(&Value::Object(preimage))
    };
    let digest = hex::encode(Sha256::digest(&without_digest));
    if object["sha256"].as_str() != Some(digest.as_str()) {
        return Err(invalid(
            "journal digest does not match the canonical preimage",
        ));
    }
    if canonical_json(value).len() != declared_len {
        return Err(invalid(
            "journal record_len does not match the canonical record",
        ));
    }
    Ok(())
}

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

mod hex {
    pub fn encode(bytes: impl AsRef<[u8]>) -> String {
        const TABLE: &[u8; 16] = b"0123456789abcdef";
        bytes
            .as_ref()
            .iter()
            .flat_map(|byte| {
                [
                    TABLE[(byte >> 4) as usize] as char,
                    TABLE[(byte & 0xf) as usize] as char,
                ]
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn append_and_replay_validates_digest_and_length() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("journal.ndjson");
        let mut journal = Journal::open(&path).unwrap();
        journal
            .append(&JournalRecord {
                tx_id: "tx-1".into(),
                seq: 1,
                kind: "edit".into(),
                payload: serde_json::json!({"offset": 0, "text": "hello"}),
            })
            .unwrap();
        let records = journal.replay().unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].tx_id, "tx-1");
        assert_eq!(records[0].kind, "edit");
    }

    #[test]
    fn replay_rejects_tampered_digest() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("journal.ndjson");
        let mut journal = Journal::open(&path).unwrap();
        journal
            .append(&JournalRecord {
                tx_id: "tx-1".into(),
                seq: 1,
                kind: "edit".into(),
                payload: serde_json::json!({"ok": true}),
            })
            .unwrap();
        std::fs::write(&path, br#"{"kind":"edit","magic":"TSED","payload":{"ok":false},"record_len":0,"seq":1,"sha256":"0000000000000000000000000000000000000000000000000000000000000000","tx_id":"tx-1","version":1}
"#).unwrap();
        assert!(journal.replay().is_err());
    }
}
