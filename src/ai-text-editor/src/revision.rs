// MODE: DEV
// PACKAGE: PROD
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum RevisionError {
    #[error("stale revision: the tab is at revision {actual}; this request supplied {expected} - re-read open or history and retry with the current revision")]
    Stale { expected: u64, actual: u64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RevisionGuard(pub u64);

impl RevisionGuard {
    pub fn check(self, actual: u64) -> Result<(), RevisionError> {
        if self.0 == actual {
            Ok(())
        } else {
            Err(RevisionError::Stale {
                expected: self.0,
                actual,
            })
        }
    }
}
