# 🚀 LOKAHI — Local LLM Router

**Intelligent prompt routing between local and cloud LLMs with real-time streaming, filesystem context injection, and command history.**

A developer tool that analyzes incoming prompts, predicts latency, reads your project structure, and routes requests optimally. Works entirely offline with Ollama, no cloud dependencies required.

## Phase 1: Complete ✅

**Status**: Production Ready
**Binary Size**: 6.7 MB (optimized release)
**Test Coverage**: 16/16 tests passing (100%)

### Features Implemented
- ✅ Session Manager with SQLite persistence
- ✅ 5-tier sliding window context payload
- ✅ Heuristic latency predictor
- ✅ Intent classification (9 types)
- ✅ Route scoring (weighted: latency 40%, quality 30%, cost 15%, reliability 15%)
- ✅ Ollama HTTP streaming client
- ✅ Ollama lifecycle management (auto-start/stop)
- ✅ **Filesystem context injection** — reads actual project structure, no hallucinations
- ✅ **Interactive terminal TUI** — stdin/stdout with rustyline for history
- ✅ **Command history** — arrow keys for previous prompts
- ✅ **Complexity analysis** — LOW/MEDIUM/HIGH routing display
- ✅ **Docker containerization** — bundled Ollama with tinyllama model
- ✅ **Path translation** — seamless Docker file access (`~/Projects` → `/host/Projects`)

## Quick Start

```bash
cd ~/Projects/lokahi
docker-compose run --rm lokahi
```

Type your prompt:
```
❯ You: what's in ~/Projects/lokahi?

🔀 Route: LOCAL | Complexity: LOW | Est. 2332ms

🤖 Assistant: [real project structure with actual files]
```

**Commands**:
- `/quit` — Exit
- `/history` — Show conversation history
- `/clear` — Clear session
- Arrow Keys ⬆️⬇️ — Navigate command history

## Installation

Choose your preferred method:

1. **Quick Install** (Recommended)
   ```bash
   bash <(curl -fsSL https://raw.githubusercontent.com/rickeshtn/lokahi/main/install.sh)
   ```

2. **NPM** (for Node.js users)
   ```bash
   npm install -g lokahi-cli
   ```

3. **Cargo** (for Rust users)
   ```bash
   cargo install lokahi
   ```

4. **From Source**
   ```bash
   git clone https://github.com/rickeshtn/lokahi
   cd lokahi && cargo build --release
   ```

See [INSTALL.md](./INSTALL.md) for detailed setup, troubleshooting, and verification.

## Architecture

```
lokahi/
├── src/
│   ├── main.rs                      # CLI entrypoint
│   ├── config.rs                    # Config loading & structs
│   ├── session/
│   │   ├── models.rs               # Session/Turn/Summary/BackendThread structs
│   │   ├── persistence.rs          # SQLite CRUD
│   │   ├── manager.rs              # Session lifecycle
│   │   ├── sliding_window.rs       # Tier eviction & importance scoring
│   │   ├── summarizer.rs           # Turn compression
│   │   └── context_builder.rs      # 5-tier payload assembly
│   ├── intake/
│   │   ├── parser.rs               # Intent classification (regex)
│   │   └── tokenizer.rs            # Token counting
│   ├── latency/
│   │   ├── predictor.rs            # Heuristic → ML switcher
│   │   ├── heuristic.rs            # Bootstrap formulas
│   │   ├── ml_model.rs             # Placeholder for XGBoost (Phase 2)
│   │   ├── system_monitor.rs       # CPU/RAM sampling
│   │   └── calibrator.rs           # Startup benchmarks
│   ├── balancer/
│   │   ├── router.rs               # Route decision logic
│   │   └── scorer.rs               # Weighted route scoring
│   ├── local/
│   │   ├── ollama.rs               # Ollama HTTP client
│   │   └── self_critique.rs        # Self-critique for hybrid route
│   ├── cloud/
│   │   ├── adapter.rs              # CloudBackend trait
│   │   ├── claude.rs               # Claude CLI backend
│   │   ├── gemini.rs               # Gemini CLI backend
│   │   └── cursor.rs               # Cursor Agent stub
│   ├── tui/                         # Terminal UI (stdin/stdout + rustyline)
│   ├── assembler/
│   │   └── merge.rs                # Response merging & validation
│   └── feedback/
│       ├── collector.rs            # Observation logging
│       └── trainer.rs              # ML model training (Phase 2)
├── config/
│   ├── default.yaml                # Default routing/patience/thresholds
│   ├── backends.yaml               # Cloud backend profiles
│   └── session.yaml                # Sliding window tier budgets
├── data/
│   ├── sessions.db                 # SQLite: sessions, turns, summaries
│   ├── feedback.db                 # SQLite: observations for ML training
│   └── latency_model.bin          # Trained XGBoost model (Phase 2+)
└── tests/
    ├── session_tests.rs
    ├── context_builder_tests.rs
    └── routing_tests.rs
```

## Key Design Decisions

