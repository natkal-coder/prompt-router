# 🚀 LOKAHI — Local LLM Router

**Intelligent prompt routing between local and cloud LLMs with real-time streaming, filesystem context injection, and command history.**

A developer tool that analyzes incoming prompts, reads your project structure, and routes requests optimally. Works entirely offline with Ollama, no cloud dependencies required. Built for developers who want fast, local-first AI assistance.

## Phase 1: Complete ✅

**Status**: Production Ready | **Binary Size**: 6.7 MB | **Test Coverage**: 16/16 tests (100%)

### Features Implemented
- ✅ Session Manager with SQLite persistence
- ✅ 5-tier sliding window context payload
- ✅ Heuristic latency predictor
- ✅ Intent classification (9 types)
- ✅ Route scoring (weighted: latency 40%, quality 30%, cost 15%, reliability 15%)
- ✅ Ollama HTTP streaming client with auto model pulling
- ✅ **Filesystem context injection** — reads your project structure, no hallucinations
- ✅ **Interactive terminal TUI** — command history with arrow keys
- ✅ **Full disk access** — transparent path translation (`~` → `/host_home`)
- ✅ **Docker containerization** — bundled Ollama with tinyllama model
- ✅ **Real-time streaming** — responses stream as they're generated

## 🚀 Getting Started (5 minutes)

