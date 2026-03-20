use crate::session::models::{Turn, Summary};

/// Importance scoring for turns — decides which ones to evict when tier 3 overflows
pub fn score_turn_importance(turn: &Turn) -> f64 {
    let mut score: f64 = 0.0;

    // Has explicit constraint or architecture decision
    let high_value_keywords = ["must", "should", "constraint", "architecture", "design", "decision"];
    let has_constraint = high_value_keywords.iter().any(|kw| turn.content.to_lowercase().contains(kw));
    if has_constraint {
        score += 0.30;
    }

    // References currently active files
    if !turn.files_referenced.is_empty() {
        score += 0.25;
    }

    // Debugging/error breakthrough keywords
    let breakthrough_keywords = ["error", "root cause", "bug", "fix", "fixed", "works"];
    let is_breakthrough = breakthrough_keywords.iter().any(|kw| turn.content.to_lowercase().contains(kw));
    if is_breakthrough {
        score += 0.20;
    }

    // Files modified → ongoing work
    if !turn.files_modified.is_empty() {
        score += 0.15;
    }

    // Some weight for simple acknowledgments (low score)
    let simple_keywords = ["thanks", "looks good", "ok", "great"];
    let is_simple = simple_keywords.iter().any(|kw| turn.content.to_lowercase() == kw.to_lowercase());
    if is_simple {
        score -= 0.10;
    }

    score.max(0.0).min(1.0)
}

/// Evict turns from tier 3 (recent) when it exceeds token budget
pub fn evict_turns_for_summarization(
    recent_turns: &[Turn],
    max_tokens: u32,
) -> (Vec<Turn>, Vec<Turn>) {
    let mut token_sum = 0u32;
    let mut keep_idx = recent_turns.len();

    for (idx, turn) in recent_turns.iter().enumerate().rev() {
        if token_sum + turn.token_count > max_tokens {
            keep_idx = idx;
            break;
        }
        token_sum += turn.token_count;
    }

    let (to_evict, to_keep) = recent_turns.split_at(keep_idx);
    (to_evict.to_vec(), to_keep.to_vec())
}

/// Score-based eviction: evict lowest-importance turns first
pub fn evict_by_importance(
    recent_turns: &[Turn],
    max_tokens: u32,
    _evict_below_score: f64,
) -> (Vec<Turn>, Vec<Turn>) {
    let mut to_keep = recent_turns.to_vec();
    let mut token_sum: u32 = to_keep.iter().map(|t| t.token_count).sum();

    while token_sum > max_tokens && !to_keep.is_empty() {
        // Find the lowest-importance turn
        let min_idx = to_keep
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| {
                a.importance_score.partial_cmp(&b.importance_score).unwrap()
            })
            .map(|(idx, _)| idx)
            .unwrap_or(0);

        let evicted = to_keep.remove(min_idx);
        token_sum = token_sum.saturating_sub(evicted.token_count);
    }

    let evicted_count = recent_turns.len() - to_keep.len();
    let to_evict = recent_turns[..evicted_count].to_vec();

    (to_evict, to_keep)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_importance_scoring() {
        let turn = Turn::new_user("thanks".to_string(), 10);
        assert!(score_turn_importance(&turn) < 0.2);

        let turn = Turn::new_user("Architecture decision: use event-driven pattern".to_string(), 50);
        assert!(score_turn_importance(&turn) > 0.25);
    }

    #[test]
    fn test_evict_by_importance() {
        let turns = vec![
            Turn::new_user("constraint: must use postgres".to_string(), 100), // high importance
            Turn::new_user("thanks".to_string(), 50),                         // low importance
            Turn::new_user("bug fixed in utils.rs".to_string(), 150),          // medium-high
        ];

        let (evicted, kept) = evict_by_importance(&turns, 200, 0.3);
        assert!(evicted.len() > 0);
        assert!(kept.len() > 0);
    }
}