### 1. **Rust + Tokio + ratatui**
- High performance, safety, and concurrency
- ratatui for TUI, tokio for async I/O
- Minimal binary size for distribution

### 2. **Ollama for Local Model**
- User runs `ollama serve` separately; LOKAHI manages model lifecycle
- Easy integration, supports any GGUF quantization
- HTTP API for streaming

### 3. **SQLite for Session Persistence**
- Embedded database, no external setup
- Full session history in local SQLite
- Feedback observations for ML training

### 4. **Heuristic-first Latency Prediction**
- Bootstrap with formula-based prediction (Phase 1)
- Auto-switch to XGBoost once 200+ observations (Phase 2)
- Features: intent, token counts, system load, network RTT

### 5. **5-Tier Sliding Window Context**
- Tier 1: System preamble (project context)
- Tier 2: Compressed history (older turns → summaries)
- Tier 3: Recent turns (verbatim, highest importance)
- Tier 4: Active code context (file contents)
- Tier 5: Current prompt

Each backend gets budget-aware payloads. Older turns evicted by importance, not just age.

### 6. **Three Routing Paths**
- **LOCAL**: Fast tasks, uses local model only
- **HYBRID**: Draft locally, refine in cloud (progressive disclosure)
- **CLOUD**: Complex tasks needing high quality

Heuristic scorer combines latency (40%) + quality (30%) + cost (15%) + reliability (15%).

## Setup & Build

### Prerequisites
- Rust 1.70+ ([install](https://rustup.rs/))
- Ollama installed and working (`ollama serve`)
- Claude CLI configured (`claude` command available)
- Gemini CLI configured (`gemini` command available) or disabled in config

### Build
```bash
cd /path/to/lokahi
cargo build --release
```

### Run Tests
```bash
cargo test
```

### First-time Calibration
When LOKAHI starts, it will:
1. Check if Ollama is running on localhost:11434
2. Benchmark local model throughput (tokens/sec)
3. Ping cloud endpoints to measure RTT
4. Initialize database schema

## Usage

### Start TUI in current directory
```bash
./target/release/lokahi
```

### Resume a specific session
```bash
./target/release/lokahi --session my-session-name
```

### Explain routing decision
```bash
./target/release/lokahi --explain
```

### Set log level
```bash
./target/release/lokahi --log-level debug
```

## Configuration

Edit `config/default.yaml`:

```yaml
patience:
  instant_threshold_ms: 2000          # < 2s feels instant
  acceptable_threshold_ms: 5000       # < 5s is acceptable
  tolerable_threshold_ms: 15000       # < 15s is tolerable

routing:
  weights:
    latency: 0.40                     # Latency importance
    quality: 0.30
    cost: 0.15
    reliability: 0.15
  force_local_intents:                # Always route locally
    - explain_code
    - format_code
    - write_docstring
  force_cloud_intents:                # Always route to cloud
    - security_audit
    - architecture_design

local_model:
  primary: qwen2.5-coder-7b-q4        # Main model
  fast: codegemma-2b-q4               # Used for sub-1s tasks
  gpu_layers: 35                      # Adjust for your VRAM

cloud_backends:
  gemini_cli:
    enabled: true
    timeout_ms: 20000
  claude_code:
    enabled: true
    timeout_ms: 30000
  cursor_agent:
    enabled: false                    # Disabled in Phase 1
```

## Testing

### Unit Tests
```bash
cargo test --lib
```

### Session Persistence
```bash
cargo test session_tests
```

### Context Builder
```bash
cargo test context_builder_tests
```

### Routing Decisions
```bash
cargo test routing_tests
```

## Development Roadmap

### Phase 1 (Current) — Complete
- ✅ Session Manager + SQLite
- ✅ Sliding window context + importance scoring
- ✅ Heuristic latency predictor
- ✅ Prompt balancer/router
- ✅ TUI shell (stdin/stdout with rustyline history)
- ✅ Ollama lifecycle management (auto-start/stop, model pulling)
- ✅ Integration tests (16/16 passing)

### Phase 2
- ML-based latency predictor (XGBoost)
- AST-aware code context extraction (tree-sitter)
- Intelligent prompt rewriting
- ML-based turn importance scoring
- Hybrid route self-critique loop (full impl)
- Race mode (low-confidence simultaneous routing)

### Phase 3
- Full feedback collection pipeline
- Automatic model retraining
- Patience profile learning
- VS Code extension
- Neovim plugin

### Phase 4
- Multi-model local ensemble (2B + 7B)
- Speculative decoding
- Persistent KV cache
- Codebase-aware fine-tuning (LoRA)
- Cross-session memory

## Key Metrics (Phase 1 Target)

- **Latency prediction MAE**: < 25%
- **Local resolution rate**: > 65%
- **Cloud token savings**: > 50% (vs raw)
- **Context coherence rate**: > 95% (cloud gives relevant responses)
- **P50 user-perceived latency**: < 2s
- **User acceptance (local)**: > 80%
- **User acceptance (hybrid)**: > 90%

## License

MIT
