pub mod predictor;
pub mod heuristic;
pub mod ml_model;
pub mod system_monitor;
pub mod calibrator;

pub use predictor::LatencyPredictor;
pub use heuristic::HeuristicPredictor;

#[derive(Debug, Clone)]
pub struct LatencyEstimate {
    pub local_ms: u32,
    pub hybrid_ms: u32,
    pub cloud_ms: u32,
    pub confidence: f64,
}

#[derive(Debug, Clone, Default)]
pub struct ContextMetrics {
    pub conversation_history_tokens: u32,  // tier2 + tier3
    pub num_turns: u32,                    // total turns in session
    pub num_active_files: u32,             // files being tracked
    pub total_payload_tokens: u32,         // entire context payload
    pub code_context_tokens: u32,          // tier4 tokens
}
