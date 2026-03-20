use super::LatencyEstimate;
use anyhow::Result;

pub struct HeuristicPredictor {
    local_tok_per_sec: f64,
    cloud_ttft_ms: u32,
    cloud_tok_per_sec: f64,
    network_rtt_ms: u32,
}

impl HeuristicPredictor {
    pub fn new() -> Self {
        Self {
            local_tok_per_sec: 30.0,     // Will be calibrated
            cloud_ttft_ms: 1000,          // Time to first token
            cloud_tok_per_sec: 50.0,      // Streaming rate after TTFT
            network_rtt_ms: 100,          // Will be calibrated
        }
    }

    pub fn predict(
        &self,
        _intent: &str,
        _input_tokens: u32,
        output_tokens: u32,
        context_tokens: u32,
        system_load: f64,
    ) -> Result<LatencyEstimate> {
        // Adjust for system load (0-1, where 1 is 100% busy)
        let load_factor = 1.0 + (system_load * 0.5);

        // LOCAL latency = (output_tokens / tok_per_sec) + (context_tokens / prefill_rate)
        let local_prefill_rate = self.local_tok_per_sec * 2.0; // Prefill is faster
        let local_ms = (
            ((output_tokens as f64 / self.local_tok_per_sec) * 1000.0) as u32
            + ((context_tokens as f64 / local_prefill_rate) * 1000.0) as u32
        ) as u32;
        let local_ms = (local_ms as f64 * load_factor) as u32;

        // CLOUD latency = TTFT + (output_tokens / tok_per_sec) + RTT
        let cloud_ms = self.cloud_ttft_ms
            + ((output_tokens as f64 / self.cloud_tok_per_sec) * 1000.0) as u32
            + self.network_rtt_ms;

        // HYBRID = local draft starts immediately, cloud refines
        // TTFT is ~200ms (local model instantly), total is faster than cloud alone
        let hybrid_ms = (local_ms as f64 * 0.4).min(200.0) as u32 + (cloud_ms as f64 * 0.6) as u32;

        // Confidence is lower when very uncertain
        let confidence = 0.65;

        Ok(LatencyEstimate {
            local_ms,
            hybrid_ms,
            cloud_ms,
            confidence,
        })
    }

    pub fn set_local_tok_per_sec(&mut self, tok_per_sec: f64) {
        self.local_tok_per_sec = tok_per_sec.max(1.0);
    }

    pub fn set_cloud_ttft_ms(&mut self, ms: u32) {
        self.cloud_ttft_ms = ms;
    }

    pub fn set_cloud_tok_per_sec(&mut self, tok_per_sec: f64) {
        self.cloud_tok_per_sec = tok_per_sec.max(1.0);
    }

    pub fn set_network_rtt_ms(&mut self, ms: u32) {
        self.network_rtt_ms = ms;
    }
}

impl Default for HeuristicPredictor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heuristic_prediction() {
        let predictor = HeuristicPredictor::new();
        let estimate = predictor
            .predict("write_code", 100, 500, 2000, 0.5)
            .expect("predict failed");

        assert!(estimate.local_ms > 0);
        assert!(estimate.cloud_ms > 0);
        assert!(estimate.hybrid_ms > 0);
        assert!(estimate.confidence > 0.0 && estimate.confidence <= 1.0);
    }

    #[test]
    fn test_load_factor() {
        let predictor = HeuristicPredictor::new();

        let low_load = predictor
            .predict("explain_code", 50, 200, 1000, 0.1)
            .expect("predict failed");
        let high_load = predictor
            .predict("explain_code", 50, 200, 1000, 0.9)
            .expect("predict failed");

        assert!(high_load.local_ms > low_load.local_ms);
    }
}
