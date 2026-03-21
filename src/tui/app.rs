use crate::config::Config;
use crate::session::manager::SessionManager;
use crate::session::models::Turn;
use crate::latency::predictor::LatencyPredictor;
use crate::balancer::router::Router;
use crate::intake::parser::{classify_intent, analyze_prompt};
use crate::local::ollama::OllamaClient;
use crate::cloud::adapter::CloudBackend;
use crate::cloud::claude::ClaudeBackend;
use crate::cloud::gemini::GeminiBackend;
use crate::feedback::collector::FeedbackCollector;
use crate::session::context_builder::ContextBuilder;
use anyhow::Result;
use std::time::Instant;
use tokio::sync::mpsc;
use tokio::process::Child;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AppState {
    Idle,
    Processing,
    Streaming,
}

#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct StatusBar {
    pub route: String,
    pub predicted_ms: u32,
    pub actual_ms: Option<u32>,
    pub cost_usd: f64,
    pub session_depth: u32,
}

impl Default for StatusBar {
    fn default() -> Self {
        Self {
            route: "IDLE".to_string(),
            predicted_ms: 0,
            actual_ms: None,
            cost_usd: 0.0,
            session_depth: 0,
        }
    }
}

pub struct App {
    // UI state
    pub messages: Vec<ChatMessage>,
    pub input: String,
    pub scroll: u16,
    pub state: AppState,
    pub streaming_buf: String,
    pub status: StatusBar,
    pub should_quit: bool,

    // Core systems
    pub config: Config,
    pub session: SessionManager,
    pub predictor: LatencyPredictor,
    pub router: Router,
    pub ollama: OllamaClient,
    pub feedback: FeedbackCollector,
    pub claude_backend: ClaudeBackend,
    pub gemini_backend: GeminiBackend,

    // Active stream receiver
    pub stream_rx: Option<mpsc::Receiver<String>>,
    pub submit_start: Option<Instant>,

    // Ollama lifecycle
    pub ollama_child: Option<Child>,
}

impl App {
    pub fn new(
        config: Config,
        session: SessionManager,
        predictor: LatencyPredictor,
        router: Router,
        ollama: OllamaClient,
        feedback: FeedbackCollector,
        _project_root: std::path::PathBuf,
    ) -> Self {
        let claude_backend = ClaudeBackend::new("claude");
        let gemini_backend = GeminiBackend::new("gemini");

        Self {
            messages: Vec::new(),
            input: String::new(),
            scroll: 0,
            state: AppState::Idle,
            streaming_buf: String::new(),
            status: StatusBar::default(),
            should_quit: false,
            config,
            session,
            predictor,
            router,
            ollama,
            feedback,
            claude_backend,
            gemini_backend,
            stream_rx: None,
            submit_start: None,
            ollama_child: None,
        }
    }

