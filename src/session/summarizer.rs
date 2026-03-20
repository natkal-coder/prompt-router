use crate::session::models::{Turn, Summary, TurnRole};
use anyhow::Result;
use chrono::Utc;

/// Summarize a batch of turns into a compressed summary
/// In Phase 1, this is a template-based compressor.
/// Later phases will call the local model for better compression.
pub fn summarize_turns(turns: &[Turn], batch_size: u32) -> Result<Vec<Summary>> {
    if turns.is_empty() {
        return Ok(Vec::new());
    }

    let mut summaries = Vec::new();
    let batch_size = batch_size as usize;

    for chunk in turns.chunks(batch_size) {
        let start_id = chunk.first().map(|t| t.turn_id).unwrap_or(0);
        let end_id = chunk.last().map(|t| t.turn_id).unwrap_or(0);

        let key_decisions = extract_key_decisions(chunk);
        let files_involved: Vec<String> = chunk
            .iter()
            .flat_map(|t| t.files_referenced.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        let content = build_summary_text(chunk);
        let token_count = count_summary_tokens(&content);

        summaries.push(Summary {
            covers_turns: (start_id, end_id),
            content,
            token_count: token_count as u32,
            key_decisions,
            files_involved,
            created_at: Utc::now(),
        });
    }

    Ok(summaries)
}

fn extract_key_decisions(turns: &[Turn]) -> Vec<String> {
    let mut decisions = Vec::new();

    for turn in turns {
        let content_lower = turn.content.to_lowercase();

        let keywords = [
            "decision", "decided", "use", "chose", "chosen",
            "architecture", "design", "pattern", "constraint", "must", "requirement",
        ];

        for keyword in &keywords {
            if content_lower.contains(keyword) {
                // Naive extraction: if line contains keyword, extract it (simplified)
                for line in turn.content.lines() {
                    if line.to_lowercase().contains(keyword) && line.len() < 200 {
                        decisions.push(line.trim().to_string());
                    }
                }
            }
        }
    }

    decisions.into_iter().take(5).collect() // Keep top 5 decisions
}

fn build_summary_text(turns: &[Turn]) -> String {
    let mut text = String::new();

    let user_turns: Vec<_> = turns.iter().filter(|t| t.role == TurnRole::User).collect();
    let assistant_turns: Vec<_> = turns.iter().filter(|t| t.role == TurnRole::Assistant).collect();

    text.push_str(&format!(
        "Turns {}-{}: {} user messages, {} assistant responses\n\n",
        turns.first().map(|t| t.turn_id).unwrap_or(0),
        turns.last().map(|t| t.turn_id).unwrap_or(0),
        user_turns.len(),
        assistant_turns.len()
    ));

    if let Some(first_user) = user_turns.first() {
        text.push_str(&format!("Initial request: {}\n\n", truncate(&first_user.content, 150)));
    }

    let files_mentioned: std::collections::HashSet<_> =
        turns.iter().flat_map(|t| t.files_referenced.clone()).collect();
    if !files_mentioned.is_empty() {
        text.push_str("Files involved: ");
        text.push_str(&files_mentioned.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", "));
        text.push_str("\n\n");
    }

    if let Some(last_assistant) = assistant_turns.last() {
        text.push_str(&format!("Final state: {}\n", truncate(&last_assistant.content, 150)));
    }

    text
}

fn count_summary_tokens(text: &str) -> usize {
    // Naive token estimation: word count * 1.3
    let word_count = text.split_whitespace().count();
    ((word_count as f64) * 1.3) as usize
}

fn truncate(text: &str, max_len: usize) -> String {
    if text.len() > max_len {
        format!("{}...", &text[..max_len])
    } else {
        text.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_summarize_turns() {
        let turns = vec![
            Turn::new_user("implement feature X".to_string(), 50),
            Turn::new_assistant("Here's the code".to_string(), 100, crate::session::models::RouteTaken::Local, "test".to_string()),
        ];

        let summaries = summarize_turns(&turns, 1).expect("summarize failed");
        assert!(!summaries.is_empty());
        assert!(summaries[0].content.len() > 0);
    }
}
