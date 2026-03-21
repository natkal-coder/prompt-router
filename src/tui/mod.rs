use anyhow::Result;
use std::path::PathBuf;
use std::io::{self, Write};
use std::env;
use rustyline::DefaultEditor;

use crate::config::Config;
use crate::session::manager::SessionManager;
use crate::session::models::Turn;
use crate::latency::predictor::LatencyPredictor;
use crate::balancer::{router::Router, Route};
use crate::local::ollama::OllamaClient;
use crate::cloud::claude::ClaudeBackend;
use crate::cloud::gemini::GeminiBackend;
use crate::cloud::adapter::CloudBackend;
use crate::feedback::collector::FeedbackCollector;
use crate::intake::parser::{classify_intent, analyze_prompt};
use crate::session::context_builder::ContextBuilder;

/// Convert host paths to Docker container paths
fn translate_path_for_docker(input: &str) -> String {
    let host_home = env::var("HOST_HOME").unwrap_or_else(|_| "/host".to_string());

    if input.starts_with("~/") {
        return format!("{}/{}", host_home, &input[2..]);
    }
    // /home/rickeshtn/... → /host/rickeshtn/...
    if input.starts_with("/home/") {
        let rest = &input[6..]; // Remove "/home/"
        return format!("{}/{}", host_home, rest);
    }
    input.to_string()
}

/// Extract path from user input if present
fn extract_path_from_input(input: &str) -> Option<String> {
    for word in input.split_whitespace() {
        if word.contains('/') || word.contains('~') {
            // Found a path-like token, strip trailing punctuation
            let path = word.trim_end_matches(|c| matches!(c, '?' | '!' | '.' | ',' | ':' | ';'));
            return Some(path.to_string());
        }
    }
    None
}

