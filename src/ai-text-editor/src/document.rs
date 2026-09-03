// MODE: DEV
// PACKAGE: PROD
use serde::{Deserialize, Serialize};
use thiserror::Error;
use unicode_normalization::UnicodeNormalization;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum DocumentMode {
    TextUtf8,
    RawBytes,
    HexView,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct Coordinate {
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum DocumentError {
    #[error("document contains invalid UTF-8")]
    InvalidUtf8,
    #[error("hex edits must address complete byte pairs")]
    InvalidHexCoordinate,
    #[error("restoration of original bytes is no longer lossless")]
    RestorationConflict,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    bytes: Vec<u8>,
    pub mode: DocumentMode,
    pub normalize_nfc: bool,
    pub mapping_revision: u64,
    pub restoration_conflict: bool,
}

impl Document {
    pub fn new(bytes: Vec<u8>, mode: DocumentMode) -> Result<Self, DocumentError> {
        if matches!(mode, DocumentMode::TextUtf8) && std::str::from_utf8(&bytes).is_err() {
            return Err(DocumentError::InvalidUtf8);
        }
        Ok(Self {
            bytes,
            mode,
            normalize_nfc: false,
            mapping_revision: 0,
            restoration_conflict: false,
        })
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn text(&self) -> Result<String, DocumentError> {
        let text = std::str::from_utf8(&self.bytes).map_err(|_| DocumentError::InvalidUtf8)?;
        Ok(if self.normalize_nfc {
            text.nfc().collect()
        } else {
            text.to_owned()
        })
    }

    pub fn coordinate(&self, offset: usize) -> Result<Coordinate, DocumentError> {
        if offset > self.bytes.len() {
            return Err(DocumentError::InvalidHexCoordinate);
        }
        match self.mode {
            DocumentMode::TextUtf8 => {
                // Coordinates always address the stored byte representation;
                // normalized presentation text can have different byte spans.
                let text =
                    std::str::from_utf8(&self.bytes).map_err(|_| DocumentError::InvalidUtf8)?;
                let prefix = text.get(..offset).ok_or(DocumentError::InvalidUtf8)?;
                let line = prefix.matches('\n').count() + 1;
                let column = prefix
                    .rsplit('\n')
                    .next()
                    .unwrap_or_default()
                    .chars()
                    .count();
                Ok(Coordinate { line, column })
            }
            DocumentMode::RawBytes => {
                let line_start = self.bytes[..offset]
                    .iter()
                    .rposition(|byte| *byte == b'\n')
                    .map(|index| index + 1)
                    .unwrap_or(0);
                Ok(Coordinate {
                    line: self.bytes[..offset]
                        .iter()
                        .filter(|byte| **byte == b'\n')
                        .count()
                        + 1,
                    column: offset - line_start,
                })
            }
            DocumentMode::HexView => Ok(Coordinate {
                line: offset / 16 + 1,
                column: offset % 16,
            }),
        }
    }

    pub fn enable_nfc(&mut self) -> Result<(), DocumentError> {
        self.text()?;
        self.normalize_nfc = true;
        self.mapping_revision = self.mapping_revision.saturating_add(1);
        Ok(())
    }

    pub fn restore_original(&mut self) -> Result<(), DocumentError> {
        if self.restoration_conflict {
            return Err(DocumentError::RestorationConflict);
        }
        self.normalize_nfc = false;
        self.mapping_revision = self.mapping_revision.saturating_add(1);
        Ok(())
    }

    pub fn apply_bytes(
        &mut self,
        offset: usize,
        delete_len: usize,
        replacement: &[u8],
    ) -> Result<(), DocumentError> {
        if offset > self.bytes.len() || delete_len > self.bytes.len() - offset {
            return Err(DocumentError::InvalidHexCoordinate);
        }
        if self.mode == DocumentMode::HexView
            && (offset > self.bytes.len() || offset.saturating_add(delete_len) > self.bytes.len())
        {
            return Err(DocumentError::InvalidHexCoordinate);
        }
        let mut candidate = self.bytes.clone();
        candidate.splice(offset..offset + delete_len, replacement.iter().copied());
        if self.mode == DocumentMode::TextUtf8 && std::str::from_utf8(&candidate).is_err() {
            return Err(DocumentError::InvalidUtf8);
        }
        self.bytes = candidate;
        if self.normalize_nfc {
            self.restoration_conflict = true;
        }
        self.mapping_revision = self.mapping_revision.saturating_add(1);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lossless_normalization_can_be_restored_and_lossy_edits_refuse() {
        let mut document =
            Document::new("Cafe\u{301}".as_bytes().to_vec(), DocumentMode::TextUtf8).unwrap();
        document.enable_nfc().unwrap();
        document.restore_original().unwrap();
        assert!(!document.normalize_nfc);

        document.enable_nfc().unwrap();
        document.apply_bytes(0, 0, b"x").unwrap();
        assert_eq!(
            document.restore_original(),
            Err(DocumentError::RestorationConflict)
        );
    }
}
