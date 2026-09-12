// MODE: DEV
// PACKAGE: PROD
//! Derives a file's outbound CodeGraph references ("jump points") by reading
//! CodeGraph's own on-disk SQLite index directly, rather than shelling out to
//! `codegraph` per symbol (its CLI has no bulk per-file JSON command).
//!
//! Grounded in CodeGraph's own source (`~/git/codegraph`, a tracked fork of
//! `colbymchenry/codegraph`): `.codegraph/codegraph.db` is a plain SQLite
//! file with `nodes(id, kind, name, file_path, start_line, ...)` and
//! `edges(source, target, kind, line, col, ...)`, where `source`/`target` are
//! node ids and `line`/`col` are the reference's own location (e.g. the call
//! site), not the target's. `nodes.file_path` is project-root-relative,
//! forward-slash normalized -- the same form `codegraph.ts` normalizes a
//! user-supplied path to before querying.
//!
//! This reader never assumes a CodeGraph schema *version*: every migration
//! from v2 through the current v9 (`src/db/migrations.ts`) only adds
//! columns, tables, or indexes, never renames or removes the ones read here,
//! but a CodeGraph install older than its SQLite-backed storage
//! (pre-~v0.9.0, per its own CHANGELOG) or some future restructuring this
//! reader has not been updated for would not have them at all. So instead of
//! branching on a version number, [`derive`] runtime-checks the actual
//! on-disk shape via `PRAGMA table_info` before querying, and degrades
//! softly (`JumpPointsError::UnsupportedSchema`) rather than crashing or
//! returning a raw SQL error when a column it needs is missing.

use rusqlite::{params, params_from_iter, Connection, OpenFlags};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Where CodeGraph's index lives, relative to a project root.
pub const CODEGRAPH_DB_RELATIVE_PATH: &str = ".codegraph/codegraph.db";

/// The outbound edge kinds CodeGraph's own `getCallees`
/// (`src/graph/traversal.ts`) treats as "you called or used this", excluding
/// `contains` (structural nesting) and the type-annotation kinds (`extends`,
/// `implements`, `type_of`, `returns`, `overrides`, `decorates`, `exports`).
const OUTBOUND_EDGE_KINDS: &[&str] = &[
    "calls",
    "references",
    "imports",
    "instantiates",
    "navigates",
];

/// Chunk size for a SQLite `IN (...)` list, mirroring
/// `queries.ts`'s own `getOutgoingEdgesFrom` batching.
const SQLITE_IN_CLAUSE_CHUNK: usize = 500;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct JumpPoint {
    pub line: u32,
    pub col: u32,
    pub edge_kind: String,
    pub target_file: String,
    pub target_line: u32,
    pub target_name: String,
    pub target_kind: String,
}

