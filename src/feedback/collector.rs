use crate::session::persistence::SessionDB;
use uuid::Uuid;
use anyhow::Result;

pub struct FeedbackCollector {
    db: SessionDB,
}

impl FeedbackCollector {
    pub fn new(db_path: &str) -> Result<Self> {
        let db = SessionDB::new(db_path)?;
        Ok(Self { db })
    }

    pub fn log_observation(
        &mut self,
        session_id: Option<&Uuid>,
        turn_id: Option<u32>,
        route: &str,
        predicted_ms: Option<u32>,
        actual_ms: u32,
        tokens_in: Option<u32>,
        tokens_out: Option<u32>,
        features: &str,
    ) -> Result<()> {
        self.db.log_feedback(
            session_id,
            turn_id,
            route,
            predicted_ms,
            actual_ms,
            tokens_in,
            tokens_out,
            features,
        )
    }
}
