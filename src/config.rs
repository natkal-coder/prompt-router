use serde::{Deserialize, Serialize};
use std::path::Path;
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub general: GeneralConfig,
    pub session: SessionConfig,
    pub local_model: LocalModelConfig,
    pub latency: LatencyConfig,
    pub patience: PatienceConfig,
    pub routing: RoutingConfig,
    pub cloud_backends: CloudBackendsConfig,
    pub compression: CompressionConfig,
    pub feedback: FeedbackConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    pub log_level: String,
    pub cache_ttl_hours: u32,
    pub cost_budget_daily_usd: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    pub auto_resume: bool,
    pub session_ttl_hours: u32,
    pub max_turns_before_compaction: u32,
    pub sliding_window: SlidingWindowConfig,
    pub importance: ImportanceConfig,
    pub intervention_summary: InterventionSummaryConfig,
    pub thread_tracking: ThreadTrackingConfig,
    pub persistence: PersistenceConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlidingWindowConfig {
    pub tier1_system_tokens: u32,
    pub tier2_summary_max_tokens: u32,
    pub tier3_recent_max_tokens: u32,
    pub tier4_code_max_tokens: u32,
    pub tier5_prompt_max_tokens: u32,
    pub latency_pressure_reduction: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportanceConfig {
    pub mode: String,
    pub evict_below_score: f64,
    pub summarize_batch_size: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterventionSummaryConfig {
    pub enabled: bool,
    pub max_tokens: u32,
    pub trigger_after_local_turns: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadTrackingConfig {
    pub keep_alive_timeout_min: u32,
    pub prefer_thread_reuse: bool,
    pub thread_reuse_latency_bonus_ms: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistenceConfig {
    pub db_path: String,
    pub export_format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalModelConfig {
    pub primary: String,
    pub fast: String,
    pub fallback: String,
    pub inference_engine: String,
    pub max_tokens: u32,
    pub temperature: f64,
    pub gpu_layers: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyConfig {
    pub predictor_mode: String,
    pub ml_model_path: String,
    pub calibration_on_startup: bool,
    pub recalibrate_interval_hours: u32,
    pub system_monitor_interval_ms: u64,
    pub network_probe_interval_s: u64,
    pub prediction_confidence_threshold: f64,
    pub output_estimation: OutputEstimationConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputEstimationConfig {
    pub intent_prior_weight: f64,
    pub input_ratio_weight: f64,
    pub similar_prompt_weight: f64,
    pub verbosity_cue_weight: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatienceConfig {
    pub profile: String,
    pub auto_learn: bool,
    pub instant_threshold_ms: u32,
    pub acceptable_threshold_ms: u32,
    pub tolerable_threshold_ms: u32,
    pub abandon_threshold_ms: u32,
    pub streaming_patience_multiplier: f64,
    pub ttft_critical_ms: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingConfig {
    pub weights: RoutingWeights,
    pub force_cloud_intents: Vec<String>,
    pub force_local_intents: Vec<String>,
    pub race_mode_confidence_threshold: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingWeights {
    pub latency: f64,
    pub quality: f64,
    pub cost: f64,
    pub reliability: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudBackendsConfig {
    pub preferred_order: Vec<String>,
    pub gemini_cli: CloudBackendConfig,
    pub claude_code: CloudBackendConfig,
    pub cursor_agent: CloudBackendConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudBackendConfig {
    pub enabled: bool,
    pub command: String,
    pub max_tokens: u32,
    pub timeout_ms: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionConfig {
    pub enabled: bool,
    pub strip_imports: bool,
    pub summarize_threshold_lines: u32,
    pub max_context_tokens: u32,
    pub rewrite_prompts: bool,
    pub latency_aware: bool,
    pub aggressive_compression_headroom_ms: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackConfig {
    pub enabled: bool,
    pub db_path: String,
    pub retrain_interval_days: u32,
    pub min_samples_for_ml: u32,
    pub drift_alert_mae_threshold: f64,
}

impl Config {
    /// Load config from YAML file
    pub fn from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = serde_yaml::from_str(&content)?;
        Ok(config)
    }

    /// Load from default location or use hardcoded defaults
    pub fn load_or_default() -> Result<Self> {
        if let Ok(config_path) = std::env::var("LOKAHI_CONFIG") {
            Self::from_file(Path::new(&config_path))
        } else {
            Ok(Self::default())
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            general: GeneralConfig {
                log_level: "info".to_string(),
                cache_ttl_hours: 24,
                cost_budget_daily_usd: 2.0,
            },
            session: SessionConfig {
                auto_resume: true,
                session_ttl_hours: 72,
                max_turns_before_compaction: 200,
                sliding_window: SlidingWindowConfig {
                    tier1_system_tokens: 400,
                    tier2_summary_max_tokens: 1500,
                    tier3_recent_max_tokens: 6000,
                    tier4_code_max_tokens: 6000,
                    tier5_prompt_max_tokens: 2800,
                    latency_pressure_reduction: 0.6,
                },
                importance: ImportanceConfig {
                    mode: "heuristic".to_string(),
                    evict_below_score: 0.3,
                    summarize_batch_size: 5,
                },
                intervention_summary: InterventionSummaryConfig {
                    enabled: true,
                    max_tokens: 400,
                    trigger_after_local_turns: 3,
                },
                thread_tracking: ThreadTrackingConfig {
                    keep_alive_timeout_min: 30,
                    prefer_thread_reuse: true,
                    thread_reuse_latency_bonus_ms: 1500,
                },
                persistence: PersistenceConfig {
                    db_path: "./data/sessions.db".to_string(),
                    export_format: "markdown".to_string(),
                },
            },
            local_model: LocalModelConfig {
                primary: "qwen2.5-coder-7b-q4".to_string(),
                fast: "codegemma-2b-q4".to_string(),
                fallback: "phi3-mini-q4".to_string(),
                inference_engine: "ollama".to_string(),
                max_tokens: 4096,
                temperature: 0.2,
                gpu_layers: 35,
            },
            latency: LatencyConfig {
                predictor_mode: "heuristic".to_string(),
                ml_model_path: "./data/latency_model.bin".to_string(),
                calibration_on_startup: true,
                recalibrate_interval_hours: 168,
                system_monitor_interval_ms: 1000,
                network_probe_interval_s: 60,
                prediction_confidence_threshold: 0.60,
                output_estimation: OutputEstimationConfig {
                    intent_prior_weight: 0.30,
                    input_ratio_weight: 0.25,
                    similar_prompt_weight: 0.35,
                    verbosity_cue_weight: 0.10,
                },
            },
            patience: PatienceConfig {
                profile: "default".to_string(),
                auto_learn: true,
                instant_threshold_ms: 2000,
                acceptable_threshold_ms: 5000,
                tolerable_threshold_ms: 15000,
                abandon_threshold_ms: 30000,
                streaming_patience_multiplier: 1.8,
                ttft_critical_ms: 3000,
            },
            routing: RoutingConfig {
                weights: RoutingWeights {
                    latency: 0.40,
                    quality: 0.30,
                    cost: 0.15,
                    reliability: 0.15,
                },
                force_cloud_intents: vec!["security_audit".to_string(), "architecture_design".to_string()],
                force_local_intents: vec![
                    "explain_code".to_string(),
                    "format_code".to_string(),
                    "write_docstring".to_string(),
                    "git_commit_message".to_string(),
                ],
                race_mode_confidence_threshold: 0.60,
            },
            cloud_backends: CloudBackendsConfig {
                preferred_order: vec![
                    "gemini_cli".to_string(),
                    "claude_code".to_string(),
                    "cursor_agent".to_string(),
                ],
                gemini_cli: CloudBackendConfig {
                    enabled: true,
                    command: "gemini".to_string(),
                    max_tokens: 8192,
                    timeout_ms: 20000,
                },
                claude_code: CloudBackendConfig {
                    enabled: true,
                    command: "claude".to_string(),
                    max_tokens: 8192,
                    timeout_ms: 30000,
                },
                cursor_agent: CloudBackendConfig {
                    enabled: false,
                    command: "cursor".to_string(),
                    max_tokens: 8192,
                    timeout_ms: 45000,
                },
            },
            compression: CompressionConfig {
                enabled: true,
                strip_imports: true,
                summarize_threshold_lines: 100,
                max_context_tokens: 4000,
                rewrite_prompts: true,
                latency_aware: true,
                aggressive_compression_headroom_ms: 1000,
            },
            feedback: FeedbackConfig {
                enabled: true,
                db_path: "./data/feedback.db".to_string(),
                retrain_interval_days: 7,
                min_samples_for_ml: 200,
                drift_alert_mae_threshold: 0.40,
            },
        }
    }
}
