mod config;
mod session;
mod intake;
mod latency;
mod balancer;
mod local;
mod cloud;
mod tui;
mod feedback;
mod assembler;

use anyhow::Result;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<()> {
    // Simple argument parsing (avoid clap version issues)
    let args: Vec<String> = std::env::args().collect();

    let mut project_root = std::env::current_dir().unwrap();
    let mut session_name: Option<String> = None;
    let mut explain = false;
    let mut headless = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--session" if i + 1 < args.len() => {
                i += 1;
                session_name = Some(args[i].clone());
            }
            "--project" if i + 1 < args.len() => {
                i += 1;
                project_root = PathBuf::from(&args[i]);
            }
            "--explain" => explain = true,
            "--headless" => headless = true,
            _ => {}
        }
        i += 1;
    }

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing_subscriber::filter::LevelFilter::INFO)
        .init();

    tracing::info!("LOKAHI starting in {:?}", project_root);

    if headless {
        tracing::info!("Headless mode (not implemented yet)");
    } else {
        // Launch TUI
        tui::run(project_root, session_name, explain).await?;
    }

    Ok(())
}
