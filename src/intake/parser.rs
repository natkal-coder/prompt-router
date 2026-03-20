use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Intent {
    ExplainCode,
    WriteCode,
    DebugCode,
    RefactorCode,
    ArchitectureDesign,
    SecurityAudit,
    FormatCode,
    WriteDocstring,
    GitCommitMessage,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct PromptAnalysis {
    pub intent: Intent,
    pub content: String,
    pub context_tokens_estimate: u32,
    pub output_tokens_estimate: u32,
    pub tree_depth: u32,
    pub file_count: u32,
    pub has_errors: bool,
}

/// Classify user intent from prompt text (regex-based heuristic)
pub fn classify_intent(text: &str) -> Intent {
    let lower = text.to_lowercase();

    if matches_pattern(&lower, &[
        "explain", "what does", "how does", "describe", "tell me about",
        "understand", "clarify"
    ]) {
        return Intent::ExplainCode;
    }

    if matches_pattern(&lower, &[
        "write", "generate", "implement", "create", "add", "make",
        "build", "function", "class", "method"
    ]) {
        return Intent::WriteCode;
    }

    if matches_pattern(&lower, &[
        "debug", "fix", "error", "bug", "broken", "crash", "issue",
        "wrong", "fail", "not working"
    ]) {
        return Intent::DebugCode;
    }

    if matches_pattern(&lower, &[
        "refactor", "simplify", "improve", "optimize", "clean up", "restructure",
        "reorganize", "rewrite"
    ]) {
        return Intent::RefactorCode;
    }

    if matches_pattern(&lower, &[
        "architecture", "design", "pattern", "structure", "organize",
        "module", "layer", "system design"
    ]) {
        return Intent::ArchitectureDesign;
    }

    if matches_pattern(&lower, &[
        "security", "audit", "vulnerability", "safe", "secure",
        "authentication", "authorization", "encrypt"
    ]) {
        return Intent::SecurityAudit;
    }

    if matches_pattern(&lower, &[
        "format", "style", "indent", "lint", "prettier", "black",
        "spacing", "whitespace", "alignment"
    ]) {
        return Intent::FormatCode;
    }

    if matches_pattern(&lower, &[
        "docstring", "comment", "documentation", "doc", "readme",
        "javadoc", "docblock"
    ]) {
        return Intent::WriteDocstring;
    }

    if matches_pattern(&lower, &[
        "commit", "git", "message", "changelog", "release notes"
    ]) {
        return Intent::GitCommitMessage;
    }

    Intent::Unknown
}

/// Analyze a prompt for routing decision
pub fn analyze_prompt(text: &str) -> PromptAnalysis {
    let intent = classify_intent(text);
    let context_tokens = estimate_context_tokens(text);
    let output_tokens = estimate_output_tokens(&intent, context_tokens);
    let has_errors = text.to_lowercase().contains("error")
        || text.to_lowercase().contains("bug")
        || text.to_lowercase().contains("fail");

    PromptAnalysis {
        intent,
        content: text.to_string(),
        context_tokens_estimate: context_tokens,
        output_tokens_estimate: output_tokens,
        tree_depth: 0, // Would be filled by AST analysis later
        file_count: count_file_mentions(text),
        has_errors,
    }
}

fn matches_pattern(text: &str, patterns: &[&str]) -> bool {
    patterns.iter().any(|p| text.contains(p))
}

fn estimate_context_tokens(text: &str) -> u32 {
    // Naive: word count * 1.3
    let word_count = text.split_whitespace().count();
    ((word_count as f64) * 1.3) as u32
}

fn estimate_output_tokens(intent: &Intent, input_tokens: u32) -> u32 {
    match intent {
        Intent::ExplainCode => input_tokens + 200,
        Intent::WriteCode => input_tokens.max(100) * 2,
        Intent::DebugCode => input_tokens + 400,
        Intent::RefactorCode => input_tokens * 3,
        Intent::ArchitectureDesign => input_tokens * 2,
        Intent::SecurityAudit => input_tokens + 500,
        Intent::FormatCode => input_tokens / 2,
        Intent::WriteDocstring => input_tokens / 3,
        Intent::GitCommitMessage => 50,
        Intent::Unknown => input_tokens,
    }
}

fn count_file_mentions(text: &str) -> u32 {
    let file_extensions = ["rs", "py", "js", "ts", "go", "java", "cpp", "c", "h", "hpp"];
    let count = file_extensions
        .iter()
        .filter(|ext| text.contains(&format!(".{}", ext)))
        .count();
    count as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intent_classification() {
        assert_eq!(
            classify_intent("explain how this function works"),
            Intent::ExplainCode
        );

        assert_eq!(
            classify_intent("write a function that does X"),
            Intent::WriteCode
        );

        assert_eq!(
            classify_intent("do a security audit of src/"),
            Intent::SecurityAudit
        );

        assert_eq!(
            classify_intent("fix this bug"),
            Intent::DebugCode
        );
    }

    #[test]
    fn test_analyze_prompt() {
        let analysis = analyze_prompt("write a function that returns the sum");
        assert_eq!(analysis.intent, Intent::WriteCode);
        assert!(analysis.output_tokens_estimate > analysis.context_tokens_estimate);
    }
}