    pub async fn submit_input(
        &mut self,
        user_input: String,
        translated_input: String,
        tx: tokio::sync::mpsc::UnboundedSender<String>,
    ) -> Result<()> {
        if user_input.trim().is_empty() {
            return Ok(());
        }

        self.state = AppState::Streaming;
        self.streaming_buf.clear();
        self.submit_start = Some(Instant::now());

        // Add user message to chat
        self.messages.push(ChatMessage {
            role: "you".to_string(),
            content: user_input.clone(),
        });

        // 1. Analyze prompt
        let analysis = analyze_prompt(&user_input);
        let intent = classify_intent(&user_input);

        // 2. Estimate tokens (crude estimates)
        let input_tokens = (user_input.len() / 4) as u32;
        let output_tokens = analysis.output_tokens_estimate;
        let session = self.session.current_session().ok_or(anyhow::anyhow!("no session"))?;
        let context_tokens = (session.turns.len() as u32) * 100; // rough estimate

        // 3. Predict latency
        let system_load = 0.5; // stub
        let estimate = self.predictor.predict(
            &format!("{:?}", intent),
            input_tokens,
            output_tokens,
            context_tokens,
            system_load,
        )?;

        // 4. Build context payload BEFORE routing (for context-aware complexity)
        let payload = ContextBuilder::build(session, &translated_input, "claude", &self.config, false);
        let context_metrics = payload.metrics();

        // 5. Route decision with context-aware scoring
        let decision = self.router.decide(intent.clone(), &estimate, &context_metrics);
        self.status.route = format!("{:?}", decision.route);
        self.status.predicted_ms = (if decision.route.to_string().contains("Cloud") {
            estimate.cloud_ms
        } else {
            estimate.local_ms
        }) as u32;
        self.status.session_depth = session.turns.len() as u32;

        // 6. Spawn backend call and get receiver directly
        let route_clone = decision.route.clone();
        let payload_clone = payload.clone();
        let ollama_clone = self.ollama.clone();
        let claude_clone = self.claude_backend.clone();
        let gemini_clone = self.gemini_backend.clone();

        let backend_rx = if route_clone.to_string().contains("Local") {
            // Concatenate context payload to string
            let prompt = format!(
                "{}\n{}\n{}\n{}\n{}",
                payload_clone.tier1_system,
                payload_clone.tier2_summaries,
                payload_clone.tier3_recent_turns,
                payload_clone.tier4_code_context,
                payload_clone.tier5_prompt
            );
            ollama_clone.generate_streaming(&prompt, 512).await.ok()
        } else if route_clone.to_string().contains("Claude") {
            claude_clone.send_streaming(&payload_clone).await.ok()
        } else if route_clone.to_string().contains("Gemini") {
            gemini_clone.send_streaming(&payload_clone).await.ok()
        } else {
            let prompt = format!("{}\n{}", payload_clone.tier1_system, payload_clone.tier5_prompt);
            ollama_clone.generate_streaming(&prompt, 512).await.ok()
        };

        if let Some(rx) = backend_rx {
            self.stream_rx = Some(rx);
        }

        Ok(())
    }

    pub fn handle_char(&mut self, c: char) {
        if c == '\n' {
            // Enter is handled separately in input.rs
            return;
        }
        self.input.push(c);
    }

    pub fn handle_backspace(&mut self) {
        self.input.pop();
    }

    pub fn finalize_streaming(&mut self) -> Result<()> {
        let _session = self.session.current_session().ok_or(anyhow::anyhow!("no session"))?;

        // Add assistant message to chat
        self.messages.push(ChatMessage {
            role: "LOCAL".to_string(),
            content: self.streaming_buf.clone(),
        });

        // Commit turns to database
        let user_turn = Turn::new_user(
            self.messages[self.messages.len() - 2].content.clone(),
            100,
        );
        let assistant_turn = Turn::new_assistant(
            self.streaming_buf.clone(),
            self.streaming_buf.len() as u32 / 4,
            crate::session::models::RouteTaken::Local,
            "ollama".to_string(),
        );

        self.session.commit_turn(user_turn)?;
        self.session.commit_turn(assistant_turn)?;

        // Log feedback
        if let Some(start) = self.submit_start {
            let elapsed_ms = start.elapsed().as_millis() as u32;
            self.status.actual_ms = Some(elapsed_ms);
            let _ = self.feedback.log_observation(
                self.session.get_session_id().as_ref(),
                None,
                &self.status.route,
                Some(self.status.predicted_ms),
                elapsed_ms,
                Some(100),
                Some(self.streaming_buf.len() as u32 / 4),
                "",
            );
        }

        self.state = AppState::Idle;
        self.stream_rx = None;
        self.submit_start = None;

        Ok(())
    }

    pub fn scroll_up(&mut self) {
        self.scroll = self.scroll.saturating_add(1);
    }

    pub fn scroll_down(&mut self) {
        self.scroll = self.scroll.saturating_sub(1);
    }

    pub fn clear_chat(&mut self) {
        self.messages.clear();
    }

    pub fn cleanup_ollama(&mut self) {
        if let Some(mut child) = self.ollama_child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}
