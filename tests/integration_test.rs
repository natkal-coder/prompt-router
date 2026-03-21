// Basic integration test to verify module compilation and basic functionality

#[test]
fn test_config_default() {
    use lokahi::config::Config;
    let config = Config::default();
    assert!(!config.local_model.primary.is_empty());
    assert!(config.patience.instant_threshold_ms > 0);
}

#[test]
fn test_session_creation() {
    use lokahi::session::Session;
    let mut session = Session::new("/test/project".to_string());
    assert_eq!(session.metadata.total_turns, 0);

    let turn = lokahi::session::Turn::new_user("test prompt".to_string(), 10);
    session.add_turn(turn);
    assert_eq!(session.metadata.total_turns, 1);
}

#[test]
fn test_intent_classification() {
    use lokahi::intake::classify_intent;
    assert_eq!(
        classify_intent("explain how this works"),
        lokahi::intake::parser::Intent::ExplainCode
    );
    assert_eq!(
        classify_intent("write a function"),
        lokahi::intake::parser::Intent::WriteCode
    );
}

#[test]
fn test_heuristic_latency_prediction() {
    use lokahi::latency::HeuristicPredictor;
    let predictor = HeuristicPredictor::new();
    let estimate = predictor
        .predict("write_code", 100, 500, 2000, 0.5)
        .expect("predict failed");

    assert!(estimate.local_ms > 0);
    assert!(estimate.cloud_ms > 0);
    assert!(estimate.hybrid_ms > 0);
}

#[test]
fn test_router_decision() {
    use lokahi::balancer::Router;
    use lokahi::config::Config;
    use lokahi::intake::parser::Intent;
    use lokahi::latency::LatencyEstimate;

    let router = Router::new(Config::default());
    let latency = LatencyEstimate {
        local_ms: 1500,
        hybrid_ms: 3000,
        cloud_ms: 8000,
        confidence: 0.75,
    };

    let context = lokahi::latency::ContextMetrics::default();
    let decision = router.decide(Intent::ExplainCode, &latency, &context);
    // ExplainCode should be forced local
    assert_eq!(decision.route, lokahi::balancer::Route::Local);
}

#[test]
fn test_token_estimation() {
    use lokahi::intake::tokenizer::estimate_tokens;
    let tokens = estimate_tokens("hello world test");
    assert!(tokens > 0);
}

#[test]
fn test_context_builder() {
    use lokahi::session::{Session, ContextBuilder};
    use lokahi::config::Config;

    let session = Session::new("/test/project".to_string());
    let config = Config::default();
    let payload = ContextBuilder::build(&session, "test prompt", "claude", &config, false);

    assert!(!payload.tier1_system.is_empty());
    assert!(!payload.tier5_prompt.is_empty());
    assert!(payload.metadata.total_tokens > 0);
}
