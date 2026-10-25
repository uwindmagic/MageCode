use crate::codex::{self, CodexLibrary, CodexMessage, CodexSession, CodexSource};
use rusqlite::{params, Connection, OptionalExtension};
use std::{fs, path::PathBuf, time::{SystemTime, UNIX_EPOCH}};

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS tools (
    id TEXT PRIMARY KEY,
    display_name TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS connections (
    id TEXT PRIMARY KEY,
    tool_id TEXT NOT NULL REFERENCES tools(id),
    display_name TEXT NOT NULL,
    source_root TEXT NOT NULL,
    read_only INTEGER NOT NULL DEFAULT 1,
    last_sync_at TEXT
);
CREATE TABLE IF NOT EXISTS hub_projects (
    id TEXT PRIMARY KEY,
    normalized_path TEXT NOT NULL UNIQUE,
    display_name TEXT NOT NULL,
    display_path TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS native_projects (
    id TEXT PRIMARY KEY,
    connection_id TEXT NOT NULL REFERENCES connections(id),
    native_project_id TEXT NOT NULL,
    parent_id TEXT,
    native_path TEXT NOT NULL,
    normalized_path TEXT NOT NULL,
    display_name TEXT NOT NULL,
    UNIQUE(connection_id, native_project_id)
);
CREATE TABLE IF NOT EXISTS project_mappings (
    native_project_id TEXT PRIMARY KEY REFERENCES native_projects(id),
    hub_project_id TEXT NOT NULL REFERENCES hub_projects(id),
    mapping_basis TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS sessions (
    connection_id TEXT NOT NULL REFERENCES connections(id),
    native_session_id TEXT NOT NULL,
    native_project_id TEXT NOT NULL REFERENCES native_projects(id),
    source_locator TEXT NOT NULL,
    source_path TEXT NOT NULL,
    title TEXT NOT NULL,
    archived INTEGER NOT NULL DEFAULT 0,
    source_created_at TEXT,
    source_updated_at TEXT,
    last_seen_run_id TEXT,
    PRIMARY KEY(connection_id, native_session_id)
);
CREATE TABLE IF NOT EXISTS session_project_links (
    connection_id TEXT NOT NULL,
    native_session_id TEXT NOT NULL,
    hub_project_id TEXT NOT NULL REFERENCES hub_projects(id),
    mapping_basis TEXT NOT NULL,
    PRIMARY KEY(connection_id, native_session_id, hub_project_id),
    FOREIGN KEY(connection_id, native_session_id) REFERENCES sessions(connection_id, native_session_id)
);
CREATE TABLE IF NOT EXISTS messages (
    connection_id TEXT NOT NULL,
    native_session_id TEXT NOT NULL,
    native_message_id TEXT NOT NULL,
    role TEXT NOT NULL CHECK(role IN ('user', 'assistant')),
    content TEXT NOT NULL,
    message_order INTEGER NOT NULL,
    source_timestamp TEXT,
    PRIMARY KEY(connection_id, native_session_id, native_message_id),
    FOREIGN KEY(connection_id, native_session_id) REFERENCES sessions(connection_id, native_session_id) ON DELETE CASCADE
);
CREATE TABLE IF NOT EXISTS session_annotations (
    connection_id TEXT NOT NULL,
    native_session_id TEXT NOT NULL,
    purpose TEXT,
    notes TEXT,
    updated_at TEXT,
    PRIMARY KEY(connection_id, native_session_id),
    FOREIGN KEY(connection_id, native_session_id) REFERENCES sessions(connection_id, native_session_id) ON DELETE CASCADE
);
CREATE TABLE IF NOT EXISTS sync_runs (
    id TEXT PRIMARY KEY,
    connection_id TEXT NOT NULL REFERENCES connections(id),
    trigger_kind TEXT NOT NULL,
    status TEXT NOT NULL,
    started_at TEXT NOT NULL,
    finished_at TEXT,
    session_count INTEGER NOT NULL DEFAULT 0,
    message_count INTEGER NOT NULL DEFAULT 0,
    error TEXT
);
CREATE TABLE IF NOT EXISTS sync_cursors (
    connection_id TEXT PRIMARY KEY REFERENCES connections(id),
    source_version TEXT,
    cursor_json TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_sessions_project ON sessions(native_project_id);
CREATE INDEX IF NOT EXISTS idx_messages_session_order ON messages(connection_id, native_session_id, message_order);
"#;

pub struct Store {
    path: PathBuf,
}

impl Store {
    pub fn open_default() -> Result<Self, String> {
        let base = std::env::var_os("MAGECODE_DATA_DIR")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("APPDATA").map(|value| PathBuf::from(value).join("MageCode")))
            .or_else(|| std::env::var_os("HOME").map(|value| PathBuf::from(value).join(".magecode")))
            .ok_or_else(|| "无法确定 MageCode 应用数据目录。".to_string())?;
        Self::open_at(base.join("magecode.db"))
    }

    pub fn open_at(path: PathBuf) -> Result<Self, String> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| format!("无法创建应用数据目录：{error}"))?;
        }
        let store = Self { path };
        let connection = store.connect()?;
        connection.execute_batch(SCHEMA).map_err(db_error)?;
        connection.pragma_update(None, "user_version", 1).map_err(db_error)?;
        Ok(store)
    }

    fn connect(&self) -> Result<Connection, String> {
        let connection = Connection::open(&self.path).map_err(db_error)?;
        connection.busy_timeout(std::time::Duration::from_secs(5)).map_err(db_error)?;
        connection.pragma_update(None, "foreign_keys", "ON").map_err(db_error)?;
        connection.pragma_update(None, "journal_mode", "WAL").map_err(db_error)?;
        Ok(connection)
    }

    pub fn sync_codex(&self, snapshot: CodexLibrary) -> Result<(), String> {
        let mut connection = self.connect()?;
        let transaction = connection.transaction().map_err(db_error)?;
        let now = now_timestamp();
        let nonce = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
        let run_id = format!("manual-{now}-{nonce}");
        transaction.execute(
            "INSERT INTO tools(id, display_name) VALUES('codex', 'Codex') ON CONFLICT(id) DO UPDATE SET display_name=excluded.display_name",
            [],
        ).map_err(db_error)?;
        transaction.execute(
            "INSERT INTO connections(id, tool_id, display_name, source_root, read_only) VALUES('codex-local-default', 'codex', 'Codex 本地', ?1, 1) ON CONFLICT(id) DO UPDATE SET source_root=excluded.source_root",
            params![snapshot.source.root],
        ).map_err(db_error)?;
        transaction.execute(
            "INSERT INTO sync_runs(id, connection_id, trigger_kind, status, started_at) VALUES(?1, 'codex-local-default', 'manual', 'running', ?2)",
            params![run_id, now],
        ).map_err(db_error)?;

        let mut message_count = 0usize;
        for session in &snapshot.sessions {
            let normalized = normalize_project_path(&session.native_project);
            let native_project_id = stable_id("native", &format!("codex-local-default|{normalized}"));
            let hub_project_id = stable_id("hub", &normalized);
            transaction.execute(
                "INSERT INTO native_projects(id, connection_id, native_project_id, native_path, normalized_path, display_name) VALUES(?1, 'codex-local-default', ?2, ?3, ?4, ?5) ON CONFLICT(id) DO UPDATE SET native_path=excluded.native_path, display_name=excluded.display_name",
                params![native_project_id, session.native_project, session.native_project, normalized, session.project_name],
            ).map_err(db_error)?;
            transaction.execute(
                "INSERT INTO hub_projects(id, normalized_path, display_name, display_path) VALUES(?1, ?2, ?3, ?4) ON CONFLICT(normalized_path) DO UPDATE SET display_name=excluded.display_name, display_path=excluded.display_path",
                params![hub_project_id, normalized, session.project_name, session.native_project],
            ).map_err(db_error)?;
            transaction.execute(
                "INSERT INTO project_mappings(native_project_id, hub_project_id, mapping_basis) VALUES(?1, ?2, 'automatic_exact_normalized_path') ON CONFLICT(native_project_id) DO UPDATE SET hub_project_id=excluded.hub_project_id, mapping_basis=excluded.mapping_basis",
                params![native_project_id, hub_project_id],
            ).map_err(db_error)?;
            transaction.execute(
                "INSERT INTO sessions(connection_id, native_session_id, native_project_id, source_locator, source_path, title, archived, source_updated_at, last_seen_run_id) VALUES('codex-local-default', ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8) ON CONFLICT(connection_id, native_session_id) DO UPDATE SET native_project_id=excluded.native_project_id, source_locator=excluded.source_locator, source_path=excluded.source_path, title=excluded.title, archived=excluded.archived, source_updated_at=excluded.source_updated_at, last_seen_run_id=excluded.last_seen_run_id",
                params![session.native_id, native_project_id, session.source_url, session.source_url, session.title, session.archived, session.last_active_at, run_id],
            ).map_err(db_error)?;
            transaction.execute(
                "DELETE FROM messages WHERE connection_id='codex-local-default' AND native_session_id=?1",
                params![session.native_id],
            ).map_err(db_error)?;
            for (index, message) in session.messages.iter().enumerate() {
                transaction.execute(
                    "INSERT INTO messages(connection_id, native_session_id, native_message_id, role, content, message_order, source_timestamp) VALUES('codex-local-default', ?1, ?2, ?3, ?4, ?5, ?6)",
                    params![session.native_id, message.native_id, message.role, message.text, index as i64, message.time],
                ).map_err(db_error)?;
                message_count += 1;
            }
        }
        transaction.execute(
            "UPDATE sync_runs SET status='completed', finished_at=?2, session_count=?3, message_count=?4 WHERE id=?1",
            params![run_id, now, snapshot.sessions.len() as i64, message_count as i64],
        ).map_err(db_error)?;
        transaction.execute(
            "UPDATE connections SET last_sync_at=?1 WHERE id='codex-local-default'",
            params![now],
        ).map_err(db_error)?;
        transaction.execute(
            "INSERT INTO sync_cursors(connection_id, source_version, cursor_json, updated_at) VALUES('codex-local-default', 'codex-jsonl-v1', ?1, ?2) ON CONFLICT(connection_id) DO UPDATE SET source_version=excluded.source_version, cursor_json=excluded.cursor_json, updated_at=excluded.updated_at",
            params![serde_json::json!({"sessionCount": snapshot.sessions.len(), "messageCount": message_count}).to_string(), now],
        ).map_err(db_error)?;
        transaction.commit().map_err(db_error)
    }

    pub fn codex_library(&self) -> Result<CodexLibrary, String> {
        let connection = self.connect()?;
        let source = connection.query_row(
            "SELECT source_root, last_sync_at FROM connections WHERE id='codex-local-default'",
            [],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?)),
        ).optional().map_err(db_error)?;
        let (root, synced_at) = match source {
            Some(value) => value,
            None => (codex::default_root()?.display().to_string(), None),
        };
        let mut statement = connection.prepare(
            "SELECT s.native_session_id, COALESCE(hp.id, np.id), COALESCE(hp.display_name, np.display_name), np.native_path, s.title, s.source_updated_at, s.archived FROM sessions s JOIN native_projects np ON np.id=s.native_project_id LEFT JOIN project_mappings pm ON pm.native_project_id=np.id LEFT JOIN hub_projects hp ON hp.id=pm.hub_project_id WHERE s.connection_id='codex-local-default' ORDER BY s.source_updated_at DESC"
        ).map_err(db_error)?;
        let sessions = statement.query_map([], |row| {
            let native_id: String = row.get(0)?;
            let last_active_at: Option<String> = row.get(5)?;
            Ok(CodexSession {
                id: format!("codex:{native_id}"),
                native_id: native_id.clone(),
                tool: "chatgpt".to_string(),
                tool_name: "ChatGPT".to_string(),
                project: row.get(1)?,
                project_name: row.get(2)?,
                native_project: row.get(3)?,
                title: row.get(4)?,
                purpose: "本机 Codex 只读会话；仅同步用户消息与明确标记的最终答复。".to_string(),
                updated: last_active_at.clone().unwrap_or_else(|| "未知".to_string()),
                last_active_at,
                archived: row.get::<_, i64>(6)? != 0,
                source: "codex-local".to_string(),
                source_url: format!("codex://threads/{native_id}"),
                messages: Vec::new(),
                tasks: Vec::new(),
                files: Vec::new(),
            })
        }).map_err(db_error)?.collect::<Result<Vec<_>, _>>().map_err(db_error)?;
        Ok(CodexLibrary {
            source: CodexSource { root, loaded_at: synced_at.unwrap_or_else(|| "尚未同步".to_string()), read_only: true },
            sessions,
        })
    }

    pub fn codex_messages(&self, native_id: &str) -> Result<Vec<CodexMessage>, String> {
        codex::validate_native_id(native_id)?;
        let connection = self.connect()?;
        let mut statement = connection.prepare(
            "SELECT native_message_id, role, content, source_timestamp FROM messages WHERE connection_id='codex-local-default' AND native_session_id=?1 ORDER BY message_order"
        ).map_err(db_error)?;
        let messages = statement.query_map(params![native_id], |row| {
            Ok(CodexMessage {
                native_id: row.get(0)?,
                role: row.get(1)?,
                text: row.get(2)?,
                time: row.get::<_, Option<String>>(3)?.unwrap_or_default(),
            })
        }).map_err(db_error)?.collect::<Result<Vec<_>, _>>().map_err(db_error)?;
        Ok(messages)
    }
}

