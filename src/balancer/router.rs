use super::{Route, RouteScorer};
use crate::config::Config;
use crate::intake::parser::Intent;
use crate::latency::LatencyEstimate;

pub struct Router {
    config: Config,
    scorer: RouteScorer,
}

#[derive(Debug, Clone)]
pub struct RoutingDecision {
    pub route: Route,
    pub reason: String,
    pub scores: RoutingScores,
}

#[derive(Debug, Clone)]
pub struct RoutingScores {
    pub local: f64,
    pub hybrid: f64,
    pub cloud_gemini: f64,
    pub cloud_claude: f64,
    pub cloud_cursor: f64,
}

impl Router {
    pub fn new(config: Config) -> Self {
        let scorer = RouteScorer::new();
        Self { config, scorer }
    }

    pub fn decide(
        &self,
        intent: Intent,
        latency_estimate: &LatencyEstimate,
    ) -> RoutingDecision {
        let intent_str = format!("{:?}", intent).to_lowercase();

        // Check force-local intents
        if self.config.routing.force_local_intents.iter().any(|i| i.to_lowercase() == intent_str) {
            return RoutingDecision {
                route: Route::Local,
                reason: "Forced LOCAL by intent".to_string(),
                scores: RoutingScores {
                    local: 1.0,
                    hybrid: 0.0,
                    cloud_gemini: 0.0,
                    cloud_claude: 0.0,
                    cloud_cursor: 0.0,
                },
            };
        }

        // Check force-cloud intents
        if self.config.routing.force_cloud_intents.iter().any(|i| i.to_lowercase() == intent_str) {
            return RoutingDecision {
                route: Route::CloudClaude, // Default cloud choice
                reason: "Forced CLOUD by intent".to_string(),
                scores: RoutingScores {
                    local: 0.0,
                    hybrid: 0.0,
                    cloud_gemini: 0.7,
                    cloud_claude: 1.0,
                    cloud_cursor: 0.5,
                },
            };
        }

        // Score all routes
        let local_score = self.scorer.score_local(latency_estimate, &self.config);
        let hybrid_score = self.scorer.score_hybrid(latency_estimate, &self.config);
        let cloud_gemini_score = self.scorer.score_cloud_gemini(latency_estimate, &self.config);
        let cloud_claude_score = self.scorer.score_cloud_claude(latency_estimate, &self.config);
        let cloud_cursor_score = self.scorer.score_cloud_cursor(latency_estimate, &self.config);

        let scores = RoutingScores {
            local: local_score,
            hybrid: hybrid_score,
            cloud_gemini: cloud_gemini_score,
            cloud_claude: cloud_claude_score,
            cloud_cursor: cloud_cursor_score,
        };

        // Pick the highest scoring route
        let (route, reason) = if local_score >= hybrid_score
            && local_score >= cloud_gemini_score
            && local_score >= cloud_claude_score
        {
            (Route::Local, "Highest latency score: LOCAL")
        } else if hybrid_score >= cloud_gemini_score
            && hybrid_score >= cloud_claude_score
            && hybrid_score >= cloud_cursor_score
        {
            (Route::Hybrid, "Highest latency score: HYBRID")
        } else if cloud_claude_score >= cloud_gemini_score && cloud_claude_score >= cloud_cursor_score {
            (Route::CloudClaude, "Highest latency score: CLOUD:claude")
        } else if cloud_gemini_score >= cloud_cursor_score {
            (Route::CloudGemini, "Highest latency score: CLOUD:gemini")
        } else {
            (Route::CloudCursor, "Highest latency score: CLOUD:cursor")
        };

        // Check for race mode (low confidence)
        if latency_estimate.confidence < self.config.routing.race_mode_confidence_threshold {
            // In a real implementation, trigger race mode (run both simultaneously)
            // For Phase 1, just log it
            tracing::info!("Low confidence ({:.2}) - would trigger race mode", latency_estimate.confidence);
        }

        RoutingDecision {
            route,
            reason: reason.to_string(),
            scores,
        }
    }
}