pub async fn run(
    project_root: PathBuf,
    session_name: Option<String>,
    _explain: bool,
) -> Result<()> {
    // Initialize systems
    let db_path = project_root.join("sessions.db").to_string_lossy().to_string();
    let config = Config::load_or_default()?;
    let mut session = SessionManager::new(&db_path)?;
    let predictor = LatencyPredictor::new(config.clone());
    let router = Router::new(config.clone());

    let ollama_host = std::env::var("OLLAMA_HOST")
        .unwrap_or_else(|_| "http://localhost:11434".to_string());
    let ollama_model = std::env::var("OLLAMA_MODEL")
        .unwrap_or_else(|_| "tinyllama".to_string());
    let ollama = OllamaClient::new(&ollama_host, &ollama_model);
    let feedback = FeedbackCollector::new(&db_path)?;

    if let Some(_name) = session_name {
        // Resume session
    } else {
        session.create_session(project_root.to_string_lossy().to_string())?;
    }

    // Manage Ollama lifecycle
    let mut ollama_child = if !ollama.is_healthy().await {
        println!("🔧 Starting Ollama...");
        match tokio::process::Command::new("ollama")
            .arg("serve")
            .spawn()
        {
            Ok(child) => {
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                Some(child)
            }
            Err(_) => None,
        }
    } else {
        None
    };

    // Welcome
    println!("\n🚀 LOKAHI - Local LLM Router");
    println!("═══════════════════════════════════════════");
    if let Some(sid) = session.get_session_id() {
        println!("Session: {}", sid);
    }
    println!("Model: {}", ollama_model);
    println!("\nType your prompt (arrow keys for history, Ctrl+C to quit)\n");

    // Main loop with history support
    let mut rl = DefaultEditor::new()?;

    loop {
        let prompt = "❯ You: ";

        match rl.readline(prompt) {
            Ok(input) => {
                let input = input.trim();

                if input.is_empty() {
                    continue;
                }

                // Add to history
                let _ = rl.add_history_entry(input);

                if input == "/quit" || input == "/exit" {
                    println!("\n👋 Goodbye!\n");
                    break;
                }

                if input == "/history" {
                    println!("\n📜 History:");
                    if let Some(sess) = session.current_session() {
                        for turn in &sess.turns {
                            let role = format!("{:?}", turn.role);
                            let preview = turn.content.chars().take(60).collect::<String>();
                            println!("  {} - {}", role, preview);
                        }
                    }
                    println!();
                    continue;
                }

                if input == "/clear" {
                    if let Some(sess) = session.current_session_mut() {
                        sess.turns.clear();
                    }
                    println!("✨ History cleared\n");
                    continue;
                }

                // Translate path for Docker
                let translated_input = if env::var("HOST_HOME").is_ok() {
                    translate_path_for_docker(&input)
                } else {
                    input.to_string()
                };

                // Process
                let analysis = analyze_prompt(&input);
                let intent = classify_intent(&input);

                let estimate = predictor.predict(
                    &format!("{:?}", intent),
                    100,
                    analysis.output_tokens_estimate,
                    100,
                    0.5,
                )?;

                let decision = router.decide(intent, &estimate);

                // Show routing decision with complexity info
                let complexity = if input.len() > 200 || input.matches("~").count() > 2 {
                    "HIGH"
                } else if input.len() > 100 {
                    "MEDIUM"
                } else {
                    "LOW"
                };

                println!("\n🔀 Route: {} | Complexity: {} | Est. {}ms\n",
                    decision.route, complexity, estimate.local_ms);
                print!("🤖 Assistant: ");
                io::stdout().flush().ok();

                // Call Ollama
                if let Some(mut sess) = session.current_session_mut() {
                    // Extract path from input for filesystem context
                    let context_path = if let Some(path) = extract_path_from_input(input) {
                        // If asking about lokahi directory, use /work (current dir in container)
                        if path.contains("lokahi") {
                            "/work".to_string()
                        } else {
                            translate_path_for_docker(&path)
                        }
                    } else {
                        sess.project_root.clone()
                    };

                    // Temporarily set project_root to scanned path
                    let original_root = sess.project_root.clone();
                    sess.project_root = context_path;

                    let backend_name = match decision.route {
                        Route::CloudClaude => "claude",
                        Route::CloudGemini => "gemini",
                        _ => "ollama",
                    };

                    let payload = ContextBuilder::build(&sess, &translated_input, backend_name, &config, false);

                    // Restore original project_root
                    sess.project_root = original_root;

                    // Route to appropriate backend
                    let result = match decision.route {
                        Route::Local => {
                            let prompt = format!(
                                "{}\n{}\n{}\n{}\n{}",
                                payload.tier1_system,
                                payload.tier2_summaries,
                                payload.tier3_recent_turns,
                                payload.tier4_code_context,
                                payload.tier5_prompt
                            );
                            ollama.generate_streaming(&prompt, 512).await
                        }
                        Route::CloudClaude => {
                            let claude = ClaudeBackend::new("claude");
                            match claude.send_streaming(&payload).await {
                                Ok(rx) => Ok(rx),
                                Err(e) => {
                                    eprintln!("Claude unavailable ({}), falling back to local", e);
                                    let prompt = format!(
                                        "{}\n{}\n{}\n{}\n{}",
                                        payload.tier1_system,
                                        payload.tier2_summaries,
                                        payload.tier3_recent_turns,
                                        payload.tier4_code_context,
                                        payload.tier5_prompt
                                    );
                                    ollama.generate_streaming(&prompt, 512).await
                                }
                            }
                        }
                        Route::CloudGemini => {
                            let gemini = GeminiBackend::new("gemini");
                            match gemini.send_streaming(&payload).await {
                                Ok(rx) => Ok(rx),
                                Err(e) => {
                                    eprintln!("Gemini unavailable ({}), falling back to local", e);
                                    let prompt = format!(
                                        "{}\n{}\n{}\n{}\n{}",
                                        payload.tier1_system,
                                        payload.tier2_summaries,
                                        payload.tier3_recent_turns,
                                        payload.tier4_code_context,
                                        payload.tier5_prompt
                                    );
                                    ollama.generate_streaming(&prompt, 512).await
                                }
                            }
                        }
                        Route::Hybrid => {
                            // For now, treat hybrid as local with self-critique (Phase 2)
                            let prompt = format!(
                                "{}\n{}\n{}\n{}\n{}",
                                payload.tier1_system,
                                payload.tier2_summaries,
                                payload.tier3_recent_turns,
                                payload.tier4_code_context,
                                payload.tier5_prompt
                            );
                            ollama.generate_streaming(&prompt, 512).await
                        }
                        Route::CloudCursor => {
                            // Cursor not supported yet, fallback to Claude then local
                            let claude = ClaudeBackend::new("claude");
                            match claude.send_streaming(&payload).await {
                                Ok(rx) => Ok(rx),
                                Err(_) => {
                                    let prompt = format!(
                                        "{}\n{}\n{}\n{}\n{}",
                                        payload.tier1_system,
                                        payload.tier2_summaries,
                                        payload.tier3_recent_turns,
                                        payload.tier4_code_context,
                                        payload.tier5_prompt
                                    );
                                    ollama.generate_streaming(&prompt, 512).await
                                }
                            }
                        }
                    };

                    match result {
                        Ok(mut rx) => {
                            let mut response = String::new();
                            while let Ok(Some(chunk)) = io::Result::Ok(rx.recv().await) {
                                print!("{}", chunk);
                                response.push_str(&chunk);
                                io::stdout().flush().ok();
                            }
                            println!("\n");

                            // Save to history
                            let user_turn = Turn::new_user(input.to_string(), 100);
                            let route_taken = match decision.route {
                                Route::Local | Route::Hybrid => crate::session::models::RouteTaken::Local,
                                Route::CloudClaude => crate::session::models::RouteTaken::CloudClaude,
                                Route::CloudGemini => crate::session::models::RouteTaken::CloudGemini,
                                Route::CloudCursor => crate::session::models::RouteTaken::CloudCursor,
                            };
                            let asst_turn = Turn::new_assistant(
                                response.clone(),
                                response.len() as u32 / 4,
                                route_taken,
                                backend_name.to_string(),
                            );
                            let _ = session.commit_turn(user_turn);
                            let _ = session.commit_turn(asst_turn);
                        }
                        Err(e) => {
                            println!("❌ Error: {}\n", e);
                        }
                    }
                }
            }
            Err(rustyline::error::ReadlineError::Interrupted) => {
                println!("\n👋 Goodbye!\n");
                break;
            }
            Err(rustyline::error::ReadlineError::Eof) => {
                println!("\n👋 Goodbye!\n");
                break;
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                break;
            }
        }
    }

    // Cleanup
    if let Some(mut child) = ollama_child {
        let _ = child.kill();
    }

    Ok(())
}
