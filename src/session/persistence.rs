use crate::session::models::*;
use anyhow::{Result, anyhow};
use rusqlite::{Connection, params};
use chrono::Utc;
use uuid::Uuid;
use std::path::Path;

pub struct SessionDB {
    conn: Connection,
}

impl SessionDB {
    pub fn new(db_path: &str) -> Result<Self> {
        std::fs::create_dir_all(
            Path::new(db_path).parent().unwrap_or_else(|| Path::new("."))
        )?;

        let conn = Connection::open(db_path)?;
        let db = Self { conn };
        db.init_schema()?;
        Ok(db)
    }

    fn init_schema(&self) -> Result<()> {
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS sessions (
                session_id TEXT PRIMARY KEY,
                created_at TEXT NOT NULL,
                project_root TEXT NOT NULL,
                active_files TEXT NOT NULL,
                metadata TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS turns (
                session_id TEXT NOT NULL,
                turn_id INTEGER NOT NULL,
                timestamp TEXT NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                route_taken TEXT,
                backend_model TEXT,
                token_count INTEGER NOT NULL,
                files_referenced TEXT NOT NULL,
                files_modified TEXT NOT NULL,
                code_blocks TEXT NOT NULL,
                is_summarized INTEGER NOT NULL,
                importance_score REAL NOT NULL,
                PRIMARY KEY (session_id, turn_id),
                FOREIGN KEY (session_id) REFERENCES sessions(session_id)
            );

            CREATE TABLE IF NOT EXISTS summaries (
                id INTEGER PRIMARY KEY,
                session_id TEXT NOT NULL,
                covers_turns_start INTEGER NOT NULL,
                covers_turns_end INTEGER NOT NULL,
                content TEXT NOT NULL,
                token_count INTEGER NOT NULL,
                key_decisions TEXT NOT NULL,
                files_involved TEXT NOT NULL,
                created_at TEXT NOT NULL,
                FOREIGN KEY (session_id) REFERENCES sessions(session_id)
            );

            CREATE TABLE IF NOT EXISTS backend_threads (
                id INTEGER PRIMARY KEY,
                session_id TEXT NOT NULL,
                backend TEXT NOT NULL,
                thread_id TEXT,
                turns_seen TEXT NOT NULL,
                last_context_payload_hash TEXT,
                is_alive INTEGER NOT NULL,
                last_used TEXT NOT NULL,
                FOREIGN KEY (session_id) REFERENCES sessions(session_id),
                UNIQUE (session_id, backend)
            );

            CREATE TABLE IF NOT EXISTS feedback (
                id INTEGER PRIMARY KEY,
                session_id TEXT,
                turn_id INTEGER,
                timestamp TEXT NOT NULL,
                route_taken TEXT NOT NULL,
                predicted_ms INTEGER,
                actual_ms INTEGER NOT NULL,
                tokens_in INTEGER,
                tokens_out INTEGER,
                features TEXT NOT NULL
            );
            "#
        )?;
        Ok(())
    }

