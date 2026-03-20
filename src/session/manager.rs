use crate::session::models::*;
use crate::session::persistence::SessionDB;
use anyhow::Result;
use uuid::Uuid;
use std::path::Path;

pub struct SessionManager {
    db: SessionDB,
    current_session: Option<Session>,
}

impl SessionManager {
    pub fn new(db_path: &str) -> Result<Self> {
        let db = SessionDB::new(db_path)?;
        Ok(Self {
            db,
            current_session: None,
        })
    }

    pub fn create_session(&mut self, project_root: String) -> Result<Session> {
        let session = Session::new(project_root);
        self.db.create_session(&session)?;
        self.current_session = Some(session.clone());
        Ok(session)
    }

    pub fn resume_session(&mut self, session_id: Uuid) -> Result<Session> {
        let mut session = self.db.load_session(&session_id)?;
        let turns = self.db.load_turns(&session_id)?;
        let summaries = self.db.load_summaries(&session_id)?;

        session.turns = turns;
        session.summary_segments = summaries;

        self.current_session = Some(session.clone());
        Ok(session)
    }

    pub fn list_sessions(&self, project_root: Option<&str>) -> Result<Vec<(Uuid, String)>> {
        self.db.list_sessions(project_root)
    }

    pub fn commit_turn(&mut self, turn: Turn) -> Result<()> {
        if let Some(ref mut session) = self.current_session {
            let mut turn = turn;
            turn.turn_id = session.metadata.total_turns;

            self.db.add_turn(&session.session_id, &turn)?;
            session.add_turn(turn);
            Ok(())
        } else {
            Err(anyhow::anyhow!("No active session"))
        }
    }

    pub fn current_session(&self) -> Option<&Session> {
        self.current_session.as_ref()
    }

    pub fn current_session_mut(&mut self) -> Option<&mut Session> {
        self.current_session.as_mut()
    }

    pub fn get_session_id(&self) -> Option<Uuid> {
        self.current_session.as_ref().map(|s| s.session_id)
    }
}
