# LOKAHI Quick Start Guide

## Install Dependencies

### 1. Rust
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
rustc --version  # Verify: 1.70+
```

### 2. Ollama
```bash
# Download from https://ollama.ai
# Or on macOS:
brew install ollama

# Start the service
ollama serve &

# Pull a model (one-time)
ollama pull qwen2.5-coder-7b-q4
```

### 3. Cloud CLIs (optional)
```bash
# Claude Code CLI
pip install anthropic-cli  # Or from https://claude.ai/download

# Gemini CLI
pip install google-generativeai  # Or from https://cloud.google.com
```

## Build LOKAHI

```bash
cd /path/to/lokahi

# Build in release mode
cargo build --release

# Binary is at: target/release/lokahi
```

## Run Tests

```bash
# All tests
cargo test

# Specific module tests
cargo test session_
cargo test latency::
cargo test intake::

# Show output
cargo test -- --nocapture
```

## First Run

```bash
# Start in current directory
./target/release/lokahi

# Or specify a project directory
./target/release/lokahi /path/to/my/project

# With debug logging
./target/release/lokahi --log-level debug

# Explain routing decisions without executing
./target/release/lokahi --explain
```

## Configuration

Edit `config/default.yaml` to customize:

```yaml
# Patience thresholds (how long to wait before giving up)
patience:
  instant_threshold_ms: 2000       # < 2s feels instant
  acceptable_threshold_ms: 5000    # < 5s is acceptable
  tolerable_threshold_ms: 15000    # < 15s is tolerable

# Routing preferences
routing:
  weights:
    latency: 0.40                  # Importance of latency
    quality: 0.30
    cost: 0.15
    reliability: 0.15
  force_local_intents:             # Always run locally
    - explain_code
    - format_code
  force_cloud_intents:             # Always run on cloud
    - security_audit
    - architecture_design

# Local model settings
local_model:
  primary: qwen2.5-coder-7b-q4     # Main model name
  gpu_layers: 35                   # For your GPU VRAM

# Cloud backend settings
cloud_backends:
  gemini_cli:
    enabled: true
    timeout_ms: 20000
  claude_code:
    enabled: true
    timeout_ms: 30000
```

## Understanding the Architecture

### Session Management
- Each conversation is a **Session** anchored to a project directory
- Sessions persist in `data/sessions.db` (SQLite)
- Resume a session with `--session session-name`

### 5-Tier Context Protocol
When sending a prompt to any backend (local or cloud), context is built in tiers:

1. **Tier 1** — System preamble (project context, active files)
2. **Tier 2** — Compressed history (summaries of old turns)
3. **Tier 3** — Recent turns (verbatim, high-importance turns kept longest)
4. **Tier 4** — Code context (files referenced in current prompt)
5. **Tier 5** — Current prompt

Each backend gets a budget (e.g., Claude: 24k tokens, Gemini: 32k tokens). Older turns are compressed into summaries to fit.

### Routing Decision
For each prompt, LOKAHI:
1. Classifies **intent** (write code, explain, debug, etc.)
2. Estimates **latency** for local vs cloud routes
3. Scores all routes by: latency (40%) + quality (30%) + cost (15%) + reliability (15%)
4. Picks the highest-scoring route

**Force Rules:**
- `explain_code`, `format_code` → always LOCAL (fast + low risk)
- `security_audit`, `architecture_design` → always CLOUD (need expertise)

### Latency Prediction
**Phase 1 (Bootstrap):**
```
T_local = (output_tokens / 30 tok/sec) + (context_tokens / 60 tok/sec)
T_cloud = 1000ms TTFT + (output_tokens / 50 tok/sec) + 100ms RTT
T_hybrid = blend of both (local draft shows first, cloud refines)
```

Adjusted by system load (CPU/memory pressure).

**Phase 2 (ML):**
- Once 200+ observations are logged, switch to XGBoost predictor
- Features: intent, tokens, system load, network RTT, time of day, etc.

### Feedback Loop
Every turn is logged to `data/feedback.db`:
- Predicted latency vs actual latency
- Tokens in/out
- Route taken (local/hybrid/cloud)
- Features for ML training

In Phase 2, this data retrains the latency predictor weekly.

## Common Commands (TUI — Coming Soon)

Once TUI is implemented, you'll have:

```
/session              # Show current session info
/session resume X     # Resume session X
/session export       # Export to markdown

/route                # Explain last routing decision
/route explain        # Show detailed scoring

/cost                 # Show daily cloud spend
/cost budget          # Check remaining budget

/file add path        # Add file to active context
/file remove path     # Remove from active context

/model list           # Show available local models
/model switch name    # Switch local model

/explain              # Show detailed prediction for last prompt
```

## Troubleshooting

### "Failed to connect to Ollama"
```bash
# Check if ollama is running
curl http://localhost:11434/api/version

# Start ollama if not running
ollama serve &
```

### "Model not found"
```bash
# Pull the model
ollama pull qwen2.5-coder-7b-q4

# List installed models
ollama list
```

### "Claude/Gemini command not found"
```bash
# Verify CLIs are installed
which claude
which gemini

# Disable in config/default.yaml if not using
cloud_backends:
  claude_code:
    enabled: false
  gemini_cli:
    enabled: false
```

### "Database locked"
Delete `data/sessions.db` and restart (will lose history):
```bash
rm data/sessions.db
./target/release/lokahi
```

## Development Tips

### Run with debug logging
```bash
RUST_LOG=debug ./target/release/lokahi --log-level debug
```

### Watch tests while developing
```bash
cargo watch -x test
```

### Check code without compiling
```bash
cargo check
```

### Format code
```bash
cargo fmt
```

### Lint
```bash
cargo clippy
```

## Next Steps

1. **Get familiar** with the session/routing architecture (see IMPLEMENTATION_SUMMARY.md)
2. **Run tests** to verify everything compiles: `cargo test`
3. **Read config** to understand latency thresholds and routing weights
4. **Implement TUI** — ratatui layout, input/output, status bar
5. **Test with real prompts** — verify local model works, cloud escalation works
6. **Collect feedback** — log observations for Phase 2 ML training
7. **Phase 2** — ML latency predictor, self-critique hybrid loop, race mode

## Architecture Files to Read

1. **src/config.rs** — All configuration structures
2. **src/session/models.rs** — Session/Turn/Summary data model
3. **src/session/context_builder.rs** — How 5-tier context is assembled
4. **src/intake/parser.rs** — Intent classification logic
5. **src/latency/heuristic.rs** — Latency prediction formulas
6. **src/balancer/router.rs** — Route decision logic
7. **tests/integration_test.rs** — Example usage patterns

## Performance Targets

| Metric | Target | How Tested |
|--------|--------|-----------|
| Startup time | <1s | Time to first prompt |
| Session resume | <500ms | Load from SQLite |
| Context assembly | <100ms | Build 5-tier payload |
| Latency prediction | <20ms | Estimate local/cloud |
| Local gen (simple task) | <2s | 100 tokens on 7B model |
| Cloud gen (complex task) | <8s | 500 tokens via CLI |
| Hybrid (draft + refine) | <6s | Progressive output |

## Support & Feedback

- **Issues**: github.com/anthropics/lokahi
- **Discussions**: github.com/anthropics/lokahi/discussions
- **Email**: rickesh@anthropic.com

Enjoy building with LOKAHI! 🚀
