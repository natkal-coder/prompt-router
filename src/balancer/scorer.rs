use crate::config::Config;
use crate::latency::LatencyEstimate;

pub struct RouteScorer;

impl RouteScorer {
    pub fn new() -> Self {
        Self
    }

    pub fn score_local(&self, latency: &LatencyEstimate, config: &Config) -> f64 {
        let latency_score = if latency.local_ms <= config.patience.instant_threshold_ms {
            1.0
        } else if latency.local_ms <= config.patience.acceptable_threshold_ms {
            0.8
        } else if latency.local_ms <= config.patience.tolerable_threshold_ms {
            0.5
        } else {
            0.0
        };

        let quality_score = 0.6; // Local models are generally lower quality
        let cost_score = 1.0;   // Free to run locally
        let reliability_score = 0.9;

        self.weighted_score(
            latency_score,
            quality_score,
            cost_score,
            reliability_score,
            config,
        )
    }

    pub fn score_hybrid(&self, latency: &LatencyEstimate, config: &Config) -> f64 {
        let latency_score = if latency.hybrid_ms <= config.patience.instant_threshold_ms {
            1.0
        } else if latency.hybrid_ms <= config.patience.acceptable_threshold_ms {
            0.9
        } else if latency.hybrid_ms <= config.patience.tolerable_threshold_ms {
            0.7
        } else {
            0.2
        };

        let quality_score = 0.85; // Better than local alone
        let cost_score = 0.5;    // Some cloud cost
        let reliability_score = 0.95;

        self.weighted_score(
            latency_score,
            quality_score,
            cost_score,
            reliability_score,
            config,
        )
    }

    pub fn score_cloud_gemini(&self, latency: &LatencyEstimate, config: &Config) -> f64 {
        let latency_score = if latency.cloud_ms <= config.patience.acceptable_threshold_ms {
            1.0
        } else if latency.cloud_ms <= config.patience.tolerable_threshold_ms {
            0.8
        } else {
            0.4
        };

        let quality_score = 0.95;
        let cost_score = 0.7;  // Gemini pricing
        let reliability_score = 0.95;

        self.weighted_score(
            latency_score,
            quality_score,
            cost_score,
            reliability_score,
            config,
        )
    }

    pub fn score_cloud_claude(&self, latency: &LatencyEstimate, config: &Config) -> f64 {
        let latency_score = if latency.cloud_ms <= config.patience.acceptable_threshold_ms {
            1.0
        } else if latency.cloud_ms <= config.patience.tolerable_threshold_ms {
            0.8
        } else {
            0.4
        };

        let quality_score = 1.0; // Highest quality
        let cost_score = 0.6;    // Claude pricing (higher than Gemini)
        let reliability_score = 0.98;

        self.weighted_score(
            latency_score,
            quality_score,
            cost_score,
            reliability_score,
            config,
        )
    }

    pub fn score_cloud_cursor(&self, _latency: &LatencyEstimate, _config: &Config) -> f64 {
        0.5 // Disabled by default in Phase 1
    }

    fn weighted_score(
        &self,
        latency: f64,
        quality: f64,
        cost: f64,
        reliability: f64,
        config: &Config,
    ) -> f64 {
        let w = &config.routing.weights;
        (latency * w.latency) + (quality * w.quality) + (cost * w.cost) + (reliability * w.reliability)
    }
}

impl Default for RouteScorer {
    fn default() -> Self {
        Self::new()
    }
}
