/// Estimate token count for text (naive word-count based)
pub fn estimate_tokens(text: &str) -> u32 {
    let word_count = text.split_whitespace().count();
    ((word_count as f64) * 1.3) as u32
}

/// Estimate output tokens based on intent and input
pub fn estimate_output_tokens_for_intent(intent: &str, input_tokens: u32) -> u32 {
    match intent {
        "explain_code" => input_tokens + 200,
        "write_code" => input_tokens.max(100) * 2,
        "debug_code" => input_tokens + 400,
        "refactor_code" => input_tokens * 3,
        "architecture_design" => input_tokens * 2,
        "security_audit" => input_tokens + 500,
        "format_code" => input_tokens / 2,
        "write_docstring" => input_tokens / 3,
        "git_commit_message" => 50,
        _ => input_tokens,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_tokens() {
        let tokens = estimate_tokens("hello world this is a test");
        assert!(tokens > 0);
    }
}
