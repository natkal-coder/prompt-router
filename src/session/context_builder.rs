use crate::session::models::{Session, Turn, Summary, TurnRole};
use crate::config::Config;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// The 5-tier context payload sent to any backend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextPayload {
    pub tier1_system: String,      // Project preamble + active files
    pub tier2_summaries: String,   // Compressed older history
    pub tier3_recent_turns: String, // Verbatim recent turns
    pub tier4_code_context: String, // Current file contents
    pub tier5_prompt: String,       // The actual user prompt
    pub metadata: PayloadMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayloadMetadata {
    pub total_tokens: u32,
    pub tier_tokens: [u32; 5],
    pub session_depth: u32,
    pub compression_factor: f64,
    pub num_active_files: u32,
}

impl ContextPayload {
    /// Extract context metrics for routing decisions
    pub fn metrics(&self) -> crate::latency::ContextMetrics {
        crate::latency::ContextMetrics {
            conversation_history_tokens: self.metadata.tier_tokens[1] + self.metadata.tier_tokens[2],
            num_turns: self.metadata.session_depth,
            num_active_files: self.metadata.num_active_files,
            total_payload_tokens: self.metadata.total_tokens,
            code_context_tokens: self.metadata.tier_tokens[3],
        }
    }
}

impl Default for ContextPayload {
    fn default() -> Self {
        Self {
            tier1_system: String::new(),
            tier2_summaries: String::new(),
            tier3_recent_turns: String::new(),
            tier4_code_context: String::new(),
            tier5_prompt: String::new(),
            metadata: PayloadMetadata {
                total_tokens: 0,
                tier_tokens: [0; 5],
                session_depth: 0,
                compression_factor: 1.0,
                num_active_files: 0,
            },
        }
    }
}

pub struct ContextBuilder;

impl ContextBuilder {
    /// Assemble a complete context payload for a backend
    pub fn build(
        session: &Session,
        current_prompt: &str,
        backend: &str,
        config: &Config,
        latency_pressure: bool,
    ) -> ContextPayload {
        let mut tier_tokens = [0u32; 5];

        // Determine token budget for this backend
        let budget = Self::get_budget(backend, config, latency_pressure);

        // Tier 1: System preamble (fixed)
        let tier1 = Self::build_tier1(session);
        tier_tokens[0] = Self::estimate_tokens(&tier1);
        let mut remaining = budget.total.saturating_sub(tier_tokens[0]);

        // Tier 5: Current prompt (always included)
        let tier5 = current_prompt.to_string();
        tier_tokens[4] = Self::estimate_tokens(&tier5);
        remaining = remaining.saturating_sub(tier_tokens[4]);

        // Tier 4: Code context
        let tier4 = Self::build_tier4(session, remaining / 2);
        tier_tokens[3] = Self::estimate_tokens(&tier4);
        remaining = remaining.saturating_sub(tier_tokens[3]);

        // Tier 3: Recent turns (greedily fill)
        let tier3 = Self::build_tier3(session, remaining.min(budget.tier3));
        tier_tokens[2] = Self::estimate_tokens(&tier3);
        remaining = remaining.saturating_sub(tier_tokens[2]);

        // Tier 2: Summaries
        let tier2 = Self::build_tier2(session, remaining.min(budget.tier2));
        tier_tokens[1] = Self::estimate_tokens(&tier2);

        let total_tokens = tier_tokens.iter().sum();
        let compression_factor = if !session.turns.is_empty() {
            let raw_tokens: u32 = session.turns.iter().map(|t| t.token_count).sum();
            if raw_tokens > 0 {
                total_tokens as f64 / raw_tokens as f64
            } else {
                1.0
            }
        } else {
            1.0
        };

        ContextPayload {
            tier1_system: tier1,
            tier2_summaries: tier2,
            tier3_recent_turns: tier3,
            tier4_code_context: tier4,
            tier5_prompt: tier5,
            metadata: PayloadMetadata {
                total_tokens,
                tier_tokens,
                session_depth: session.metadata.total_turns,
                compression_factor,
                num_active_files: session.active_files.len() as u32,
            },
        }
    }

    fn get_budget(backend: &str, config: &Config, latency_pressure: bool) -> BudgetConfig {
        let mut budget = match backend {
            "claude" => BudgetConfig {
                total: config.session.sliding_window.tier1_system_tokens
                    + config.session.sliding_window.tier2_summary_max_tokens * 2
                    + config.session.sliding_window.tier3_recent_max_tokens
                    + config.session.sliding_window.tier4_code_max_tokens
                    + config.session.sliding_window.tier5_prompt_max_tokens,
                tier1: config.session.sliding_window.tier1_system_tokens,
                tier2: config.session.sliding_window.tier2_summary_max_tokens * 2,
                tier3: config.session.sliding_window.tier3_recent_max_tokens,
                tier4: config.session.sliding_window.tier4_code_max_tokens,
                tier5: config.session.sliding_window.tier5_prompt_max_tokens,
            },
            "gemini" => BudgetConfig {
                total: 32000,
                tier1: 500,
                tier2: 2000,
                tier3: 12000,
                tier4: 12000,
                tier5: 5500,
            },
            "cursor" => BudgetConfig {
                total: 20000,
                tier1: 400,
                tier2: 1200,
                tier3: 6000,
                tier4: 8000,
                tier5: 4400,
            },
            _ => BudgetConfig {
                total: config.session.sliding_window.tier1_system_tokens
                    + config.session.sliding_window.tier2_summary_max_tokens
                    + config.session.sliding_window.tier3_recent_max_tokens
                    + config.session.sliding_window.tier4_code_max_tokens
                    + config.session.sliding_window.tier5_prompt_max_tokens,
                tier1: config.session.sliding_window.tier1_system_tokens,
                tier2: config.session.sliding_window.tier2_summary_max_tokens,
                tier3: config.session.sliding_window.tier3_recent_max_tokens,
                tier4: config.session.sliding_window.tier4_code_max_tokens,
                tier5: config.session.sliding_window.tier5_prompt_max_tokens,
            },
        };

        if latency_pressure {
            let factor = config.session.sliding_window.latency_pressure_reduction;
            budget.total = (budget.total as f64 * factor) as u32;
            budget.tier2 = (budget.tier2 as f64 * factor) as u32;
            budget.tier3 = (budget.tier3 as f64 * factor) as u32;
            budget.tier4 = (budget.tier4 as f64 * factor) as u32;
        }

        budget
    }

