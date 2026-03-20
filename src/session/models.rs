use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc, serde::ts_seconds};
use uuid::Uuid;
use std::collections::HashMap;

/// A session represents one continuous conversation with LOKAHI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub session_id: Uuid,
    #[serde(with = "ts_seconds")]
    pub created_at: DateTime<Utc>,
    pub project_root: String,
    pub active_files: Vec<String>,
    pub turns: Vec<Turn>,
    pub summary_segments: Vec<Summary>,
    pub file_snapshots: HashMap<String, String>, // path -> content_hash
    pub metadata: SessionMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetadata {
    pub total_turns: u32,
    pub total_tokens_used: u32,
    pub cloud_backends_used: Vec<String>,
    #[serde(with = "ts_seconds")]
    pub last_active: DateTime<Utc>,
}

/// A single conversational turn (user prompt or assistant response)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Turn {
    pub turn_id: u32,
    #[serde(with = "ts_seconds")]
    pub timestamp: DateTime<Utc>,
    pub role: TurnRole,
    pub content: String,
    pub route_taken: Option<RouteTaken>,
    pub backend_model: Option<String>,
    pub token_count: u32,
    pub files_referenced: Vec<String>,
    pub files_modified: Vec<String>,
    pub code_blocks: Vec<CodeBlock>,
    pub is_summarized: bool,
    pub importance_score: f64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TurnRole {
    User,
    Assistant,
    System,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RouteTaken {
    Local,
    Hybrid,
    CloudGemini,
    CloudClaude,
    CloudCursor,
}

impl std::fmt::Display for RouteTaken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Local => write!(f, "LOCAL"),
            Self::Hybrid => write!(f, "HYBRID"),
            Self::CloudGemini => write!(f, "CLOUD:gemini"),
            Self::CloudClaude => write!(f, "CLOUD:claude"),
            Self::CloudCursor => write!(f, "CLOUD:cursor"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeBlock {
    pub language: String,
    pub content: String,
    pub file_path: Option<String>,
    pub line_range: Option<(usize, usize)>,
}

/// A compressed summary of evicted turns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Summary {
    pub covers_turns: (u32, u32), // (start, end) turn IDs
    pub content: String,
    pub token_count: u32,
    pub key_decisions: Vec<String>,
    pub files_involved: Vec<String>,
    #[serde(with = "ts_seconds")]
    pub created_at: DateTime<Utc>,
}

/// Per-backend thread state for context optimization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendThread {
    pub backend: String,
    pub thread_id: Option<String>,
    pub turns_seen: Vec<u32>,
    pub last_context_payload_hash: Option<String>,
    pub is_alive: bool,
    #[serde(with = "ts_seconds")]
    pub last_used: DateTime<Utc>,
}

impl Session {
    pub fn new(project_root: String) -> Self {
        Self {
            session_id: Uuid::new_v4(),
            created_at: Utc::now(),
            project_root,
            active_files: Vec::new(),
            turns: Vec::new(),
            summary_segments: Vec::new(),
            file_snapshots: HashMap::new(),
            metadata: SessionMetadata {
                total_turns: 0,
                total_tokens_used: 0,
                cloud_backends_used: Vec::new(),
                last_active: Utc::now(),
            },
        }
    }

    pub fn add_turn(&mut self, mut turn: Turn) {
        turn.turn_id = self.metadata.total_turns;
        self.metadata.total_turns += 1;
        self.metadata.total_tokens_used += turn.token_count;
        self.metadata.last_active = Utc::now();
        self.turns.push(turn);
    }

    pub fn latest_turn(&self) -> Option<&Turn> {
        self.turns.last()
    }
}

impl Turn {
    pub fn new_user(content: String, token_count: u32) -> Self {
        Self {
            turn_id: 0,
            timestamp: Utc::now(),
            role: TurnRole::User,
            content,
            route_taken: None,
            backend_model: None,
            token_count,
            files_referenced: Vec::new(),
            files_modified: Vec::new(),
            code_blocks: Vec::new(),
            is_summarized: false,
            importance_score: 0.5,
        }
    }

    pub fn new_assistant(content: String, token_count: u32, route: RouteTaken, backend: String) -> Self {
        Self {
            turn_id: 0,
            timestamp: Utc::now(),
            role: TurnRole::Assistant,
            content,
            route_taken: Some(route),
            backend_model: Some(backend),
            token_count,
            files_referenced: Vec::new(),
            files_modified: Vec::new(),
            code_blocks: Vec::new(),
            is_summarized: false,
            importance_score: 0.5,
        }
    }

}