pub fn normalize_project_path(value: &str) -> String {
    let replaced = value.trim().replace('/', "\\");
    let is_drive_root = replaced.len() == 3 && replaced.as_bytes().get(1) == Some(&b':') && replaced.ends_with('\\');
    let trimmed = if is_drive_root { replaced } else { replaced.trim_end_matches('\\').to_string() };
    if cfg!(windows) { trimmed.to_lowercase() } else { trimmed }
}

fn stable_id(prefix: &str, value: &str) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in value.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{prefix}-{hash:016x}")
}

fn now_timestamp() -> String {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs().to_string()
}

fn db_error(error: rusqlite::Error) -> String {
    format!("MageCode 数据库操作失败：{error}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_windows_paths_without_merging_different_directories() {
        assert_eq!(normalize_project_path("H:/Projects/Demo/"), normalize_project_path("h:\\projects\\demo"));
        assert_ne!(normalize_project_path("H:\\Projects\\Demo"), normalize_project_path("H:\\Projects\\Demo\\Child"));
        assert_eq!(normalize_project_path("H:\\"), "h:\\");
    }

    #[test]
    fn repeated_sync_is_idempotent_and_preserves_annotations() {
        let root = std::env::temp_dir().join(format!("magecode-store-test-{}", std::process::id()));
        let path = root.join("test.db");
        let store = Store::open_at(path.clone()).unwrap();
        let snapshot = CodexLibrary {
            source: CodexSource { root: "test".into(), loaded_at: "1".into(), read_only: true },
            sessions: vec![CodexSession {
                id: "codex:01a17456-ac5d-7592-a93e-e8430a661fea".into(),
                native_id: "01a17456-ac5d-7592-a93e-e8430a661fea".into(), tool: "chatgpt".into(), tool_name: "ChatGPT".into(),
                project: "temporary".into(), project_name: "Demo".into(), native_project: "H:\\Projects\\Demo".into(),
                title: "Test".into(), purpose: String::new(), updated: "1".into(), last_active_at: Some("1".into()), archived: false,
                source: "codex-local".into(), source_url: "codex://threads/01a17456-ac5d-7592-a93e-e8430a661fea".into(),
                messages: vec![CodexMessage { native_id: "m1".into(), role: "user".into(), text: "Visible".into(), time: "1".into() }], tasks: vec![], files: vec![],
            }],
        };
        store.sync_codex(snapshot.clone()).unwrap();
        store.connect().unwrap().execute("INSERT INTO session_annotations(connection_id, native_session_id, purpose) VALUES('codex-local-default', ?1, 'Keep')", params![snapshot.sessions[0].native_id]).unwrap();
        store.sync_codex(snapshot).unwrap();
        let connection = store.connect().unwrap();
        assert_eq!(connection.query_row("SELECT count(*) FROM sessions", [], |row| row.get::<_, i64>(0)).unwrap(), 1);
        assert_eq!(connection.query_row("SELECT count(*) FROM messages", [], |row| row.get::<_, i64>(0)).unwrap(), 1);
        assert_eq!(connection.query_row("SELECT purpose FROM session_annotations", [], |row| row.get::<_, String>(0)).unwrap(), "Keep");
        drop(connection);
        drop(store);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    #[ignore = "reads private Codex history and writes a temporary filtered snapshot; run explicitly"]
    fn syncs_default_source_into_temporary_database() {
        let snapshot = codex::collect_snapshot().expect("default Codex source should sync");
        assert!(!snapshot.sessions.is_empty());
        assert!(snapshot.sessions.iter().any(|session| !session.messages.is_empty()));
        let root = std::env::temp_dir().join(format!("magecode-real-sync-test-{}", std::process::id()));
        let store = Store::open_at(root.join("test.db")).unwrap();
        store.sync_codex(snapshot).unwrap();
        let library = store.codex_library().unwrap();
        assert!(!library.sessions.is_empty());
        assert!(library.sessions.iter().any(|session| store.codex_messages(&session.native_id).map(|messages| !messages.is_empty()).unwrap_or(false)));
        drop(store);
        let _ = fs::remove_dir_all(root);
    }
}