    fn build_tier1(session: &Session) -> String {
        let mut result = format!("Project: {}\nActive files: {}",
            session.project_root,
            session.active_files.join(", "));

        // Add directory structure if available
        if let Ok(structure) = Self::scan_directory(&session.project_root, 3) {
            if !structure.is_empty() {
                result.push_str("\n\nProject Structure:\n");
                result.push_str(&structure);
            }
        }

        result
    }

    /// Scan a directory recursively and format as tree structure
    fn scan_directory(path: &str, max_depth: usize) -> Result<String, std::io::Error> {
        let mut output = String::new();
        Self::scan_dir_recursive(Path::new(path), 0, max_depth, &mut output)?;
        Ok(output)
    }

    fn scan_dir_recursive(
        path: &Path,
        depth: usize,
        max_depth: usize,
        output: &mut String,
    ) -> Result<(), std::io::Error> {
        if depth > max_depth {
            return Ok(());
        }

        let indent = "  ".repeat(depth);

        match fs::read_dir(path) {
            Ok(entries) => {
            let mut items: Vec<_> = entries.collect();
            items.sort_by_key(|a| {
                a.as_ref()
                    .ok()
                    .and_then(|e| e.file_name().into_string().ok())
                    .unwrap_or_default()
            });

            for entry in items {
                if let Ok(entry) = entry {
                    let file_name = entry.file_name();
                    let file_name_str = file_name.to_string_lossy();

                    // Skip hidden files and common unneeded dirs
                    if file_name_str.starts_with('.')
                        || file_name_str == "target"
                        || file_name_str == "node_modules"
                        || file_name_str == ".git" {
                        continue;
                    }

                    if let Ok(metadata) = entry.metadata() {
                        if metadata.is_dir() {
                            output.push_str(&format!("{}📁 {}/\n", indent, file_name_str));
                            let _ = Self::scan_dir_recursive(&entry.path(), depth + 1, max_depth, output);
                        } else {
                            let size = metadata.len();
                            let size_str = if size < 1024 {
                                format!("{}B", size)
                            } else if size < 1024 * 1024 {
                                format!("{}KB", size / 1024)
                            } else {
                                format!("{}MB", size / (1024 * 1024))
                            };
                            output.push_str(&format!("{}📄 {} ({})\n", indent, file_name_str, size_str));
                        }
                    }
                }
            }
            }
            Err(_) => {
                // Directory not readable, skip
            }
        }

        Ok(())
    }

    fn build_tier2(session: &Session, max_tokens: u32) -> String {
        let mut text = String::new();
        let mut token_count = 0u32;

        for summary in &session.summary_segments {
            let summary_tokens = Self::estimate_tokens(&summary.content);
            if token_count + summary_tokens > max_tokens {
                break;
            }
            text.push_str(&summary.content);
            text.push_str("\n\n");
            token_count += summary_tokens;
        }

        if text.is_empty() {
            text.push_str("(No previous history yet)");
        }

        text
    }

    fn build_tier3(session: &Session, max_tokens: u32) -> String {
        let mut text = String::new();
        let mut token_count = 0u32;

        for turn in session.turns.iter().rev().take(20) {
            let turn_tokens = Self::estimate_tokens(&turn.content);
            if token_count + turn_tokens > max_tokens {
                continue;
            }

            let role_str = match turn.role {
                TurnRole::User => "User",
                TurnRole::Assistant => "Assistant",
                TurnRole::System => "System",
            };

            text = format!(
                "{}: {}\n\n{}",
                role_str, turn.content.lines().next().unwrap_or(""), text
            );
            token_count += turn_tokens;
        }

        if text.is_empty() {
            text.push_str("(No conversation history yet)");
        }

        text
    }

    fn build_tier4(session: &Session, _max_tokens: u32) -> String {
        let mut text = String::new();

        for file in session.active_files.iter().take(5) {
            text.push_str(&format!("--- {} ---\n", file));
            // In a real implementation, read the file from disk
            text.push_str("(file content would go here)\n\n");
        }

        text
    }

    fn estimate_tokens(text: &str) -> u32 {
        // Naive approximation: words * 1.3
        let word_count = text.split_whitespace().count();
        ((word_count as f64) * 1.3) as u32
    }
}

struct BudgetConfig {
    total: u32,
    tier1: u32,
    tier2: u32,
    tier3: u32,
    tier4: u32,
    tier5: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::models::Session;

    #[test]
    fn test_context_builder() {
        let session = Session::new("/test/project".to_string());
        let config = Config::default();
        let payload = ContextBuilder::build(&session, "test prompt", "claude", &config, false);

        assert!(!payload.tier1_system.is_empty());
        assert!(!payload.tier5_prompt.is_empty());
        assert!(payload.metadata.total_tokens > 0);
    }
}
