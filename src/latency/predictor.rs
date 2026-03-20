use super::{LatencyEstimate, HeuristicPredictor};
use crate::config::Config;
use anyhow::Result;

pub struct LatencyPredictor {
    heuristic: HeuristicPredictor,
    config: Config,
    use_ml: bool,
    observation_count: u32,
}

impl LatencyPredictor {
    pub fn new(config: Config) -> Self {
        let heuristic = HeuristicPredictor::new();
        let use_ml = config.feedback.min_samples_for_ml == 0; // Start with heuristic

        Self {
            heuristic,
            config,
            use_ml,
            observation_count: 0,
        }
    }

    /// Predict latencies for all routes
    pub fn predict(
        &self,
        intent: &str,
        input_tokens: u32,
        output_tokens: u32,
        context_tokens: u32,
        system_load: f64,
    ) -> Result<LatencyEstimate> {
        if self.use_ml && self.observation_count >= self.config.feedback.min_samples_for_ml {
            // TODO: ML prediction (Phase 2)
            // For now, fall back to heuristic
            self.heuristic.predict(
                intent,
                input_tokens,
                output_tokens,
                context_tokens,
                system_load,
            )
        } else {
            self.heuristic.predict(
                intent,
                input_tokens,
                output_tokens,
                context_tokens,
                system_load,
            )
        }
    }

    pub fn record_observation(&mut self, _actual_ms: u32) {
        self.observation_count += 1;
        if self.observation_count >= self.config.feedback.min_samples_for_ml {
            self.use_ml = true;
        }
    }
}