### Prerequisites
- **Docker & Docker Compose** ([install](https://docs.docker.com/get-docker/))
- **4GB free disk** (for tinyllama model on first run)
- **2GB RAM** minimum

### Option 1: Docker (Recommended - No Local Setup)

```bash
# Clone and run
git clone https://github.com/rickeshtn/lokahi
cd lokahi
docker-compose run --rm lokahi
```

**First run**: ~2 minutes (Ollama downloads tinyllama model)
**Subsequent runs**: ~5 seconds startup

Then type your prompt:
```
❯ You: explain the session module

🔀 Route: LOCAL | Complexity: HIGH | Est. 10415ms

🤖 Assistant: The session module...
```

### Option 2: Build from Source (Advanced)

**Requirements**: Rust 1.70+, Ollama running locally

```bash
git clone https://github.com/rickeshtn/lokahi
cd lokahi
cargo build --release
./target/release/lokahi
```

### Commands
- **Arrow Keys ⬆️⬇️** — Navigate command history
- **Ctrl+C** — Quit
- `/quit` or `/exit` — Exit gracefully
- `/history` — Show conversation history
- `/clear` — Clear current session

## What Happens When You Start LOKAHI

```
🔍 DOCKER MOUNT DEBUG:
   /work → true
   /host_home → true
   /host_root → true
   /host_home contents: 136 items

Model pull initiated, waiting for download (this may take a minute)...
... Ready!

🚀 LOKAHI - Local LLM Router
═══════════════════════════════════════════
Session: e542d28f-ffc2-49e8-949e-b7a2476fb89f
Model: tinyllama

Type your prompt (arrow keys for history, Ctrl+C to quit)
```

**What's happening**:
1. ✅ **DOCKER MOUNT DEBUG** — Verifies your filesystem is accessible
2. ⏳ **Model pull** — First run downloads the model (~2 min), cached afterward
3. 📝 **Session created** — SQLite database initialized for conversation history
4. 🎯 **Ready for input** — Type your prompt

When you type a prompt:
```
❯ You: explain the router module

🔍 CONTEXT BUILDER DEBUG:
   Original project_root: /work
   Context scan path: /work
   Building context from: /work
   Tier4 scanned: 9 files, 8780 tokens

🔀 Route: CLOUD:gemini | Complexity: HIGH | Est. 10415ms

🤖 Assistant: [streaming response...]
```

**Understanding the output**:
- **CONTEXT BUILDER** — How many files analyzed for project context
- **Route** — Where this prompt is being processed (LOCAL/CLOUD)
- **Complexity** — How hard is this prompt (LOW/MEDIUM/HIGH)
- **Est. Xms** — Estimated response time

## Common Use Cases

### Explain existing code
```
❯ You: explain the session module
```
→ Local model scans your project, explains what it finds

### Get code suggestions
```
❯ You: write a function to parse JSON
```
→ Routes to cloud (Gemini/Claude) if available, fallback to local

### Ask about your project
```
❯ You: what's in ~/Projects/myapp/src?
```
→ Reads actual files, no hallucinations

### Continue conversation
Use **arrow keys** to navigate history, type `/history` to see full session

## How It Works (High-Level)

```
┌─────────────────────────────────────────────────────────────┐
│ 1. You type a prompt                                        │
│    "explain the router module"                              │
└────────────────────┬────────────────────────────────────────┘
                     │
┌────────────────────▼────────────────────────────────────────┐
│ 2. LOKAHI analyzes the prompt                               │
│    • Detects intent (explain_code)                          │
│    • Counts tokens (~50)                                    │
│    • Builds context from your project (~8000 tokens)        │
└────────────────────┬────────────────────────────────────────┘
                     │
┌────────────────────▼────────────────────────────────────────┐
│ 3. Route decision                                           │
│    • Complexity: HIGH (8000+ tokens)                        │
│    • Estimated latency: ~10s                                │
│    • Routes to: Local Ollama (fast) or Cloud (better)       │
└────────────────────┬────────────────────────────────────────┘
                     │
┌────────────────────▼────────────────────────────────────────┐
│ 4. Stream response back to you                              │
│    Real-time token-by-token output                          │
└─────────────────────────────────────────────────────────────┘
```

## Architecture

**Core modules** (for developers):

```
src/
├── tui/                 # Terminal UI (rustyline + streaming)
│   └── handles user input and displays responses
├── session/             # Session management & context
│   └── builds 5-tier context payload from project files
├── latency/             # Predicts response time
│   └── decides if response will be fast enough
├── balancer/            # Routing decisions
│   └── scores LOCAL vs CLOUD based on latency/quality/cost
├── local/               # Ollama integration
│   └── talks to local ollama service via HTTP
└── cloud/               # Cloud backends
    └── Claude/Gemini CLIs (fallback when local insufficient)
```

**Data**:
- `sessions.db` — SQLite with conversation history
- `config/default.yaml` — Routing settings, model choice

**Full codebase structure** — see [Architecture Deep Dive](#architecture-deep-dive) below

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
- **LOCAL**: Fast tasks, uses local model only (~1-3s)
- **HYBRID**: Draft locally, refine in cloud (Phase 2)
- **CLOUD**: Complex tasks needing Claude/Gemini (fallback when needed)

Heuristic scorer combines latency (40%) + quality (30%) + cost (15%) + reliability (15%).

## Architecture Deep Dive

For developers wanting to understand or modify LOKAHI:

```
src/
├── main.rs                      # CLI entrypoint
├── lib.rs                       # Module exports
├── config.rs                    # Load default.yaml settings
│
├── session/                     # Session & context management
│   ├── manager.rs              # Create/load/save sessions
│   ├── persistence.rs          # SQLite operations
│   ├── models.rs               # Session/Turn/Summary structs
│   ├── context_builder.rs      # Build 5-tier payload
│   │   └── Tier 1-5: system, history, recent, code, prompt
│   ├── sliding_window.rs       # Evict old turns, keep recent
│   └── summarizer.rs           # Compress turns with token limits
│
├── intake/                      # Parse user input
│   ├── parser.rs               # Intent classification (explain, write, etc)
│   └── tokenizer.rs            # Count tokens for budget tracking
│
├── latency/                     # Predict response time
│   ├── predictor.rs            # Main predictor interface
│   ├── heuristic.rs            # Formula-based (current)
│   ├── ml_model.rs             # XGBoost (Phase 2)
│   ├── system_monitor.rs       # CPU/RAM usage
│   └── calibrator.rs           # Startup benchmarks
│
├── balancer/                    # Route decisions
│   ├── router.rs               # Main decision engine
│   └── scorer.rs               # Score each route (LOCAL/CLOUD)
│
├── local/                       # Local model (Ollama)
│   ├── ollama.rs               # HTTP client for Ollama API
│   └── self_critique.rs        # Quality validation (Phase 2)
│
├── cloud/                       # Cloud backends
│   ├── adapter.rs              # CloudBackend trait
│   ├── claude.rs               # Claude CLI wrapper
│   ├── gemini.rs               # Gemini CLI wrapper
│   └── cursor.rs               # Cursor Agent (Phase 2)
│
├── tui/                         # Terminal UI
│   ├── mod.rs                  # Main event loop (rustyline)
│   ├── app.rs                  # Session state & input handling
│   ├── input.rs                # Key bindings & command parsing
│   └── ui.rs                   # Output formatting & streaming
│
├── assembler/                   # Response processing
│   └── merge.rs                # Combine local + cloud responses
│
└── feedback/                    # ML training data collection
    ├── collector.rs            # Log observations (timing, quality)
    └── trainer.rs              # Generate training examples
```

## Setup & Build (For Contributors)

### Prerequisites
- Rust 1.70+ ([install](https://rustup.rs/))
- Ollama for local testing
- Git for version control

### Build locally
```bash
cargo build --release    # Optimized binary in target/release/
cargo build              # Debug build (slower)
```

### Run locally (requires Ollama)
```bash
# Start Ollama in another terminal
ollama serve

# Then run LOKAHI
./target/release/lokahi
```

### Run in Docker (easiest)
```bash
docker-compose run --rm lokahi
```

### Verify setup
```bash
cargo test              # Run all tests (should pass 16/16)
cargo build --release  # Build release binary
```

## How to Change the Local Model

LOKAHI uses **tinyllama** by default (fast, ~1.1GB). You can switch to any model available on [Ollama](https://ollama.ai/library).

### Step 1: Find a Model
Popular models:
- **tinyllama** (default) — 1.1GB, fast, good for simple tasks
- **llama2:7b** — 3.8GB, better quality, 2-3s responses
- **mistral:7b** — 4.1GB, better coding, faster than llama2
- **neural-chat:7b** — 4.1GB, optimized for conversation
- **qwen2.5-coder:7b** — 4.7GB, best for coding, requires more VRAM

### Step 2: Update Configuration

Edit `config/default.yaml`:
```yaml
local_model:
  name: llama2:7b                     # Change this line
  # or use: mistral:7b, neural-chat:7b, qwen2.5-coder:7b
```

Or set via environment variable:
```bash
OLLAMA_MODEL=mistral:7b docker-compose run --rm lokahi
```

### Step 3: First Run
First run will download the model (~2-5 minutes depending on size). Subsequent runs use the cached model.

**Check available models**:
```bash
ollama list
```

**Download a model manually** (if needed):
```bash
ollama pull mistral:7b
docker-compose run --rm lokahi
```

### GPU Support
For better performance with larger models, LOKAHI automatically uses GPU if available:
- **NVIDIA**: Works automatically with nvidia-docker
- **Apple Silicon**: Use `OLLAMA_NUM_THREAD=4` environment variable
- **CPU Only**: Works fine, just slower

## Configuration

Edit `config/default.yaml` for advanced settings:

```yaml
# Model settings
local_model:
  name: tinyllama                     # Model to use
  # Other options: llama2:7b, mistral:7b, neural-chat:7b

# Routing thresholds
patience:
  instant_threshold_ms: 2000          # Response feels instant
  acceptable_threshold_ms: 5000       # Acceptable wait
  tolerable_threshold_ms: 15000       # Still acceptable

# Route scoring weights
routing:
  weights:
    latency: 0.40                     # Speed (40%)
    quality: 0.30                     # Quality (30%)
    cost: 0.15                        # Cost (15%)
    reliability: 0.15                 # Reliability (15%)

# Cloud backends (only active when available)
cloud_backends:
  claude_cli:
    enabled: true
    timeout_ms: 30000
  gemini_cli:
    enabled: true
    timeout_ms: 20000
```

Most users don't need to change these — defaults are optimized for local-first development.

## Troubleshooting

### "Model not found" error
**Problem**: First run shows `{"error":"model 'tinyllama' not found"}`

**Solution**: Wait 2-5 minutes for model to download. The download happens in the background.
```bash
# Check download progress
docker logs lokahi_ollama

# Or download manually first
docker-compose run --rm lokahi bash -c "ollama pull tinyllama"
```

### Empty responses or hanging
**Problem**: LOKAHI starts but responses are empty or it hangs waiting for input

**Possible causes**:
1. **Model still downloading** — First run takes 1-2 minutes
2. **Ollama not responding** — Check docker logs:
   ```bash
   docker logs lokahi_ollama
   ```
3. **Out of memory** — Reduce model size or increase Docker memory:
   ```bash
   # In docker-compose.yml, add to lokahi service:
   mem_limit: 4g
   ```

### Docker permission error
**Problem**: `permission denied while trying to connect to Docker daemon`

**Solution**:
```bash
# Add your user to docker group
sudo usermod -aG docker $USER
newgrp docker

# Or use sudo
sudo docker-compose run --rm lokahi
```

### "Port 11434 already in use"
**Problem**: `Port 11434 is already allocated`

**Solution**: Another Ollama instance is running
```bash
# Stop any running Ollama
docker-compose down

# Or use a different port
OLLAMA_PORT=11435 docker-compose up
```

### Model performance is slow
**Problem**: Responses take 10+ seconds

**Suggestions**:
1. **Use a faster model**:
   ```bash
   OLLAMA_MODEL=tinyllama docker-compose run --rm lokahi
   ```
2. **Enable GPU** (if you have NVIDIA):
   ```bash
   # Use nvidia-docker
   docker run --gpus all -v lokahi_ollama_data:/root/.ollama ollama/ollama
   ```
3. **Increase memory allocation** in docker-compose.yml

## Testing

### Run all tests
```bash
cargo test
```

### Unit tests only
```bash
cargo test --lib
```

### Specific test suite
```bash
cargo test session_tests
cargo test context_builder_tests
cargo test routing_tests
```

### With logging
```bash
RUST_LOG=debug cargo test
```

## Next Steps for New Users

### First: Get comfortable with the basics
1. Run LOKAHI a few times, notice how it routes different prompts
2. Try asking about your own project structure
3. Check `/history` to see stored sessions

### Second: Customize the model
1. Try a different model (mistral:7b, llama2:7b)
2. See how speed and quality differ
3. Pick the one you like best

### Third: Integrate into your workflow
- Open LOKAHI in a second terminal while coding
- Use it for code explanations, suggestions, debugging
- Use arrow keys to reuse previous prompts

### Developers: Run the tests
```bash
cargo test              # All tests
cargo test session      # Just session tests
```

## Development Roadmap

### Phase 1 (Current) — ✅ Complete
- ✅ Session Manager with SQLite
- ✅ Sliding window context with importance scoring
- ✅ Heuristic latency predictor
- ✅ Intelligent routing (LOCAL/CLOUD)
- ✅ Full-disk access with path translation
- ✅ Real-time streaming responses
- ✅ Ollama auto-management (pull models, lifecycle)
- ✅ 16/16 integration tests

### Phase 2 (Planned)
- ML-based latency predictor (XGBoost for better accuracy)
- Code-aware context extraction (tree-sitter for AST parsing)
- Hybrid route self-critique (draft locally → refine in cloud)
- Multi-backend race mode (query multiple backends in parallel)
- Automatic quality scoring

### Phase 3 (Future)
- Automatic model fine-tuning (LoRA)
- VS Code integration
- Neovim plugin
- Web-based dashboard for session management

### Phase 4 (Later)
- Multi-model ensemble (combine 2B + 7B locally)
- Speculative decoding
- Cross-session memory with RAG

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
