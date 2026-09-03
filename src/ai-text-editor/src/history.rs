// MODE: DEV
// PACKAGE: PROD
use crate::document::Document;

#[derive(Debug, Clone)]
struct Snapshot {
    before: Vec<u8>,
    after: Vec<u8>,
}

#[derive(Debug, Default)]
pub struct History {
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
}

impl History {
    pub fn depths(&self) -> (usize, usize) {
        (self.undo.len(), self.redo.len())
    }

    pub fn record(&mut self, before: &Document, after: &Document) {
        self.undo.push(Snapshot {
            before: before.bytes().to_vec(),
            after: after.bytes().to_vec(),
        });
        self.redo.clear();
    }

    pub fn undo_target(&self) -> Option<&[u8]> {
        self.undo.last().map(|snapshot| snapshot.before.as_slice())
    }

    pub fn redo_target(&self) -> Option<&[u8]> {
        self.redo.last().map(|snapshot| snapshot.after.as_slice())
    }

    pub fn undo(&mut self, document: &mut Document) -> bool {
        let Some(snapshot) = self.undo.pop() else {
            return false;
        };
        let current = document.bytes().to_vec();
        if document
            .apply_bytes(0, document.bytes().len(), &snapshot.before)
            .is_err()
        {
            return false;
        }
        self.redo.push(Snapshot {
            before: current,
            after: snapshot.after,
        });
        true
    }

    pub fn redo(&mut self, document: &mut Document) -> bool {
        let Some(snapshot) = self.redo.pop() else {
            return false;
        };
        let current = document.bytes().to_vec();
        if document
            .apply_bytes(0, document.bytes().len(), &snapshot.after)
            .is_err()
        {
            return false;
        }
        self.undo.push(Snapshot {
            before: current,
            after: snapshot.after,
        });
        true
    }
}
