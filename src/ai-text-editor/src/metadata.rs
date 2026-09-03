// MODE: DEV
// PACKAGE: PROD
use crate::document::DocumentMode;
use crate::index::{IndexBlock, LineIndex};
use rusqlite::{params, Connection, OptionalExtension};
use std::io;
use std::path::{Path, PathBuf};

pub struct Metadata {
    connection: Connection,
    path: PathBuf,
}

impl Metadata {
    pub fn open(file: &Path) -> io::Result<Self> {
        let root = std::env::var_os("TSCH_AI_EDITOR_METADATA_DIR")
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("HOME")
                    .map(|home| PathBuf::from(home).join(".config/tsch-ai-skills/editor"))
            })
            .or_else(|| {
                std::env::var_os("USERPROFILE")
                    .map(|home| PathBuf::from(home).join("codex/tsch-ai-skills/editor"))
            })
            .unwrap_or_else(|| std::env::temp_dir().join("tsch-ai-skills/editor"));
        std::fs::create_dir_all(&root)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o700))?;
        }
        let identity = std::fs::canonicalize(file).unwrap_or_else(|_| file.to_path_buf());
        let key = blake3::hash(identity.to_string_lossy().as_bytes())
            .to_hex()
            .to_string();
        let path = root.join(format!("tab-{key}.sqlite"));
        let connection = Connection::open(&path).map_err(sqlite_error)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
        }
        connection
            .pragma_update(None, "synchronous", "NORMAL")
            .map_err(sqlite_error)?;
        connection
            .pragma_update(None, "journal_mode", "WAL")
            .map_err(sqlite_error)?;
        connection
            .pragma_update(None, "temp_store", "FILE")
            .map_err(sqlite_error)?;
        connection
            .busy_timeout(std::time::Duration::from_secs(5))
            .map_err(sqlite_error)?;
        connection.execute_batch("CREATE TABLE IF NOT EXISTS tab (id INTEGER PRIMARY KEY CHECK (id = 1), path TEXT NOT NULL, mode TEXT NOT NULL, revision INTEGER NOT NULL, byte_len INTEGER NOT NULL, updated_at TEXT NOT NULL)")
            .map_err(sqlite_error)?;
        connection.execute_batch("CREATE TABLE IF NOT EXISTS line_index (id INTEGER PRIMARY KEY CHECK (id = 1), granularity INTEGER NOT NULL, byte_len INTEGER NOT NULL, line_count INTEGER NOT NULL, checksum TEXT NOT NULL, complete INTEGER NOT NULL, revision INTEGER NOT NULL, updated_at TEXT NOT NULL); CREATE TABLE IF NOT EXISTS line_index_block (granularity INTEGER NOT NULL, line INTEGER NOT NULL, byte_offset INTEGER NOT NULL, checksum TEXT NOT NULL, revision INTEGER NOT NULL, PRIMARY KEY(granularity, line)); CREATE TABLE IF NOT EXISTS result_set (result_id TEXT PRIMARY KEY, mode TEXT NOT NULL, query_digest TEXT NOT NULL, revision INTEGER NOT NULL, match_count INTEGER NOT NULL, complete INTEGER NOT NULL, updated_at TEXT NOT NULL); CREATE TABLE IF NOT EXISTS result_match (result_id TEXT NOT NULL, ordinal INTEGER NOT NULL, match_json TEXT NOT NULL, PRIMARY KEY(result_id, ordinal), FOREIGN KEY(result_id) REFERENCES result_set(result_id) ON DELETE CASCADE)")
            .map_err(sqlite_error)?;
        connection
            .execute_batch("PRAGMA foreign_keys=ON")
            .map_err(sqlite_error)?;
        Ok(Self { connection, path })
    }

    pub fn record(
        &self,
        path: &Path,
        mode: DocumentMode,
        revision: u64,
        byte_len: usize,
    ) -> io::Result<()> {
        self.connection.execute(
            "INSERT INTO tab(id,path,mode,revision,byte_len,updated_at) VALUES(1,?1,?2,?3,?4,datetime('now')) ON CONFLICT(id) DO UPDATE SET path=excluded.path, mode=excluded.mode, revision=excluded.revision, byte_len=excluded.byte_len, updated_at=excluded.updated_at",
            params![path.to_string_lossy(), format!("{mode:?}"), revision, byte_len],
        ).map_err(sqlite_error)?;
        Ok(())
    }

    pub fn cleanup(&mut self) -> io::Result<()> {
        let connection = std::mem::replace(
            &mut self.connection,
            Connection::open_in_memory().map_err(sqlite_error)?,
        );
        connection
            .close()
            .map_err(|(_, error)| sqlite_error(error))?;
        remove_if_present(&self.path)?;
        remove_if_present(&self.path.with_extension("sqlite-wal"))?;
        remove_if_present(&self.path.with_extension("sqlite-shm"))?;
        Ok(())
    }

    pub fn record_index(
        &self,
        granularity: usize,
        byte_len: usize,
        line_count: usize,
        checksum: &str,
        complete: bool,
        revision: u64,
    ) -> io::Result<()> {
        self.connection
            .execute(
                "INSERT INTO line_index(id,granularity,byte_len,line_count,checksum,complete,revision,updated_at) VALUES(1,?1,?2,?3,?4,?5,?6,datetime('now')) ON CONFLICT(id) DO UPDATE SET granularity=excluded.granularity,byte_len=excluded.byte_len,line_count=excluded.line_count,checksum=excluded.checksum,complete=excluded.complete,revision=excluded.revision,updated_at=excluded.updated_at",
                params![granularity, byte_len, line_count, checksum, complete as i64, revision],
            )
            .map_err(sqlite_error)?;
        Ok(())
    }

    pub fn record_index_blocks(
        &self,
        blocks: &[IndexBlock],
        granularity: usize,
        checksum: &str,
        revision: u64,
    ) -> io::Result<()> {
        let transaction = self
            .connection
            .unchecked_transaction()
            .map_err(sqlite_error)?;
        transaction
            .execute(
                "DELETE FROM line_index_block WHERE granularity=?1",
                params![granularity],
            )
            .map_err(sqlite_error)?;
        for block in blocks {
            transaction
                .execute(
                    "INSERT INTO line_index_block(granularity,line,byte_offset,checksum,revision) VALUES(?1,?2,?3,?4,?5)",
                    params![granularity, block.line, block.byte_offset, checksum, revision],
                )
                .map_err(sqlite_error)?;
        }
        transaction.commit().map_err(sqlite_error)
    }

    /// Load an index only when every identity field matches the current tab.
    /// A missing, incomplete, stale, or malformed persisted index is treated as
    /// a cache miss; the caller then rebuilds it from the document.
    pub fn load_index(
        &self,
        granularity: usize,
        byte_len: usize,
        checksum: &str,
        revision: u64,
    ) -> io::Result<Option<LineIndex>> {
        let row = self
            .connection
            .query_row(
                "SELECT granularity, byte_len, line_count, checksum, complete, revision FROM line_index WHERE id=1",
                [],
                |row| {
                    Ok((
                        row.get::<_, usize>(0)?,
                        row.get::<_, usize>(1)?,
                        row.get::<_, usize>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, i64>(4)?,
                        row.get::<_, u64>(5)?,
                    ))
                },
            )
            .optional()
            .map_err(sqlite_error)?;
        let Some((
            stored_granularity,
            stored_bytes,
            stored_lines,
            stored_checksum,
            complete,
            stored_revision,
        )) = row
        else {
            return Ok(None);
        };
        if stored_granularity != granularity
            || stored_bytes != byte_len
            || stored_checksum != checksum
            || complete == 0
            || stored_revision != revision
        {
            return Ok(None);
        }
        let mut blocks = Vec::new();
        let mut statement = self
            .connection
            .prepare(
                "SELECT line, byte_offset FROM line_index_block WHERE granularity=?1 AND checksum=?2 AND revision=?3 ORDER BY line",
            )
            .map_err(sqlite_error)?;
        let mut rows = statement
            .query(params![granularity, checksum, revision])
            .map_err(sqlite_error)?;
        while let Some(row) = rows.next().map_err(sqlite_error)? {
            blocks.push(IndexBlock {
                line: row.get(0).map_err(sqlite_error)?,
                byte_offset: row.get(1).map_err(sqlite_error)?,
            });
        }
        if blocks.first()
            != Some(&IndexBlock {
                line: 1,
                byte_offset: 0,
            })
            || blocks
                .iter()
                .any(|block| block.line == 0 || block.byte_offset > byte_len)
        {
            return Ok(None);
        }
        Ok(Some(LineIndex {
            granularity,
            bytes: byte_len,
            lines: stored_lines,
            blocks,
        }))
    }

    pub fn record_result(
        &self,
        result_id: &str,
        mode: &str,
        query_digest: &str,
        revision: u64,
        match_count: usize,
        complete: bool,
    ) -> io::Result<()> {
        self.connection
            .execute(
                "INSERT INTO result_set(result_id,mode,query_digest,revision,match_count,complete,updated_at) VALUES(?1,?2,?3,?4,?5,?6,datetime('now')) ON CONFLICT(result_id) DO UPDATE SET mode=excluded.mode,query_digest=excluded.query_digest,revision=excluded.revision,match_count=excluded.match_count,complete=excluded.complete,updated_at=excluded.updated_at",
                params![result_id, mode, query_digest, revision, match_count, complete as i64],
            )
            .map_err(sqlite_error)?;
        Ok(())
    }

    pub fn record_result_matches(
        &self,
        result_id: &str,
        mode: &str,
        query_digest: &str,
        revision: u64,
        matches: &[serde_json::Value],
        complete: bool,
    ) -> io::Result<()> {
        let transaction = self
            .connection
            .unchecked_transaction()
            .map_err(sqlite_error)?;
        transaction
            .execute(
                "INSERT INTO result_set(result_id,mode,query_digest,revision,match_count,complete,updated_at) VALUES(?1,?2,?3,?4,?5,?6,datetime('now')) ON CONFLICT(result_id) DO UPDATE SET mode=excluded.mode,query_digest=excluded.query_digest,revision=excluded.revision,match_count=excluded.match_count,complete=excluded.complete,updated_at=excluded.updated_at",
                params![result_id, mode, query_digest, revision, matches.len(), complete as i64],
            )
            .map_err(sqlite_error)?;
        transaction
            .execute(
                "DELETE FROM result_match WHERE result_id=?1",
                params![result_id],
            )
            .map_err(sqlite_error)?;
        for (ordinal, value) in matches.iter().enumerate() {
            let encoded = serde_json::to_string(value)
                .map_err(|error| io::Error::other(error.to_string()))?;
            transaction
                .execute(
                    "INSERT INTO result_match(result_id,ordinal,match_json) VALUES(?1,?2,?3)",
                    params![result_id, ordinal, encoded],
                )
                .map_err(sqlite_error)?;
        }
        transaction.commit().map_err(sqlite_error)
    }

    /// Restore a complete result set for the current revision. Invalid or
    /// partial rows are treated as a cache miss so paging can safely request a
    /// fresh search instead of presenting untrusted metadata.
    pub fn load_result_matches(
        &self,
        result_id: &str,
        revision: u64,
    ) -> io::Result<Option<Vec<serde_json::Value>>> {
        let expected = self
            .connection
            .query_row(
                "SELECT match_count FROM result_set WHERE result_id=?1 AND revision=?2 AND complete=1",
                params![result_id, revision],
                |row| row.get::<_, usize>(0),
            )
            .optional()
            .map_err(sqlite_error)?;
        let Some(expected) = expected else {
            return Ok(None);
        };
        let mut statement = self
            .connection
            .prepare("SELECT match_json FROM result_match WHERE result_id=?1 ORDER BY ordinal")
            .map_err(sqlite_error)?;
        let mut rows = statement.query(params![result_id]).map_err(sqlite_error)?;
        let mut matches = Vec::with_capacity(expected);
        while let Some(row) = rows.next().map_err(sqlite_error)? {
            let encoded: String = row.get(0).map_err(sqlite_error)?;
            let value = serde_json::from_str(&encoded).map_err(|_| {
                io::Error::new(io::ErrorKind::InvalidData, "invalid persisted result")
            })?;
            matches.push(value);
        }
        if matches.len() != expected {
            return Ok(None);
        }
        Ok(Some(matches))
    }

    pub fn load_historical_result_matches(
        &self,
        result_id: &str,
    ) -> io::Result<Option<(u64, Vec<serde_json::Value>)>> {
        let Some((revision, expected)) = self
            .connection
            .query_row(
                "SELECT revision, match_count FROM result_set WHERE result_id=?1 AND complete=1",
                params![result_id],
                |row| Ok((row.get::<_, u64>(0)?, row.get::<_, usize>(1)?)),
            )
            .optional()
            .map_err(sqlite_error)?
        else {
            return Ok(None);
        };
        let mut statement = self
            .connection
            .prepare("SELECT match_json FROM result_match WHERE result_id=?1 ORDER BY ordinal")
            .map_err(sqlite_error)?;
        let mut rows = statement.query(params![result_id]).map_err(sqlite_error)?;
        let mut matches = Vec::with_capacity(expected);
        while let Some(row) = rows.next().map_err(sqlite_error)? {
            let encoded: String = row.get(0).map_err(sqlite_error)?;
            let value = serde_json::from_str(&encoded).map_err(|_| {
                io::Error::new(io::ErrorKind::InvalidData, "invalid persisted result")
            })?;
            matches.push(value);
        }
        if matches.len() != expected {
            return Ok(None);
        }
        Ok(Some((revision, matches)))
    }
}