#[derive(Debug, Error)]
pub enum JumpPointsError {
    #[error("codegraph is not enabled for this project (no .codegraph/codegraph.db)")]
    NoIndex,
    #[error("codegraph.db is missing an expected column: {0}")]
    UnsupportedSchema(String),
    #[error("codegraph.db query failed: {0}")]
    Db(#[from] rusqlite::Error),
}

/// Walks up from `file_path` (inclusive of its own directory) looking for a
/// `.codegraph` directory -- the project root CodeGraph itself would resolve
/// `file_path` against.
pub fn find_project_root(file_path: &Path) -> Option<PathBuf> {
    let mut dir = if file_path.is_dir() {
        Some(file_path.to_path_buf())
    } else {
        file_path.parent().map(Path::to_path_buf)
    };
    while let Some(candidate) = dir {
        if candidate.join(".codegraph").is_dir() {
            return Some(candidate);
        }
        dir = candidate.parent().map(Path::to_path_buf);
    }
    None
}

/// The version-robustness mechanism: confirms the specific columns `derive`
/// depends on actually exist, rather than trusting a schema version number.
/// `PRAGMA table_info` returns zero rows for a table that does not exist at
/// all, so a missing table is reported the same way as a missing column.
fn schema_supports(conn: &Connection) -> Result<(), JumpPointsError> {
    check_columns(
        conn,
        "nodes",
        &["id", "kind", "name", "file_path", "start_line"],
    )?;
    check_columns(conn, "edges", &["source", "target", "kind", "line", "col"])?;
    Ok(())
}

fn check_columns(conn: &Connection, table: &str, required: &[&str]) -> Result<(), JumpPointsError> {
    let mut statement = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let mut rows = statement.query([])?;
    let mut present: HashSet<String> = HashSet::new();
    while let Some(row) = rows.next()? {
        present.insert(row.get::<_, String>(1)?);
    }
    for column in required {
        if !present.contains(*column) {
            return Err(JumpPointsError::UnsupportedSchema(format!(
                "{table}.{column}"
            )));
        }
    }
    Ok(())
}

/// Project-root-relative, forward-slash normalized -- the form CodeGraph's
/// own indexer stores in `nodes.file_path`.
fn relative_file_path(project_root: &Path, file_path: &Path) -> Option<String> {
    let relative = file_path.strip_prefix(project_root).ok()?;
    Some(relative.to_string_lossy().replace('\\', "/"))
}

struct Edge {
    target: String,
    kind: String,
    line: u32,
    col: u32,
}

struct TargetNode {
    file_path: String,
    start_line: u32,
    name: String,
    kind: String,
}

/// Derives every outbound jump point `file_path` (inside `project_root`)
/// carries in CodeGraph's index: one entry per outbound edge whose source is
/// a node (file- or symbol-level) defined in this file, resolved to where
/// each edge's target is actually defined. Sorted by the referring line.
pub fn derive(project_root: &Path, file_path: &Path) -> Result<Vec<JumpPoint>, JumpPointsError> {
    let db_path = project_root.join(CODEGRAPH_DB_RELATIVE_PATH);
    if !db_path.is_file() {
        return Err(JumpPointsError::NoIndex);
    }
    let Some(relative) = relative_file_path(project_root, file_path) else {
        return Ok(Vec::new());
    };

    let conn = Connection::open_with_flags(&db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    schema_supports(&conn)?;

    let mut source_ids: Vec<String> = {
        let mut statement = conn.prepare("SELECT id FROM nodes WHERE file_path = ?1")?;
        let mut rows = statement.query(params![relative])?;
        let mut ids = Vec::new();
        while let Some(row) = rows.next()? {
            ids.push(row.get::<_, String>(0)?);
        }
        ids
    };
    source_ids.sort();
    source_ids.dedup();
    if source_ids.is_empty() {
        return Ok(Vec::new());
    }

    let mut edges: Vec<Edge> = Vec::new();
    for chunk in source_ids.chunks(SQLITE_IN_CLAUSE_CHUNK) {
        let id_placeholders = std::iter::repeat_n("?", chunk.len())
            .collect::<Vec<_>>()
            .join(",");
        let kind_placeholders = std::iter::repeat_n("?", OUTBOUND_EDGE_KINDS.len())
            .collect::<Vec<_>>()
            .join(",");
        let sql = format!(
            "SELECT target, kind, line, col FROM edges WHERE source IN ({id_placeholders}) AND kind IN ({kind_placeholders}) AND line IS NOT NULL"
        );
        let mut statement = conn.prepare(&sql)?;
        let mut bound: Vec<String> = chunk.to_vec();
        bound.extend(OUTBOUND_EDGE_KINDS.iter().map(|kind| kind.to_string()));
        let mut rows = statement.query(params_from_iter(bound.iter()))?;
        while let Some(row) = rows.next()? {
            edges.push(Edge {
                target: row.get(0)?,
                kind: row.get(1)?,
                line: row.get::<_, i64>(2)? as u32,
                col: row.get::<_, Option<i64>>(3)?.unwrap_or(0) as u32,
            });
        }
    }
    if edges.is_empty() {
        return Ok(Vec::new());
    }

    let mut target_ids: Vec<String> = edges.iter().map(|edge| edge.target.clone()).collect();
    target_ids.sort();
    target_ids.dedup();

    let mut targets: HashMap<String, TargetNode> = HashMap::new();
    for chunk in target_ids.chunks(SQLITE_IN_CLAUSE_CHUNK) {
        let placeholders = std::iter::repeat_n("?", chunk.len())
            .collect::<Vec<_>>()
            .join(",");
        let sql = format!(
            "SELECT id, file_path, start_line, name, kind FROM nodes WHERE id IN ({placeholders})"
        );
        let mut statement = conn.prepare(&sql)?;
        let mut rows = statement.query(params_from_iter(chunk.iter()))?;
        while let Some(row) = rows.next()? {
            let id: String = row.get(0)?;
            targets.insert(
                id,
                TargetNode {
                    file_path: row.get(1)?,
                    start_line: row.get::<_, i64>(2)? as u32,
                    name: row.get(3)?,
                    kind: row.get(4)?,
                },
            );
        }
    }

    let mut jump_points: Vec<JumpPoint> = edges
        .into_iter()
        .filter_map(|edge| {
            let target = targets.get(&edge.target)?;
            Some(JumpPoint {
                line: edge.line,
                col: edge.col,
                edge_kind: edge.kind,
                target_file: target.file_path.clone(),
                target_line: target.start_line,
                target_name: target.name.clone(),
                target_kind: target.kind.clone(),
            })
        })
        .collect();
    jump_points.sort_by_key(|point| point.line);
    Ok(jump_points)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seed_real_schema(conn: &Connection) {
        conn.execute_batch(
            "CREATE TABLE nodes (
                id TEXT PRIMARY KEY,
                kind TEXT NOT NULL,
                name TEXT NOT NULL,
                qualified_name TEXT NOT NULL,
                file_path TEXT NOT NULL,
                language TEXT NOT NULL,
                start_line INTEGER NOT NULL,
                end_line INTEGER NOT NULL,
                start_column INTEGER NOT NULL,
                end_column INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );
            CREATE TABLE edges (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                source TEXT NOT NULL,
                target TEXT NOT NULL,
                kind TEXT NOT NULL,
                metadata TEXT,
                line INTEGER,
                col INTEGER,
                provenance TEXT DEFAULT NULL
            );",
        )
        .unwrap();
    }

    fn seed_fixture(conn: &Connection) {
        seed_real_schema(conn);
        conn.execute(
            "INSERT INTO nodes VALUES ('file:src/a.rs','file','a.rs','a.rs','src/a.rs','rust',1,10,0,0,0)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO nodes VALUES ('sym:foo','function','foo','a::foo','src/a.rs','rust',3,5,0,0,0)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO nodes VALUES ('sym:bar','function','bar','b::bar','src/b.rs','rust',7,9,0,0,0)",
            [],
        )
        .unwrap();
        // foo calls bar: a real jump point.
        conn.execute(
            "INSERT INTO edges (source,target,kind,line,col) VALUES ('sym:foo','sym:bar','calls',4,8)",
            [],
        )
        .unwrap();
        // file contains foo: structural, must never surface.
        conn.execute(
            "INSERT INTO edges (source,target,kind,line,col) VALUES ('file:src/a.rs','sym:foo','contains',3,0)",
            [],
        )
        .unwrap();
        // foo references bar with no line info: must be skipped.
        conn.execute(
            "INSERT INTO edges (source,target,kind,line,col) VALUES ('sym:foo','sym:bar','references',NULL,NULL)",
            [],
        )
        .unwrap();
    }

    fn temp_project(with_index: impl FnOnce(&Connection)) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join(".codegraph")).unwrap();
        let conn = Connection::open(dir.path().join(CODEGRAPH_DB_RELATIVE_PATH)).unwrap();
        with_index(&conn);
        dir
    }

    #[test]
    fn derives_the_real_outbound_jump_point_and_skips_structural_and_null_line_edges() {
        let project = temp_project(seed_fixture);
        let points = derive(project.path(), &project.path().join("src/a.rs")).unwrap();
        assert_eq!(points.len(), 1);
        let point = &points[0];
        assert_eq!(point.line, 4);
        assert_eq!(point.col, 8);
        assert_eq!(point.edge_kind, "calls");
        assert_eq!(point.target_file, "src/b.rs");
        assert_eq!(point.target_line, 7);
        assert_eq!(point.target_name, "bar");
        assert_eq!(point.target_kind, "function");
    }

    #[test]
    fn a_file_with_no_nodes_at_all_yields_no_jump_points() {
        let project = temp_project(seed_real_schema);
        let points = derive(project.path(), &project.path().join("src/unindexed.rs")).unwrap();
        assert!(points.is_empty());
    }

    #[test]
    fn a_missing_codegraph_directory_is_no_index() {
        let dir = tempfile::tempdir().unwrap();
        let error = derive(dir.path(), &dir.path().join("src/a.rs")).unwrap_err();
        assert!(matches!(error, JumpPointsError::NoIndex));
    }

    #[test]
    fn a_database_missing_a_depended_on_column_is_unsupported_schema_not_a_panic() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join(".codegraph")).unwrap();
        let conn = Connection::open(dir.path().join(CODEGRAPH_DB_RELATIVE_PATH)).unwrap();
        // A pre-SQLite-storage-era or unforeseen future shape: no `col` on
        // edges, and no `start_line` on nodes.
        conn.execute_batch(
            "CREATE TABLE nodes (id TEXT PRIMARY KEY, kind TEXT, name TEXT, file_path TEXT);
             CREATE TABLE edges (source TEXT, target TEXT, kind TEXT, line INTEGER);",
        )
        .unwrap();
        let error = derive(dir.path(), &dir.path().join("src/a.rs")).unwrap_err();
        assert!(matches!(error, JumpPointsError::UnsupportedSchema(_)));
    }

    #[test]
    fn find_project_root_walks_up_to_the_nearest_dot_codegraph() {
        let project = temp_project(seed_real_schema);
        let nested = project.path().join("src/deep/nested");
        std::fs::create_dir_all(&nested).unwrap();
        let file = nested.join("leaf.rs");
        assert_eq!(find_project_root(&file).as_deref(), Some(project.path()));
        let outside = tempfile::tempdir().unwrap();
        assert_eq!(find_project_root(outside.path()), None);
    }
}