    pub fn create_session(&mut self, session: &Session) -> Result<()> {
        let active_files_json = serde_json::to_string(&session.active_files)?;
        let metadata_json = serde_json::to_string(&session.metadata)?;

        self.conn.execute(
            "INSERT INTO sessions (session_id, created_at, project_root, active_files, metadata)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                session.session_id.to_string(),
                session.created_at.to_rfc3339(),
                session.project_root,
                active_files_json,
                metadata_json,
            ],
        )?;
        Ok(())
    }

    pub fn load_session(&self, session_id: &Uuid) -> Result<Session> {
        let mut stmt = self.conn.prepare(
            "SELECT created_at, project_root, active_files, metadata FROM sessions WHERE session_id = ?1"
        )?;

        let session = stmt.query_row(params![session_id.to_string()], |row| {
            let created_at: String = row.get(0)?;
            let project_root: String = row.get(1)?;
            let active_files_json: String = row.get(2)?;
            let metadata_json: String = row.get(3)?;

            let active_files: Vec<String> = serde_json::from_str(&active_files_json).unwrap_or_default();
            let metadata: SessionMetadata = serde_json::from_str(&metadata_json).unwrap_or_default();

            Ok(Session {
                session_id: *session_id,
                created_at: chrono::DateTime::parse_from_rfc3339(&created_at)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                project_root,
                active_files,
                turns: Vec::new(),
                summary_segments: Vec::new(),
                file_snapshots: std::collections::HashMap::new(),
                metadata,
            })
        })?;

        Ok(session)
    }

    pub fn list_sessions(&self, project_root: Option<&str>) -> Result<Vec<(Uuid, String)>> {
        let mut stmt = self.conn.prepare(
            "SELECT session_id, project_root FROM sessions ORDER BY created_at DESC"
        )?;

        let sessions = if let Some(root) = project_root {
            let rows = stmt.query_map(params![root], |row| {
                let id_str: String = row.get(0)?;
                let root: String = row.get(1)?;
                Ok((Uuid::parse_str(&id_str).unwrap_or_default(), root))
            })?;
            rows.collect::<Result<Vec<_>, _>>()?
        } else {
            let rows = stmt.query_map([], |row| {
                let id_str: String = row.get(0)?;
                let root: String = row.get(1)?;
                Ok((Uuid::parse_str(&id_str).unwrap_or_default(), root))
            })?;
            rows.collect::<Result<Vec<_>, _>>()?
        };

        Ok(sessions)
    }

    pub fn add_turn(&mut self, session_id: &Uuid, turn: &Turn) -> Result<()> {
        let files_ref_json = serde_json::to_string(&turn.files_referenced)?;
        let files_mod_json = serde_json::to_string(&turn.files_modified)?;
        let code_blocks_json = serde_json::to_string(&turn.code_blocks)?;

        self.conn.execute(
            "INSERT INTO turns (session_id, turn_id, timestamp, role, content, route_taken, backend_model, token_count, files_referenced, files_modified, code_blocks, is_summarized, importance_score)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                session_id.to_string(),
                turn.turn_id,
                turn.timestamp.to_rfc3339(),
                format!("{:?}", turn.role).to_lowercase(),
                turn.content,
                turn.route_taken.map(|r| format!("{:?}", r).to_lowercase()),
                turn.backend_model,
                turn.token_count,
                files_ref_json,
                files_mod_json,
                code_blocks_json,
                turn.is_summarized as i32,
                turn.importance_score,
            ],
        )?;
        Ok(())
    }

    pub fn load_turns(&self, session_id: &Uuid) -> Result<Vec<Turn>> {
        let mut stmt = self.conn.prepare(
            "SELECT turn_id, timestamp, role, content, route_taken, backend_model, token_count, files_referenced, files_modified, code_blocks, is_summarized, importance_score
             FROM turns WHERE session_id = ?1 ORDER BY turn_id ASC"
        )?;

        let turns = stmt.query_map(params![session_id.to_string()], |row| {
            let role_str: String = row.get(2)?;
            let role = match role_str.as_str() {
                "assistant" => TurnRole::Assistant,
                "system" => TurnRole::System,
                _ => TurnRole::User,
            };

            let route_str: Option<String> = row.get(4)?;
            let route_taken = route_str.and_then(|r| match r.as_str() {
                "local" => Some(RouteTaken::Local),
                "hybrid" => Some(RouteTaken::Hybrid),
                "cloud_gemini" => Some(RouteTaken::CloudGemini),
                "cloud_claude" => Some(RouteTaken::CloudClaude),
                "cloud_cursor" => Some(RouteTaken::CloudCursor),
                _ => None,
            });

            let files_ref_json: String = row.get(7)?;
            let files_mod_json: String = row.get(8)?;
            let code_blocks_json: String = row.get(9)?;

            Ok(Turn {
                turn_id: row.get(0)?,
                timestamp: {
                    let ts: String = row.get(1)?;
                    chrono::DateTime::parse_from_rfc3339(&ts)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now())
                },
                role,
                content: row.get(3)?,
                route_taken,
                backend_model: row.get(5)?,
                token_count: row.get(6)?,
                files_referenced: serde_json::from_str(&files_ref_json).unwrap_or_default(),
                files_modified: serde_json::from_str(&files_mod_json).unwrap_or_default(),
                code_blocks: serde_json::from_str(&code_blocks_json).unwrap_or_default(),
                is_summarized: row.get::<_, i32>(10)? != 0,
                importance_score: row.get(11)?,
            })
        })?;

        let mut result = Vec::new();
        for turn in turns {
            result.push(turn?);
        }
        Ok(result)
    }

    pub fn add_summary(&mut self, session_id: &Uuid, summary: &Summary) -> Result<()> {
        let key_decisions_json = serde_json::to_string(&summary.key_decisions)?;
        let files_involved_json = serde_json::to_string(&summary.files_involved)?;

        self.conn.execute(
            "INSERT INTO summaries (session_id, covers_turns_start, covers_turns_end, content, token_count, key_decisions, files_involved, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                session_id.to_string(),
                summary.covers_turns.0,
                summary.covers_turns.1,
                summary.content,
                summary.token_count,
                key_decisions_json,
                files_involved_json,
                summary.created_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    pub fn load_summaries(&self, session_id: &Uuid) -> Result<Vec<Summary>> {
        let mut stmt = self.conn.prepare(
            "SELECT covers_turns_start, covers_turns_end, content, token_count, key_decisions, files_involved, created_at
             FROM summaries WHERE session_id = ?1 ORDER BY created_at ASC"
        )?;

        let summaries = stmt.query_map(params![session_id.to_string()], |row| {
            let key_decisions_json: String = row.get(4)?;
            let files_involved_json: String = row.get(5)?;

            Ok(Summary {
                covers_turns: (row.get(0)?, row.get(1)?),
                content: row.get(2)?,
                token_count: row.get(3)?,
                key_decisions: serde_json::from_str(&key_decisions_json).unwrap_or_default(),
                files_involved: serde_json::from_str(&files_involved_json).unwrap_or_default(),
                created_at: {
                    let ts: String = row.get(6)?;
                    chrono::DateTime::parse_from_rfc3339(&ts)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now())
                },
            })
        })?;

        let mut result = Vec::new();
        for summary in summaries {
            result.push(summary?);
        }
        Ok(result)
    }

    pub fn log_feedback(
        &mut self,
        session_id: Option<&Uuid>,
        turn_id: Option<u32>,
        route_taken: &str,
        predicted_ms: Option<u32>,
        actual_ms: u32,
        tokens_in: Option<u32>,
        tokens_out: Option<u32>,
        features: &str,
    ) -> Result<()> {
        self.conn.execute(
            "INSERT INTO feedback (session_id, turn_id, timestamp, route_taken, predicted_ms, actual_ms, tokens_in, tokens_out, features)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                session_id.map(|id| id.to_string()),
                turn_id,
                Utc::now().to_rfc3339(),
                route_taken,
                predicted_ms,
                actual_ms,
                tokens_in,
                tokens_out,
                features,
            ],
        )?;
        Ok(())
    }
}

impl Default for SessionMetadata {
    fn default() -> Self {
        Self {
            total_turns: 0,
            total_tokens_used: 0,
            cloud_backends_used: Vec::new(),
            last_active: Utc::now(),
        }
    }
}