fn remove_if_present(path: &Path) -> io::Result<()> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

fn sqlite_error(error: rusqlite::Error) -> io::Error {
    io::Error::other(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_tab_identity_in_isolated_database() {
        let directory = tempfile::tempdir().unwrap();
        std::env::set_var("TSCH_AI_EDITOR_METADATA_DIR", directory.path());
        let file = directory.path().join("document.txt");
        let metadata = Metadata::open(&file).unwrap();
        metadata
            .record(&file, DocumentMode::TextUtf8, 7, 12)
            .unwrap();
        metadata
            .record_index(10_000, 12, 2, "digest", true, 7)
            .unwrap();
        metadata
            .record_index_blocks(
                &[IndexBlock {
                    line: 1,
                    byte_offset: 0,
                }],
                10_000,
                "digest",
                7,
            )
            .unwrap();
        metadata
            .record_result("result-1", "exact_text", "query", 7, 3, true)
            .unwrap();
        let identity = std::fs::canonicalize(&file).unwrap_or_else(|_| file.clone());
        let database = rusqlite::Connection::open(directory.path().join(format!(
            "tab-{}.sqlite",
            blake3::hash(identity.to_string_lossy().as_bytes()).to_hex()
        )))
        .unwrap();
        let row: (String, i64, i64) = database
            .query_row(
                "SELECT path, revision, byte_len FROM tab WHERE id=1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(row, (file.to_string_lossy().into_owned(), 7, 12));
        let index: (i64, i64, i64) = database
            .query_row(
                "SELECT granularity, byte_len, line_count FROM line_index WHERE id=1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(index, (10_000, 12, 2));
        let block: (i64, i64) = database
            .query_row(
                "SELECT line, byte_offset FROM line_index_block WHERE granularity=10000",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(block, (1, 0));
        let result: (String, i64) = database
            .query_row(
                "SELECT mode, match_count FROM result_set WHERE result_id='result-1'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(result, ("exact_text".into(), 3));
        let loaded = metadata
            .load_index(10_000, 12, "digest", 7)
            .unwrap()
            .unwrap();
        assert_eq!(loaded.lines, 2);
        assert_eq!(loaded.blocks[0].byte_offset, 0);
        assert!(metadata
            .load_index(10_000, 13, "digest", 7)
            .unwrap()
            .is_none());
        metadata
            .record_result_matches(
                "result-2",
                "exact_text",
                "query",
                7,
                &[serde_json::json!({"line": 1})],
                true,
            )
            .unwrap();
        assert_eq!(
            metadata
                .load_result_matches("result-2", 7)
                .unwrap()
                .unwrap(),
            vec![serde_json::json!({"line": 1})]
        );
        assert!(metadata
            .load_result_matches("result-2", 8)
            .unwrap()
            .is_none());
        assert_eq!(
            metadata
                .load_historical_result_matches("result-2")
                .unwrap()
                .unwrap(),
            (7, vec![serde_json::json!({"line": 1})])
        );
        let database_path = metadata.path.clone();
        drop(database);
        let mut metadata = metadata;
        metadata.cleanup().unwrap();
        assert!(!database_path.exists());
        std::env::remove_var("TSCH_AI_EDITOR_METADATA_DIR");
    }
}
