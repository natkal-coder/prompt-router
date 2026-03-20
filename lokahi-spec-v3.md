# Project: LOKAHI — Local Orchestration Kernel for Adaptive Hierarchical Intelligence

## Vision

Build a locally-running developer tool — with a **Claude Code-style interactive terminal UI** — that acts as an intelligent middleware layer between the developer and expensive cloud LLM APIs. A small local model (or ensemble of tiny recursive models) intercepts all developer prompts, **predicts the end-to-end latency each execution path would incur**, and routes accordingly — handling fast tasks locally, escalating slow/hard tasks to cloud, and using hybrid draft-then-refine for everything in between. Critically, **LOKAHI maintains a persistent conversational session with a sliding-window context protocol**, ensuring that when prompts are escalated to cloud backends (Gemini CLI, Claude Code, Cursor Agent), the cloud model receives enough conversational history to produce coherent, context-aware responses — not gibberish from a cold start. The goal: **reduce cloud API invocations by 60-80%** while maintaining output quality parity, **optimize the tokens sent** when cloud calls are necessary, **preserve session continuity across routing boundaries**, and **keep perceived latency under the user's patience threshold at all times**.

---

## Core Insight: Latency as the Primary Routing Signal

Previous iteration of this spec used abstract "complexity scores" as the routing signal. That was wrong. **Predicted latency is the routing signal**, because:

1. **Latency is what the user actually feels.** A complexity score of 6.2 means nothing to the developer. "This will take ~1.5s locally vs ~8s via cloud" is a real tradeoff the system can optimize.

2. **Latency subsumes complexity.** A high-complexity task takes longer to generate tokens for. A large context window takes longer to process. A multi-step reasoning chain takes longer to complete. Latency is the observable consequence of all these factors combined.

3. **Latency is measurable and trainable.** Unlike "complexity" which is subjective, latency is a number you can measure after every call, compare to your prediction, and use to retrain the predictor. This creates a tight feedback loop.

4. **Latency determines UX strategy.** If predicted latency is <2s, stream locally and the user barely notices. If 2-5s, show a progress indicator. If 5-15s, start a local draft immediately while cloud works in parallel. If >15s, decompose the task. The UX adapts to the prediction.

---

## System Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│              LOKAHI TUI (Claude Code-style Frontend)             │
│                                                                 │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │  Interactive REPL with persistent session                 │  │
│  │  > user prompt input                                      │  │
│  │  > streaming output (local/cloud, inline diffs)           │  │
│  │  > status bar: [route] [latency] [cost] [session depth]   │  │
│  │  > file tree, git status, project context sidebar         │  │
│  └──────────────────────┬────────────────────────────────────┘  │
└─────────────────────────┼───────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────────┐
│                LOKAHI — Local Orchestration Layer                │
│                                                                 │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │  0. SESSION MANAGER (new — owns all conversational state) │  │
│  │     - Maintains sliding window of conversation turns      │  │
│  │     - Builds context payload for EVERY route (local/cloud)│  │
│  │     - Compresses older turns into summaries               │  │
│  │     - Tracks which turns each backend has already seen    │  │
│  │     - Manages per-backend conversation threads            │  │
│  └──────────────────────┬────────────────────────────────────┘  │
│                         │                                       │
│  ┌──────────────────────▼────────────────────────────────────┐  │
│  │  1. INTAKE PARSER                                        │  │
│  │     - Tokenize + classify intent                         │  │
│  │     - Extract code context window                        │  │
│  │     - Emit feature vector for latency prediction         │  │
│  │     - Tag: {intent, lang, context_tokens, output_class,  │  │
│  │             tree_depth, file_count, has_errors}           │  │
│  └──────────────────────┬────────────────────────────────────┘  │
│                         │                                       │
│  ┌──────────────────────▼────────────────────────────────────┐  │
│  │  2. LATENCY PREDICTOR                                    │  │
│  │     - Predict: T_local, T_hybrid, T_cloud per backend    │  │
│  │     - Factor in: session context size (from Session Mgr), │  │
│  │       system load, model warmth, queue depth, token       │  │
│  │       estimates, network RTT, time-of-day                 │  │
│  │     - Output: latency envelope per route                  │  │
│  └──────────────────────┬────────────────────────────────────┘  │
│                         │                                       │
│  ┌──────────────────────▼────────────────────────────────────┐  │
│  │  3. PROMPT BALANCER (route decision engine)               │  │
│  │     - Combine: predicted latency + quality risk + cost    │  │
│  │     - Apply user's patience profile                       │  │
│  │     - Route: LOCAL | HYBRID | CLOUD                       │  │
│  │     - CRITICAL: Request context payload from Session Mgr  │  │
│  │       before dispatching to any backend                   │  │
│  └──────┬──────────────┬──────────────────┬──────────────────┘  │
│         │              │                  │                      │
│    ┌────▼───┐    ┌─────▼──────┐    ┌──────▼───────┐            │
│    │ LOCAL  │    │  HYBRID    │    │    CLOUD     │            │
│    │ + ctx  │    │ + ctx      │    │ + ctx        │            │
│    │ window │    │ Local draft│    │ Session ctx  │            │
│    │        │    │ streams    │    │ + compressed │            │
│    │ Full   │    │ immediately│    │ history sent │            │
│    │ session│    │ while cloud│    │ to backend   │            │
│    │ in KV  │    │ refines    │    │              │            │
│    └────┬───┘    └─────┬──────┘    └──────┬───────┘            │
│         │              │                  │                      │
│  ┌──────▼──────────────▼──────────────────▼──────────────────┐  │
│  │  4. RESPONSE ASSEMBLER                                    │  │
│  │     - Merge/validate outputs                              │  │
│  │     - **Commit turn to Session Manager (prompt+response)**│  │
│  │     - Record ACTUAL latency vs predicted                  │  │
│  │     - Cache result + metadata                             │  │
│  └──────────────────────┬────────────────────────────────────┘  │
│                         │                                       │
│  ┌──────────────────────▼────────────────────────────────────┐  │
│  │  5. FEEDBACK & LEARNING LOOP                              │  │
│  │     - Prediction error: |T_predicted - T_actual|          │  │
│  │     - Retrain latency model on accumulated observations   │  │
│  │     - Calibrate per-backend latency profiles              │  │
│  │     - Adjust patience thresholds from user behavior       │  │
│  └───────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

---

## Core Insight: Session Continuity is Non-Negotiable

The v2 spec focused on latency prediction but treated each prompt as independent. That's a fatal flaw. Real developer workflows are **conversational**: "refactor this function" → "now add error handling" → "actually, make it async too" → "write tests for what we just built." If LOKAHI routes turn 3 to cloud but cloud has no memory of turns 1-2, the response will be gibberish.

**The Session Manager solves this by maintaining a single source of truth for conversational state**, and building an appropriate context payload for whichever backend handles each turn. Cloud backends are stateless — they don't remember previous API calls. Every escalation must carry enough history for the cloud model to understand what's going on, without sending the entire conversation (which would be expensive and slow).

The sliding window approach is inspired by how Claude Code and Cursor work internally: keep recent turns verbatim, compress older turns into summaries, and always include the active file context.

---

## Component 0: Session Manager (Owns All Conversational State)

**Purpose:** Maintain a persistent, structured conversational session. Build context payloads for every backend (local and cloud) that are coherent, token-efficient, and preserve the information the model needs to give a good answer.

### 0A: Session Data Model

Every interaction in LOKAHI lives inside a **Session**. A session maps roughly to a working context — a project directory, a debugging task, or a feature implementation. Sessions persist across LOKAHI restarts (stored in SQLite).

```
SESSION
├── session_id: uuid
├── created_at: timestamp
├── project_root: /path/to/project        # Anchored to a codebase
├── active_files: [file_path, ...]         # Files currently "open" / referenced
├── turns: [Turn, ...]                     # Ordered conversation history
├── summary_segments: [Summary, ...]       # Compressed older history
├── file_snapshots: {path → content_hash}  # Track file state at each turn
└── metadata:
    ├── total_turns: int
    ├── total_tokens_used: int
    ├── cloud_backends_used: [backend_name, ...]
    └── last_active: timestamp

TURN
├── turn_id: int (monotonic within session)
├── timestamp: timestamp
├── role: "user" | "assistant" | "system"
├── content: string                        # The actual prompt or response text
├── route_taken: "local" | "hybrid" | "cloud_gemini" | "cloud_claude" | "cloud_cursor"
├── backend_model: string                  # e.g. "qwen2.5-coder-7b", "claude-sonnet-4"
├── token_count: int                       # Token count of this turn's content
├── files_referenced: [file_path, ...]     # Files mentioned or modified in this turn
├── files_modified: [file_path, ...]       # Files actually changed as a result
├── code_blocks: [{language, content, file_path, line_range}, ...]
├── is_summarized: bool                    # Has this turn been compressed into a summary?
└── importance_score: float                # How critical is this turn for future context?
    # High importance: architecture decisions, error root causes, user constraints
    # Low importance: formatting requests, typo fixes, acknowledgments

SUMMARY
├── covers_turns: [start_turn_id, end_turn_id]
├── content: string                        # Compressed natural language summary
├── token_count: int
├── key_decisions: [string, ...]           # Extracted decisions/facts (structured)
├── files_involved: [file_path, ...]
└── created_at: timestamp
```

### 0B: The Sliding Window Context Protocol

The core challenge: cloud backends are stateless. Every API call must contain all the context the model needs. But sending the full session history every time would be:
- **Expensive** (thousands of tokens of history × cloud pricing)
- **Slow** (more input tokens = longer prefill = higher latency)
- **Wasteful** (most of the history isn't relevant to the current turn)

The sliding window solves this with a **three-tier context structure**:

```
CONTEXT PAYLOAD (sent to any backend for each turn)
┌─────────────────────────────────────────────────────────┐
│  TIER 1: SYSTEM PREAMBLE (always present)               │
│  - Project description (auto-extracted or user-defined)  │
│  - Active file list + tree structure                     │
│  - Key constraints/conventions from earlier in session    │
│  - ~200-500 tokens, refreshed per session                │
├─────────────────────────────────────────────────────────┤
│  TIER 2: COMPRESSED HISTORY (sliding summaries)          │
│  - Summaries of older conversation segments              │
│  - Extracted key decisions, variable names, architecture │
│  - Oldest material, most aggressively compressed         │
│  - ~300-1500 tokens depending on session length          │
├─────────────────────────────────────────────────────────┤
│  TIER 3: RECENT TURNS (verbatim sliding window)          │
│  - Last N turns in full (user prompt + assistant response)│
│  - N is dynamic: sized to fit within token budget        │
│  - Most recent turn is ALWAYS included verbatim          │
│  - ~1000-6000 tokens depending on turn sizes             │
├─────────────────────────────────────────────────────────┤
│  TIER 4: ACTIVE CODE CONTEXT (current file state)        │
│  - Full content of files referenced in current prompt    │
│  - Diffs since last turn for modified files              │
│  - Relevant function signatures from imported modules    │
│  - ~500-4000 tokens depending on code size               │
├─────────────────────────────────────────────────────────┤
│  TIER 5: CURRENT PROMPT (the new user message)           │
│  - The actual prompt for this turn                       │
│  - ~50-2000 tokens                                       │
└─────────────────────────────────────────────────────────┘

TOTAL BUDGET: configurable per backend (see below)
```

**Token budget allocation per backend:**

```yaml
context_budgets:
  local:
    total_budget_tokens: 16000       # Local model's effective context window
    tier1_system: 400                # Fixed
    tier2_summaries: 800             # Compressed history
    tier3_recent_turns: 6000         # Verbatim recent turns
    tier4_code_context: 6000         # Active files
    tier5_prompt: 2800               # Current prompt
    # Note: local model sees ALL turns in KV cache, so tier2/3 are
    # only needed when KV cache is cold or session was long ago

  cloud_gemini:
    total_budget_tokens: 32000       # We CAN send more, but latency scales with input
    tier1_system: 500
    tier2_summaries: 2000
    tier3_recent_turns: 12000
    tier4_code_context: 12000
    tier5_prompt: 5500

  cloud_claude:
    total_budget_tokens: 24000       # Balance context richness vs latency
    tier1_system: 500
    tier2_summaries: 1500
    tier3_recent_turns: 8000
    tier4_code_context: 10000
    tier5_prompt: 4000

  cloud_cursor:
    total_budget_tokens: 20000
    tier1_system: 400
    tier2_summaries: 1200
    tier3_recent_turns: 6000
    tier4_code_context: 8000
    tier5_prompt: 4400
    
  # When latency prediction is near patience threshold,
  # reduce budgets by this factor to compress more aggressively
  latency_pressure_reduction: 0.6   # Cut to 60% of budget when under pressure
```

### 0C: Sliding Window Mechanics — How Turns Flow Through the Tiers

```
SESSION START (Turn 0):
  Tier 2 (summaries): empty
  Tier 3 (recent):    empty
  Tier 4 (code):      initial file context
  → Payload is small and fast

TURNS 1-5 (early conversation):
  Tier 2: empty (nothing old enough to summarize)
  Tier 3: [Turn 1, Turn 2, Turn 3, Turn 4, Turn 5] — all verbatim
  → Everything fits, no compression needed

TURNS 6-10 (window starts sliding):
  Tier 3 fills its token budget. When a new turn arrives:
  
  1. CHECK: Does tier 3 (recent turns) exceed its token budget?
  2. If YES: Evict the oldest turn(s) from tier 3
  3. BEFORE EVICTING: Score each turn's importance:
     - Architecture decisions / key constraints → importance 0.9
     - Error root causes / debugging breakthroughs → importance 0.8
     - Code generation that's still relevant → importance 0.7
     - Simple Q&A / explanations → importance 0.4
     - Formatting / typo fixes / acknowledgments → importance 0.1
  4. Evict LOWEST importance turns first (not strictly oldest)
  5. Evicted turns get compressed into a summary and added to tier 2

TURNS 20+ (long session):
  Tier 2 also has a budget. When it overflows:
  1. Merge the two oldest summaries into one more compressed summary
  2. Extract only key decisions / named entities / file references
  3. Drop details that are no longer referenced by recent turns

EXAMPLE — Turn 25 context payload:
  Tier 1: "Python FastAPI project, JWT auth, PostgreSQL, deployed on AWS"
  Tier 2: "Turns 1-8: Set up project structure, chose SQLAlchemy ORM,
           implemented User model with email/password fields, decided on
           bcrypt for hashing. Turns 9-15: Built auth endpoints (login,
           register, refresh), discovered bug in token expiry calculation
           (off-by-one in timedelta), fixed in auth/utils.py."
  Tier 3: [Turn 22: user asked to add rate limiting to login endpoint,
           assistant added slowapi middleware. Turn 23: user reported
           429 errors in tests, assistant fixed test fixtures. Turn 24:
           user asked to add admin role check, assistant modified
           auth/permissions.py]
  Tier 4: auth/utils.py (full), auth/permissions.py (full),
           tests/test_auth.py (relevant test functions only)
  Tier 5: "Now add logging for failed login attempts — store IP, 
           timestamp, and email in a new audit_log table"
```

### 0D: Importance Scoring — What to Keep, What to Compress

Not all turns are equal. The Session Manager uses a lightweight importance scorer to decide eviction order:

**High importance (keep verbatim as long as possible):**
- Turns where the user stated explicit constraints ("must be backwards compatible", "don't use ORM", "keep under 100ms latency")
- Turns where an architectural decision was made ("let's use Redis for caching", "switch to event-driven")
- Turns containing error root cause analysis (the debugging breakthrough)
- Turns where the user corrected the assistant ("no, I meant X not Y")
- Turns referencing files that are still in the active file set

**Low importance (compress or evict first):**
- Simple acknowledgments ("thanks", "looks good")
- Formatting-only requests ("add type hints", "fix indentation")
- Turns about files no longer in the active set
- Repeated/superseded information (if turn 5 says "use v1 API" but turn 12 says "actually switch to v2 API", turn 5's constraint is stale)
- Explanatory turns where the user asked "what does X do?" (the explanation was consumed, not a persistent constraint)

**Scoring algorithm:**
```
importance(turn) = 
    0.30 * has_explicit_constraint(turn)
  + 0.25 * references_active_files(turn)
  + 0.20 * is_decision_or_correction(turn)
  + 0.15 * recency_decay(turn)          # Even important turns decay over time
  + 0.10 * was_referenced_later(turn)    # If a later turn said "as we discussed..."
```

The local model can also be used to score importance: feed it a turn and ask "On a scale of 1-5, how important is this exchange for understanding the current task?" — this takes ~200ms and is more accurate than heuristics, but only triggered when the heuristic score is ambiguous (0.4-0.6 range).

### 0E: Context Payload Assembly — The Build Pipeline

When the balancer decides on a route and backend, the Session Manager assembles the context payload:

```
assemble_context(session, current_prompt, target_backend) → ContextPayload:

  budget = context_budgets[target_backend]
  
  # If latency predictor says we're near patience threshold, compress harder
  if latency_pressure:
    budget = budget * latency_pressure_reduction
  
  # TIER 1: System preamble (pre-computed, cached per session)
  tier1 = session.system_preamble  # refreshed when active_files change
  remaining = budget.total - tier1.tokens
  
  # TIER 5: Current prompt (always included in full)
  tier5 = current_prompt
  remaining -= tier5.tokens
  
  # TIER 4: Active code context
  tier4 = build_code_context(
    files=session.active_files,
    current_prompt=current_prompt,
    budget=min(budget.tier4, remaining * 0.45)
  )
  remaining -= tier4.tokens
  
  # TIER 3: Recent turns (fill greedily from most recent)
  tier3 = []
  for turn in reversed(session.turns):
    if turn.is_summarized: continue
    if turn.token_count > remaining * 0.7: break  # Reserve space for tier 2
    tier3.prepend(turn)
    remaining -= turn.token_count
  
  # TIER 2: Compressed history (fill with summaries covering evicted turns)
  tier2 = []
  for summary in session.summary_segments:
    if summary.token_count > remaining: break
    tier2.append(summary)
    remaining -= summary.token_count
  
  return ContextPayload(tier1, tier2, tier3, tier4, tier5)
```

### 0F: Per-Backend Thread Tracking — Avoiding Redundant Context

A subtle optimization: if the same cloud backend handled turns 8, 12, and 15 of the session, and now handles turn 18, does it need the summaries of turns 8, 12, and 15? **No** — if we're using the same backend's conversation thread.

Some backends support multi-turn conversation threads:
- **Gemini CLI:** Supports `--continue` flag or session files to maintain conversation state
- **Claude Code:** Maintains conversation within a running session via `--continue`/`--resume`
- **Cursor Agent:** Maintains state within an active Composer session

The Session Manager tracks per-backend state:

```
BACKEND_THREAD
├── backend: "claude_code"
├── thread_id: string                    # Backend's conversation ID if applicable
├── turns_seen: [8, 12, 15]             # Turns this backend has already processed
├── last_context_payload_hash: string    # Detect if context needs refreshing
└── is_alive: bool                       # Is the backend process/session still running?

OPTIMIZATION:
  If backend.is_alive AND backend has seen recent turns:
    → Send only: new turns since last interaction + current prompt
    → Skip: summaries of turns the backend already processed
    → Savings: potentially 50-70% fewer context tokens on follow-up calls

  If backend.is_alive BUT many local-only turns happened in between:
    → Send: compressed summary of intervening local turns + current prompt
    → The backend needs to know what happened while it was "away"

  If backend thread is dead (new process, timed out, different backend):
    → Send: full context payload (all tiers)
    → This is the "cold start" case — most expensive but unavoidable
```

### 0G: The Intervention Summary — Bridging Local↔Cloud Gaps

When the session has been running locally for several turns and suddenly escalates to cloud, the cloud model has zero memory. The Session Manager builds an **intervention summary** — a compact narrative that catches the cloud model up:

```
INTERVENTION SUMMARY TEMPLATE:
"""
[CONVERSATION CONTEXT]
You are continuing a development session. Here is what has happened so far:

Project: {{project_description}}
Active files: {{file_list}}

Session summary (turns 1-{{last_summarized_turn}}):
{{tier2_summaries}}

Recent exchanges (turns {{first_recent}}-{{last_recent}}):
{{tier3_turns_formatted}}

[CURRENT FILE STATE]
{{tier4_code_context}}

[CURRENT REQUEST]
The developer now asks:
{{tier5_current_prompt}}

IMPORTANT: This is turn {{current_turn_number}} of an ongoing session. 
The developer expects continuity — reference previous decisions and context 
in your response. Do not re-explain things that were already established.
"""
```

**The intervention summary is built by the local model** in ~300-500ms. It reads the tier 2 summaries and tier 3 recent turns, then produces a tight narrative paragraph that orients the cloud model. This is more effective than sending raw turn transcripts because:
- It resolves coreferences ("it" → "the auth module")
- It highlights what's still relevant vs. what's been superseded
- It's 30-50% shorter than the raw turns while preserving more meaning
- It's written in a format optimized for the target LLM's comprehension

### 0H: Session Persistence and Lifecycle

Sessions persist across LOKAHI restarts via SQLite:

```sql
CREATE TABLE sessions (
  session_id TEXT PRIMARY KEY,
  project_root TEXT NOT NULL,
  created_at TEXT NOT NULL,
  last_active TEXT NOT NULL,
  system_preamble TEXT,
  metadata_json TEXT
);

CREATE TABLE turns (
  turn_id INTEGER NOT NULL,
  session_id TEXT NOT NULL REFERENCES sessions,
  timestamp TEXT NOT NULL,
  role TEXT NOT NULL,              -- 'user', 'assistant', 'system'
  content TEXT NOT NULL,
  route_taken TEXT,
  backend_model TEXT,
  token_count INTEGER,
  files_referenced_json TEXT,
  files_modified_json TEXT,
  code_blocks_json TEXT,
  is_summarized INTEGER DEFAULT 0,
  importance_score REAL,
  PRIMARY KEY (session_id, turn_id)
);

CREATE TABLE summaries (
  summary_id INTEGER PRIMARY KEY AUTOINCREMENT,
  session_id TEXT NOT NULL REFERENCES sessions,
  covers_turn_start INTEGER NOT NULL,
  covers_turn_end INTEGER NOT NULL,
  content TEXT NOT NULL,
  token_count INTEGER NOT NULL,
  key_decisions_json TEXT,
  files_involved_json TEXT,
  created_at TEXT NOT NULL
);

CREATE TABLE backend_threads (
  session_id TEXT NOT NULL REFERENCES sessions,
  backend TEXT NOT NULL,
  thread_id TEXT,
  turns_seen_json TEXT,           -- [8, 12, 15]
  last_payload_hash TEXT,
  is_alive INTEGER DEFAULT 0,
  last_used TEXT,
  PRIMARY KEY (session_id, backend)
);
```

**Lifecycle rules:**
- Sessions auto-resume when LOKAHI starts in the same project directory
- Sessions expire after `session_ttl_hours` of inactivity (default: 72h)
- On expiry: session is archived (compressed), recent turns are summarized
- `lokahi session new` force-starts a fresh session
- `lokahi session list` shows active and archived sessions
- `lokahi session resume <id>` reloads an archived session
- `lokahi session export <id>` dumps full session as markdown (for sharing/review)

---

## Component 0.5: LOKAHI TUI — Claude Code-Style Interactive Frontend

**Purpose:** Provide a terminal-native, persistent, interactive development experience that feels like Claude Code but with LOKAHI's routing intelligence underneath.

### TUI Layout

```
┌─────────────────────────────────────────────────────────────────────────┐
│ LOKAHI v0.1.0  │  Session: auth-refactor-jwt  │  ~/project/backend/    │
├───────────────────────────────────┬─────────────────────────────────────┤
│                                   │  FILES                              │
│  [system] Session resumed.        │  ├── src/                           │
│  Project: backend/ (Python/       │  │   ├── auth/                      │
│  FastAPI)                         │  │   │   ├── routes.py  ✎           │
│  Context: 12 turns loaded,        │  │   │   ├── utils.py   ✎           │
│  3 summaries                      │  │   │   └── permissions.py         │
│                                   │  │   ├── models/                    │
│  [you] Add rate limiting to       │  │   │   └── user.py               │
│  the login endpoint               │  │   └── main.py                   │
│                                   │  ├── tests/                         │
│  [assistant → local, 1.2s]        │  │   └── test_auth.py  ✎           │
│  I'll add slowapi rate limiting   │  └── requirements.txt              │
│  to the login endpoint...         │                                     │
│  ```python                        │  GIT STATUS                         │
│  from slowapi import Limiter      │  M  src/auth/routes.py             │
│  ...                              │  M  requirements.txt               │
│  ```                              │  ?? src/auth/rate_limit.py         │
│                                   │                                     │
│  [you] The tests are returning    │  SESSION                           │
│  429 errors now                   │  Turns: 24  │  Summaries: 3        │
│                                   │  Tokens: 18.2k total               │
│  [assistant → hybrid, 4.8s]       │  Cloud calls: 7 (Claude: 4,       │
│  The issue is your test fixtures  │    Gemini: 3)                      │
│  aren't accounting for the rate   │  Cost today: $0.42                 │
│  limiter. Here's the fix...       │  Budget remaining: $1.58           │
│  [refined by claude-sonnet-4]    │                                     │
│  ```python                        │  LATENCY                           │
│  @pytest.fixture                  │  Last 5 calls:                     │
│  def no_rate_limit(app):          │  ● 1.2s local                     │
│  ...                              │  ● 4.8s hybrid                    │
│  ```                              │  ● 0.8s local                     │
│                                   │  ● 2.1s local                     │
│                                   │  ● 6.2s cloud (claude)            │
│                                   │                                     │
├───────────────────────────────────┴─────────────────────────────────────┤
│ [local ⚡ 42 tok/s] [gpu 34%] [session 24 turns, 3 summaries]          │
│ [budget $1.58/$2.00] [cloud: gemini ● claude ● cursor ○]               │
├─────────────────────────────────────────────────────────────────────────┤
│ > Now add logging for failed login attempts_                            │
└─────────────────────────────────────────────────────────────────────────┘
```

### TUI Features

**Core interaction model (like Claude Code):**
- Persistent REPL — type a prompt, get a response, session continues
- Responses stream token-by-token (local or cloud)
- Code blocks are syntax-highlighted inline
- File modifications are shown as inline diffs (green/red)
- Multi-line input with `Shift+Enter` or `\` continuation
- Command mode with `/` prefix: `/session`, `/explain`, `/cost`, `/clear`, `/undo`

**Session-aware features:**
- Status bar shows session depth (turns, summaries, token count)
- `/session info` — shows full session state, what's summarized, what's verbatim
- `/session context` — shows exactly what context payload would be sent to cloud right now
- `/session fork` — branch the session (useful for "let me try a different approach")
- `/session rewind N` — roll back N turns (undo in conversation, not just code)

**Routing transparency:**
- Each response shows `[→ route, latency]` tag: `[→ local, 1.2s]`, `[→ hybrid, 4.8s]`, `[→ cloud (claude), 6.2s]`
- Hybrid responses show the refinement inline: `[refined by claude-sonnet-4]`
- `/explain` after any response shows the full routing decision breakdown
- Latency sparkline in sidebar shows recent call patterns

**File and project awareness:**
- Right sidebar shows project file tree with modification indicators (✎)
- Git status integration (modified, staged, untracked)
- Files mentioned in conversation are auto-highlighted in tree
- `@file.py` syntax to explicitly attach a file to the prompt
- File watcher: if a file referenced in context changes externally, alert the user

**Keyboard shortcuts:**
```
Ctrl+C          Cancel current generation
Ctrl+L          Clear screen (keep session)
Ctrl+R          Search conversation history
Ctrl+T          Toggle sidebar
Tab             Auto-complete file paths / commands
Up/Down         Navigate prompt history
Ctrl+Shift+E    Open current response in $EDITOR
Ctrl+Shift+D    Show routing decision for last response
```

**Command palette:**

```
/session new              Start fresh session
/session info             Show session metadata and context state  
/session context          Preview context payload for next cloud call
/session fork [name]      Branch session
/session rewind [N]       Undo last N turns
/session export [path]    Export as markdown
/session list             List all sessions for this project
/session resume [id]      Resume an archived session

/route explain            Show why last prompt was routed where it was
/route force local        Force next prompt to local
/route force cloud [be]   Force next prompt to specific cloud backend
/route auto               Resume automatic routing

/cost today               Show today's cloud spending
/cost session             Show this session's cloud spending
/cost breakdown           Show cost by backend and intent

/file add [path]          Add file to active context
/file remove [path]       Remove file from active context
/file diff [path]         Show diff since session start

/model status             Show local model status (loaded, VRAM, tok/s)
/model switch [name]      Switch local model

/clear                    Clear screen (session preserved)
/help                     Show all commands
/quit                     Exit (session auto-saved)
```

### TUI Technology Stack

```yaml
tui_stack:
  framework: "ratatui"              # Rust TUI framework (or textual for Python)
  terminal_protocol: "crossterm"    # Cross-platform terminal handling
  syntax_highlighting: "syntect"    # Code highlighting engine
  diff_rendering: "similar"         # Inline diff computation
  markdown_rendering: "termimad"    # Markdown → terminal rendering
  file_watcher: "notify"            # File system change detection
  git_integration: "git2"           # libgit2 bindings for status/diff
```

---

## Component Specifications (Routing & Execution Pipeline)

### Component 1: Intake Parser

**Purpose:** Normalize and enrich every incoming prompt, emitting a feature vector the latency predictor can consume.

**Responsibilities:**
- Parse natural language intent (question, generation, refactor, debug, explain, test-write)
- Extract attached code context: identify language, file paths, diff hunks, error traces
- Compute a context window fingerprint (hash of relevant file state for cache lookups)
- **Emit latency-relevant features** (see feature vector below)

**Output feature vector:**
```json
{
  "intent": "refactor",
  "language": "python",
  "input_tokens": 2340,
  "estimated_output_tokens": 1200,
  "context_file_count": 3,
  "max_file_depth_lines": 480,
  "has_error_trace": false,
  "ast_node_count": 187,
  "cyclomatic_complexity": 12,
  "cross_file_references": 4,
  "requires_reasoning_chain": true,
  "similar_past_prompt_hash": "ef29a1",
  "cache_hit": false,
  "session_turn_number": 24,
  "session_context_tokens": 8200,
  "references_prior_turns": true,
  "coreference_count": 3,
  "active_backend_threads": ["claude_code"]
}
```

Note: `references_prior_turns` and `coreference_count` detect when the user's prompt depends on conversational history (e.g., "make that async too", "fix the same issue in the other file"). High coreference count means the context payload must be richer — the model can't resolve "that" and "the other file" without history.

**Local model usage:** A fine-tuned classifier (distilled from a larger model's routing decisions) that runs as a ~100M-500M parameter model. Candidates: Phi-3-mini, Qwen2.5-Coder-1.5B, or a custom LoRA on top of a small base.

**Key design decision:** The parser should be **fast** (<200ms) and **conservative** — when in doubt, overestimate output tokens so the latency predictor doesn't undercount. Surprising the user with a faster-than-expected response is always better than the reverse.

---

### Component 2: Latency Predictor (NEW — Core Component)

**Purpose:** Given the intake feature vector and current system state, predict the wall-clock latency for every possible execution path before the balancer makes its decision.

**Why this matters:** The developer's experience is dominated by how long they wait. A 3-second local answer that's 85% correct often beats a 12-second cloud answer that's 95% correct — especially for iterative coding where the developer will refine anyway. The latency predictor turns this intuition into a measurable, optimizable system.

#### 2A: Latency Factors — What Actually Drives Wall-Clock Time

**Local inference latency factors:**

| Factor | Impact | How to measure | Range |
|---|---|---|---|
| **Output token count** | DOMINANT — linear with generation time | Estimate from intent + input size | 50-8000 tokens |
| **Model size / quantization** | Sets tokens-per-second ceiling | Benchmark at startup | 15-120 tok/s |
| **GPU VRAM pressure** | KV cache eviction → recomputation | Monitor VRAM headroom | 0-100% |
| **Input context length** | Prefill time scales ~quadratically (attention) | Count tokens | 100-32000 tokens |
| **Batch queue depth** | Other requests ahead in the queue | Monitor inference server | 0-N requests |
| **Model warmth** | Cold model load vs. already in VRAM | Track last inference time | cold/warm/hot |
| **Quantization level** | Q4 is ~2x faster than FP16 but lower quality | Config parameter | Q2-FP16 |
| **Speculative decoding** | Draft model can accelerate by 2-3x | Track acceptance rate | 1-3x speedup |

**Cloud inference latency factors:**

| Factor | Impact | How to measure | Range |
|---|---|---|---|
| **Network RTT** | Fixed overhead per call | Periodic ping to API endpoints | 20-500ms |
| **API queue / load** | Unpredictable, time-of-day dependent | Track from historical calls | 100-15000ms |
| **Input token count** | Prefill time at the provider | Count after compression | 100-100000 tokens |
| **Output token count** | DOMINANT — same as local, but faster tok/s | Estimate from intent | 50-8000 tokens |
| **Time-to-first-token (TTFT)** | User perceives this as "is it working?" | Measure per backend | 200-5000ms |
| **Streaming vs. batch** | Streaming shows progress, reduces perceived wait | Config per backend | streaming/batch |
| **Backend-specific throttling** | Rate limits, quota exhaustion | Track 429s and retries | 0-60s penalty |
| **Prompt compression ratio** | Less input = faster prefill at cloud | Measure after compression | 0.3-1.0x |
| **Provider model routing** | Some providers route to faster/slower instances | Opaque, learn from data | variable |
| **Geographic proximity** | Closer data center = lower RTT | Config / auto-detect | 20-300ms |

**Hybrid pipeline latency factors (unique to hybrid route):**

| Factor | Impact | How to measure | Range |
|---|---|---|---|
| **Local draft generation time** | First stage; user sees streaming output | Predict from local factors | 500-5000ms |
| **Self-critique evaluation time** | Second stage; runs locally | ~30-50% of draft time | 200-2000ms |
| **Compression time** | Preparing the escalation prompt | Mostly deterministic | 50-200ms |
| **Cloud refinement time** | Typically shorter than full cloud (less output needed) | Predict from cloud factors | 500-8000ms |
| **Parallelism savings** | If draft streams while cloud processes | Architecture-dependent | 20-60% savings |

#### 2B: Latency Prediction Model

**Architecture:** A lightweight gradient-boosted tree (XGBoost or LightGBM) trained on accumulated latency observations. Not a neural net — this needs to run in <5ms and be interpretable.

**Input features (concatenated from intake + system state + session state):**
```
PROMPT FEATURES (from intake parser):
  - intent_onehot[8]           # 8 intent categories
  - language_onehot[12]        # top 12 languages + "other"
  - input_tokens               # log-scaled
  - estimated_output_tokens    # log-scaled
  - context_file_count
  - ast_node_count
  - cyclomatic_complexity
  - cross_file_references
  - requires_reasoning_chain   # boolean
  - has_error_trace            # boolean

SESSION STATE FEATURES (from Session Manager):
  - session_turn_number        # How deep into the conversation we are
  - session_context_tokens     # Total tokens in assembled context payload
  - tier2_summary_tokens       # How much compressed history we're carrying
  - tier3_recent_turns_count   # Number of verbatim turns in window
  - references_prior_turns     # Does prompt depend on earlier conversation?
  - coreference_count          # How many unresolved references ("it", "that")
  - active_backend_alive       # Is there a live thread to reuse? (huge latency win)
  - turns_since_last_cloud     # How many local-only turns since last cloud call
  - intervention_summary_needed # Will we need to build a catch-up summary?

SYSTEM STATE FEATURES (sampled at request time):
  - gpu_vram_utilization       # 0.0-1.0
  - gpu_compute_utilization    # 0.0-1.0
  - local_model_warmth         # 0=cold, 1=warm, 2=hot
  - local_queue_depth          # requests waiting
  - cpu_load_1min              # system load average
  - available_ram_gb

CLOUD STATE FEATURES (rolling averages):
  - gemini_p50_latency_ms      # last 50 calls
  - gemini_p95_latency_ms
  - gemini_last_429_minutes_ago # time since last rate limit
  - claude_p50_latency_ms
  - claude_p95_latency_ms
  - claude_last_429_minutes_ago
  - cursor_p50_latency_ms
  - cursor_p95_latency_ms
  - network_rtt_ms             # recent average
  - hour_of_day                # 0-23, captures API load patterns
  - day_of_week                # 0-6, weekday vs weekend patterns
```

**Critical latency impact of session context:** The session context payload directly affects cloud latency because more input tokens = longer prefill time. The latency predictor must account for this:

```
T_cloud_adjusted = T_cloud_base + (session_context_tokens / prefill_tok_per_sec * 1000)

# Example:
# Base cloud call for a simple prompt: 3000ms
# With 8000 tokens of session context at 30k tok/s prefill: +267ms
# With 24000 tokens of rich context at 30k tok/s prefill: +800ms
# This matters! It can push a call from "acceptable" to "tolerable" range

# But ALSO: reusing a live backend thread avoids resending context
# If active_backend_alive=true AND turns_since_last_cloud < 3:
#   T_cloud_adjusted = T_cloud_base + (new_turns_tokens / prefill_tok_per_sec * 1000)
#   Savings: often 60-80% of context tokens
```

**Output:** A latency envelope for each route:
```json
{
  "local": {
    "p50_ms": 1800,
    "p75_ms": 2400,
    "p95_ms": 4100,
    "ttft_ms": 120,
    "confidence": 0.87
  },
  "hybrid": {
    "p50_ms": 4200,
    "p75_ms": 5800,
    "p95_ms": 9500,
    "ttft_ms": 120,
    "user_sees_streaming_at_ms": 120,
    "refinement_arrives_at_ms": 4200,
    "confidence": 0.72
  },
  "cloud_gemini": {
    "p50_ms": 3100,
    "p75_ms": 4500,
    "p95_ms": 8200,
    "ttft_ms": 800,
    "confidence": 0.81
  },
  "cloud_claude": {
    "p50_ms": 4600,
    "p75_ms": 6200,
    "p95_ms": 11000,
    "ttft_ms": 1200,
    "confidence": 0.78
  },
  "cloud_cursor": {
    "p50_ms": 7200,
    "p75_ms": 10500,
    "p95_ms": 18000,
    "ttft_ms": 2500,
    "confidence": 0.65
  }
}
```

#### 2C: Output Token Estimation — The Hardest Subproblem

Output token count is the single biggest driver of latency (both local and cloud), but it's also the hardest to predict before generation starts. The system uses a multi-signal estimator:

**Signal 1: Intent-based priors.** Historical median output length per intent category:
```yaml
output_token_priors:
  explain_code:      { p50: 200,  p95: 600 }
  write_docstring:   { p50: 80,   p95: 200 }
  format_code:       { p50: 120,  p95: 400 }   # roughly mirrors input
  simple_refactor:   { p50: 300,  p95: 800 }
  complex_refactor:  { p50: 800,  p95: 3000 }
  debug:             { p50: 400,  p95: 1500 }
  generate_tests:    { p50: 600,  p95: 2500 }
  architecture:      { p50: 1500, p95: 5000 }
  security_audit:    { p50: 1000, p95: 4000 }
  git_commit_msg:    { p50: 30,   p95: 80 }
```

**Signal 2: Input-output ratio.** For generation tasks, output typically scales with input:
- Refactoring: output ≈ 0.8-1.2x input tokens (similar code, restructured)
- Test generation: output ≈ 1.5-3x the function under test
- Explanation: output ≈ 0.3-0.5x code being explained
- Docstrings: output ≈ 0.1-0.2x function body

**Signal 3: Similar-prompt lookup.** Hash the prompt's structural signature (intent + language + approximate input size bucket) and look up actual output lengths from the feedback database for similar past prompts.

**Signal 4: Explicit user constraints.** If the prompt says "keep it brief" or "comprehensive analysis", adjust the estimate accordingly. The intake parser extracts verbosity cues.

**Combined estimator:**
```
estimated_output_tokens = weighted_average(
    intent_prior_p50     * 0.30,
    input_ratio_estimate * 0.25,
    similar_prompt_actual * 0.35,  # 0 weight if no similar prompt found
    user_verbosity_adj   * 0.10
)
```

#### 2D: Bootstrapping — Before You Have Data

On first install, the latency predictor has no historical data. The bootstrap strategy:

**Week 1 (cold start):** Use hardcoded heuristic formulas:
```
T_local_ms  = (input_tokens / prefill_tok_per_sec) + (estimated_output_tokens / gen_tok_per_sec) + model_load_overhead
T_cloud_ms  = network_rtt + api_queue_estimate + (input_tokens / cloud_prefill_rate) + (estimated_output_tokens / cloud_gen_rate)
T_hybrid_ms = T_local_ms * 0.6 + compression_overhead + T_cloud_ms * 0.5
```

Where `prefill_tok_per_sec`, `gen_tok_per_sec` etc. are benchmarked once at startup by running calibration prompts through each backend.

**Week 2-4 (warming up):** Heuristic predictions are logged alongside actual measurements. Once 200+ observations accumulate, train the first XGBoost model. Run both heuristic and ML predictions in parallel, log both, switch to ML when its MAE beats the heuristic on a held-out validation set.

**Week 5+ (steady state):** Retrain weekly on rolling 30-day window. Monitor prediction error by category — if a specific intent or language shows >40% mean prediction error, flag it for investigation.

#### 2E: Latency Prediction Calibration

Raw point estimates aren't enough. The system needs calibrated confidence intervals so the balancer can account for uncertainty.

**Calibration method:** After each call, record `(predicted_p50, actual_latency)`. Periodically compute calibration curves: "When I predict 2000ms, what fraction of actual latencies fall below 2000ms?" Adjust prediction intervals so that the stated p50 is a true 50th percentile, p95 is a true 95th, etc.

**Why this matters for routing:** If the local prediction is `p50=1500ms, p95=4500ms` (wide interval, low confidence) vs cloud at `p50=3000ms, p95=3500ms` (tight interval, high confidence), the balancer might prefer cloud for reliability even though the local p50 is lower — because the risk of a 4.5s local surprise is high.

---

### Component 3: Prompt Balancer (Route Decision Engine)

**Purpose:** Given the latency envelope from the predictor, combined with quality risk and cost, make the routing decision.

**The key change from v1:** Latency prediction is now the **primary input**, not one dimension among equals. Quality and cost are secondary modifiers.

#### 3A: User Patience Profile

Different developers have different tolerance thresholds. The system learns these, but starts with sensible defaults:

```yaml
patience_profiles:
  default:
    instant_threshold_ms: 2000    # Below this: user won't even notice the wait
    acceptable_threshold_ms: 5000  # Below this: user is fine, no frustration
    tolerable_threshold_ms: 15000  # Below this: annoying but acceptable for hard tasks
    abandon_threshold_ms: 30000    # Above this: user will ctrl+C or context-switch
    
    # Streaming modifiers — streaming output buys patience
    streaming_patience_multiplier: 1.8  # Users tolerate 1.8x longer if they see output appearing
    ttft_critical_ms: 3000        # If nothing appears for 3s, user assumes it's broken
    
  impatient:                       # Learned from user who ctrl+C's frequently
    instant_threshold_ms: 1000
    acceptable_threshold_ms: 3000
    tolerable_threshold_ms: 8000
    abandon_threshold_ms: 15000
    
  patient:                         # Learned from user who waits for quality
    instant_threshold_ms: 3000
    acceptable_threshold_ms: 8000
    tolerable_threshold_ms: 30000
    abandon_threshold_ms: 60000
```

**How patience is learned:** Track user behavior signals:
- **Ctrl+C / cancel rate** per latency bucket → user's true abandon threshold
- **Immediate re-prompt rate** → user didn't wait for or didn't like the response
- **Edit distance after acceptance** → low edits at cloud-latency = user values quality; high edits at any latency = user just wants a fast starting point
- **Time between prompt and next action** → if user starts typing before response completes, they've already moved on mentally

#### 3B: Routing Decision Matrix

The balancer combines latency prediction with quality risk and cost into a single routing decision:

```
FOR EACH ROUTE r IN {local, hybrid, cloud_gemini, cloud_claude, cloud_cursor}:

  latency_score(r) = score based on where predicted p50 falls in patience profile
    - p50 < instant:     1.0 (perfect)
    - p50 < acceptable:  0.8
    - p50 < tolerable:   0.5
    - p50 < abandon:     0.2
    - p50 > abandon:     0.0 (eliminate this route)

  quality_score(r) = estimated answer quality for this task type
    - Lookup from historical acceptance rates per (route, intent, language)
    - Default: local=0.70, hybrid=0.88, cloud=0.93

  cost_score(r) = inverse cost normalized 0-1
    - local: 1.0 (free)
    - hybrid: 0.6 (some cloud tokens)
    - cloud: varies by backend and estimated tokens

  reliability_score(r) = based on prediction confidence interval width
    - Narrow p50-p95 spread → high reliability → 1.0
    - Wide p50-p95 spread → low reliability → 0.5
    - Backend recently had 429s/timeouts → penalty → 0.3

  COMBINED SCORE:
    score(r) = latency_score * W_latency
             + quality_score * W_quality
             + cost_score    * W_cost
             + reliability_score * W_reliability

  DEFAULT WEIGHTS:
    W_latency     = 0.40   # Latency is king
    W_quality     = 0.30   # But quality still matters
    W_cost        = 0.15   # Cost is a soft constraint
    W_reliability = 0.15   # Predictability matters for UX

SELECT: route with highest combined score
```

#### 3C: Latency-Aware Override Rules

Some situations override the scoring entirely:

```yaml
latency_overrides:
  # SPEED OVERRIDES — always pick fastest viable route
  - condition: "user typed --fast flag"
    action: "pick route with lowest predicted p50, ignore quality_score"
    
  - condition: "user is in a rapid iteration loop (>3 prompts in 60s)"
    action: "bias W_latency to 0.60, reduce W_quality to 0.20"
    reason: "User is exploring, not seeking final answers"

  # QUALITY OVERRIDES — accept latency for correctness
  - condition: "intent is security_audit OR architecture_design"
    action: "set minimum quality_score threshold 0.85, eliminate routes below"
    reason: "Getting this wrong has high downstream cost"
    
  - condition: "user typed --thorough flag"
    action: "bias W_quality to 0.50, reduce W_latency to 0.25"

  # BUDGET OVERRIDES — hard economic constraints
  - condition: "daily cloud budget > 80% consumed"
    action: "force local unless quality_score(local) < 0.40"
    
  - condition: "daily cloud budget exhausted"
    action: "force local always, warn user"

  # SYSTEM STATE OVERRIDES
  - condition: "local GPU utilization > 90%"
    action: "boost cloud latency_score by 0.2 (local will be slow)"
    
  - condition: "network_rtt > 500ms OR last cloud call timed out"
    action: "boost local latency_score by 0.3 (cloud will be slow/unreliable)"
    
  - condition: "model not loaded (cold start)"
    action: "add estimated_load_time to local p50, may route to cloud for first call"
```

#### 3D: The Streaming Advantage — Hybrid's Secret Weapon

The hybrid route has a unique latency property: **the user sees output almost immediately** (local model starts streaming in ~100-200ms) even though the final, refined answer arrives later.

This is why the patience profile has a `streaming_patience_multiplier`. In practice:
- Local route: user waits T_local and gets the answer
- Cloud route: user waits T_cloud (staring at a spinner)  
- Hybrid route: user sees draft in ~200ms, gets refined answer at T_hybrid

For a user with a 5s acceptable threshold, hybrid with streaming effectively has a 9s acceptable threshold (5s × 1.8). This often makes hybrid the **best perceived-latency route** even when its total wall-clock time is longer than cloud.

**UX implementation:**
```
[0ms]     → Start local generation, begin streaming tokens to terminal
[200ms]   → User sees first tokens appearing (feels instant)
[1500ms]  → Local draft complete, shown to user with subtle "refining..." indicator
[1500ms]  → Simultaneously: compressed prompt sent to cloud
[4500ms]  → Cloud response arrives
[4600ms]  → Diff the cloud refinement against local draft
[4700ms]  → Smooth in-place update: changed sections highlight briefly, then settle
            User never experienced a "blank screen" moment
```

#### 3E: Prompt Compression Pipeline (for CLOUD and HYBRID routes)

When escalating to cloud, the balancer works with the Session Manager to build an optimal payload:

1. **Session Manager assembles context payload** — builds the tiered context (system preamble + compressed history + recent turns + code context + current prompt) within the target backend's token budget
2. **Checks for live backend thread** — if the target backend has an active session, sends only delta turns (new turns since last interaction) instead of full context, saving 50-80% of context tokens
3. **Builds intervention summary if needed** — when switching from local to cloud after several local-only turns, the local model generates a 200-400 token catch-up narrative for the cloud model
4. **Strips redundant context within turns** — removes boilerplate imports, unchanged file sections, repeated code blocks that appear in multiple turns
5. **Rewrites the prompt** — converts verbose developer stream-of-consciousness into a tight, structured prompt optimized for the target backend
6. **Injects format directives** — adds output format constraints to reduce wasted tokens in the response (e.g., "respond only with the modified function, no explanation")
7. **Latency-aware compression level** — if the latency prediction shows cloud is near the patience threshold, the Session Manager receives a tighter token budget (60% of normal via `latency_pressure_reduction`), triggering more aggressive summarization and context trimming

This combined session-aware compression reduces token usage by 50-70% compared to sending raw conversation history, and the live-thread optimization adds another 30-50% reduction on follow-up cloud calls.

---

### Component 4: Cloud Backend Selector

**Purpose:** When cloud is needed, pick the best backend for this specific task, with **latency as the primary selection criterion**.

**Backend profiles (latency-centric):**

```yaml
backends:
  gemini_cli:
    strengths: [long_context, multimodal, fast_streaming, broad_knowledge]
    weaknesses: [sometimes_verbose, weaker_on_niche_frameworks]
    cost_per_1k_tokens: 0.0005
    max_context: 1000000
    # LATENCY PROFILE (calibrated from observations, updated continuously)
    latency_profile:
      ttft_p50_ms: 400            # Time to first token — fast
      ttft_p95_ms: 1200
      generation_tok_per_sec: 80   # Fast generation
      prefill_tok_per_sec: 50000   # How fast it processes input
      fixed_overhead_ms: 200       # API overhead regardless of payload
      queue_sensitivity: low       # Relatively stable latency
      peak_hours_penalty_ms: 500   # Extra latency during 9am-5pm PST
    best_for: [large_codebase_questions, documentation, multi_file_refactor]
    
  claude_code:
    strengths: [precise_reasoning, code_quality, instruction_following, safety]
    weaknesses: [smaller_context_than_gemini, higher_cost]
    cost_per_1k_tokens: 0.003
    max_context: 200000
    latency_profile:
      ttft_p50_ms: 800            # Slower first token
      ttft_p95_ms: 2500
      generation_tok_per_sec: 60
      prefill_tok_per_sec: 30000
      fixed_overhead_ms: 300
      queue_sensitivity: medium    # Can spike during peak
      peak_hours_penalty_ms: 1200
    best_for: [complex_logic, debugging, architecture, security_review]
    
  cursor_agent:
    strengths: [ide_integration, file_editing, multi_step_execution, tool_use]
    weaknesses: [less_controllable, opaque_routing, multi_step_adds_latency]
    cost_per_1k_tokens: variable
    max_context: 120000
    latency_profile:
      ttft_p50_ms: 1500           # Slowest first token (agentic overhead)
      ttft_p95_ms: 4000
      generation_tok_per_sec: 50
      prefill_tok_per_sec: 20000
      fixed_overhead_ms: 800       # Agent setup overhead
      queue_sensitivity: high      # Most variable latency
      peak_hours_penalty_ms: 2000
      # Agentic multiplier: multi-step tasks multiply base latency
      agentic_step_overhead_ms: 1500  # Per tool-use step
      estimated_steps_for_intent:
        multi_file_edit: 3
        test_generation: 2
        scaffolding: 4
    best_for: [multi_file_edits, test_generation, project_scaffolding]
```

**Selection logic (latency-first):**

```
1. Compute predicted latency for each enabled backend:
   T_backend = fixed_overhead
             + (input_tokens / prefill_tok_per_sec * 1000)
             + ttft_p50
             + (estimated_output_tokens / generation_tok_per_sec * 1000)
             + peak_hours_penalty (if applicable)
             + agentic_step_overhead * estimated_steps (if cursor)
             + rolling_adjustment (from recent observations)

2. Eliminate backends where T_backend > user's abandon_threshold

3. Among remaining, score by:
   backend_score = (1 / T_backend) * W_speed
                 + quality_fit     * W_quality
                 + (1 / cost)      * W_cost

4. Select highest-scoring backend
```

---

### Component 5: Local Model Tier

**Purpose:** Handle the 60-80% of developer tasks that don't need a frontier model.

**What local handles well:**
- Code completion and boilerplate generation
- Docstring and comment writing
- Simple refactors (rename, extract function, inline variable)
- Regex construction
- Format conversion (JSON ↔ YAML ↔ TOML)
- Error message explanation (for common/known errors)
- Git commit message generation
- Simple test scaffolding
- Code explanation for small snippets
- Linting and style suggestions

**Model candidates (with latency benchmarks):**

| Model | Parameters | VRAM | Tokens/sec (Q4) | Best for | Typical latency for 200-tok output |
|---|---|---|---|---|---|
| Qwen2.5-Coder-7B | 7B | ~6GB | 40-60 tok/s | General code, quality ceiling | 3-5s |
| DeepSeek-Coder-V2-Lite | 2.4B active | ~10GB | 50-70 tok/s | Fast + capable | 2.5-4s |
| Phi-3-mini-4k | 3.8B | ~3GB | 60-90 tok/s | Ultra-fast simple tasks | 2-3s |
| CodeGemma-2B | 2B | ~2GB | 80-120 tok/s | Minimal latency, completions | 1.5-2.5s |
| StarCoder2-3B | 3B | ~3GB | 60-80 tok/s | Code, multilingual | 2.5-3.5s |

**Latency optimization strategies for local tier:**
- **Model tiering within local:** Use 2B model for tasks predicted <1s output, 7B model for harder local tasks
- **Speculative decoding:** Use 2B draft model + 7B verifier for 1.5-2.5x speedup
- **Persistent KV cache:** Keep the last N file contexts in GPU memory to avoid re-prefilling
- **Quantization auto-selection:** Switch to heavier quantization (Q2) when VRAM is tight, lighter (Q8) when there's headroom

**Recursive small model pattern:**
For tasks in the hybrid band, use a chain-of-thought loop with the local model:
1. Local model generates initial draft
2. Local model self-critiques the draft (separate prompt)
3. If self-critique score > threshold → ship it locally
4. If self-critique score < threshold → escalate draft + critique to cloud for refinement
5. Cloud model receives a **pre-digested** problem, not raw — saving significant tokens

---

### Component 6: Hybrid Execution Pipeline

**Purpose:** The key differentiator. Local does the heavy lifting; cloud does precision finishing. **Latency is managed by parallelism and progressive disclosure. Session context flows to both tiers.**

**Pattern: Draft-and-Refine with Session Context (with latency annotations)**
```
SESSION STATE at turn 24:
  Tier 2 summaries: "Turns 1-15: Built JWT auth system, bcrypt hashing, 
    token refresh flow. Fixed off-by-one in expiry. Added slowapi rate limiting."
  Tier 3 recent: [Turn 20: added admin role, Turn 21: refactored permissions,
    Turn 22: added rate limiting, Turn 23: fixed test 429s]
  Active backend threads: claude_code (alive, last used turn 21)
  Active files: auth/routes.py, auth/utils.py, tests/test_auth.py

Developer asks (turn 24): "Now add logging for failed login attempts"

LATENCY PREDICTION:
  local_only:   p50=3200ms  (local can do it, has full session in KV cache)
  hybrid:       p50=5800ms  (streaming at 150ms, but cloud needs session context)
  cloud_claude: p50=4100ms  (live thread! only 3 new turns to send, not full history)
  cloud_gemini: p50=5500ms  (cold start, needs full context payload = ~8k tokens)
  
ROUTE DECISION: HYBRID (via claude, reusing live thread)
  Reason: live claude thread means cloud cost is low (delta context only).
          hybrid gives instant streaming + claude refinement.

Step 1 (LOCAL, starts at T=0ms):
  Session Manager provides: full session from local KV cache (no rebuild needed)
  T=0ms:     Prefill begins (session context already warm in KV cache)
  T=150ms:   First token streams to user
  T=2800ms:  Local draft complete. Self-critique flags: missing audit_log model
  
Step 2 (CLOUD via live claude thread, starts at T=2900ms):
  Session Manager provides: 
    NOT full context (claude already saw turns 1-21)
    ONLY: intervention summary of turns 22-23 (~200 tokens)
         + current code state of auth/routes.py (changed since turn 21)
         + draft + critique from Step 1
  Total context sent: ~1200 tokens (vs ~8000 if cold start)
  
  T=2900ms:  API request dispatched to live claude session
  T=3500ms:  Claude TTFT (fast — small payload)
  T=5200ms:  Claude response complete (adds AuditLog model + migration)

Step 3 (MERGE, T=5250ms):
  T=5300ms:  In-place update with highlighted changes
  
  Session Manager: commits turn 24 to session log
    → Records: route=hybrid, backend=claude_code, tokens_sent=1200
    → Updates: claude thread.turns_seen += [24]
    → Importance score: 0.7 (new model creation, referenced later)
  
TOTAL WALL CLOCK: 5300ms
CONTEXT TOKENS SENT TO CLOUD: 1200 (vs 8000 cold start — 85% savings!)
```

**Pattern: Cloud Cold Start — When No Thread is Alive**
```
SESSION STATE at turn 30:
  Last cloud call was turn 21 (claude). Thread has since timed out.
  9 local-only turns since then.

Developer asks (turn 30): "I'm seeing a race condition in the auth flow 
                           under concurrent requests. Debug this."

ROUTE DECISION: CLOUD (claude) — cold start, complex debugging
  This is the expensive case. No live thread. Cloud needs full context.

Session Manager assembles FULL CONTEXT PAYLOAD:
  Tier 1 (system):     "Python FastAPI, JWT auth, PostgreSQL..."    [400 tok]
  Tier 2 (summaries):  Compressed turns 1-20                        [1200 tok]
  Tier 3 (recent):     Turns 25-29 verbatim                         [3800 tok]
                        (turns 21-24 were medium importance, now summarized)
  Tier 4 (code):       auth/routes.py, auth/utils.py, models/user.py [4200 tok]
  Tier 5 (prompt):     Current debugging request                     [180 tok]
  
  PLUS: Intervention summary built by local model in ~400ms:
    "Since your last interaction with this codebase (turn 21, where
     you helped restructure the permissions module), the developer 
     has added: rate limiting via slowapi, an audit_log table for 
     failed logins, pagination on the admin user list, and a health 
     check endpoint. The auth flow now has 3 middleware layers: 
     rate_limit → jwt_verify → permission_check."
  
  TOTAL PAYLOAD: ~10,200 tokens → latency impact: +340ms prefill

LATENCY: 6800ms total (vs ~4100ms with live thread — 65% slower)
  This is why keeping backend threads alive matters!
```

**Pattern: Conversation Context Saves a Bad Route**
```
Turn 18 (user): "Use the v2 API for the webhook handler"
Turn 19 (local): [generates webhook handler using v2 API]
Turn 20 (user): "Now test it"
Turn 21 (local): [generates tests, but uses v1 API patterns — local model 
                  doesn't understand that "it" refers to the v2 handler]

WITHOUT SESSION CONTEXT: local model generates v1-style tests. User is confused.
  "I just told you to use v2!" — user has to re-explain.

WITH SESSION CONTEXT: the sliding window includes turn 18 verbatim.
  Local model sees the v2 constraint, generates correct tests.
  Even if turn 18 has been summarized, the summary includes:
  "User specified v2 API for webhook handler (turn 18)"

If local STILL gets it wrong, and hybrid escalates to cloud:
  Cloud receives: summary mentioning v2 constraint + recent turns + code
  Cloud generates correct v2-style tests because it has the full picture.
```

**Pattern: Latency Race**
When prediction confidence is low, race local and cloud in parallel:
```
Developer asks: "Why is this test failing?" + 3 files + error trace

LATENCY PREDICTION:
  local:       p50=2500ms, confidence=0.55  (might solve it, might not)
  cloud_gemini: p50=3800ms, confidence=0.82

LOW CONFIDENCE ON LOCAL → trigger race mode:

  T=0ms:     Start local generation AND cloud request simultaneously
             Session Manager provides context to BOTH:
               - Local: full session from KV cache
               - Cloud: assembled context payload (~8k tokens)
  T=150ms:   Local starts streaming a hypothesis
  T=2200ms:  Local completes. Self-critique says: confidence=0.72 (decent)
  T=3600ms:  Cloud response arrives.
  
  DECISION POINT:
    If local self-critique > 0.80 → show local, discard cloud (save cost? no, already sent)
    If local self-critique < 0.80 → replace with cloud response
    Always: log both for training data (free quality comparison)
    Always: commit winning response to session as the "assistant" turn
```

**Pattern: Context Compression**
```
Developer asks: "Why is this test failing?" + 3 files + error trace

Step 1 (LOCAL):
  - Parse error trace → identify failing assertion
  - Trace call chain through the 3 files
  - Narrow down to the 2 relevant functions + the test

Step 2 (CLOUD — if local can't solve):
  Send only: session context + failing test + 2 relevant functions + error message
  Not: full session + 3 full files + entire test suite + raw trace
```

---

### Component 7: Feedback & Learning Loop

**Purpose:** The system gets smarter over time by learning from routing outcomes, **with latency prediction accuracy as the primary optimization target**.

**Data collected per interaction:**
```json
{
  "timestamp": "2026-03-19T10:30:00Z",
  "session_id": "auth-refactor-jwt",
  "turn_number": 24,
  "prompt_hash": "abc123",
  "intent": "debug",
  "feature_vector": { ... },
  
  "session_context": {
    "total_session_turns": 24,
    "tier2_summary_tokens": 1200,
    "tier3_recent_turn_count": 5,
    "tier3_recent_turn_tokens": 3800,
    "tier4_code_context_tokens": 4200,
    "total_context_payload_tokens": 9780,
    "backend_thread_alive": true,
    "delta_tokens_sent": 1200,
    "intervention_summary_needed": false,
    "coreference_count": 3,
    "context_assembly_ms": 45
  },

  "latency_predictions": {
    "local":        { "p50": 2500, "p95": 4100, "confidence": 0.82 },
    "hybrid":       { "p50": 5200, "p95": 8500, "confidence": 0.71 },
    "cloud_gemini":  { "p50": 3800, "p95": 7200, "confidence": 0.78 },
    "cloud_claude":  { "p50": 5100, "p95": 9800, "confidence": 0.75 }
  },
  
  "route_chosen": "hybrid",
  "cloud_backend": "claude_code",
  
  "actual_latencies": {
    "intake_parse_ms": 85,
    "latency_prediction_ms": 3,
    "context_assembly_ms": 45,
    "intervention_summary_ms": 0,
    "local_prefill_ms": 220,
    "local_ttft_ms": 380,
    "local_generation_ms": 2650,
    "self_critique_ms": 890,
    "prompt_compression_ms": 120,
    "cloud_dispatch_ms": 15,
    "cloud_ttft_ms": 950,
    "cloud_generation_ms": 2100,
    "response_merge_ms": 45,
    "session_commit_ms": 12,
    "total_wall_clock_ms": 5280,
    "user_perceived_wait_ms": 380
  },
  
  "prediction_errors": {
    "hybrid_p50_error_ms": -80,
    "hybrid_p50_error_pct": -1.5
  },
  
  "token_counts": {
    "input_tokens_raw": 3800,
    "input_tokens_compressed": 820,
    "session_context_tokens_sent": 1200,
    "session_context_tokens_saved_by_thread_reuse": 6800,
    "output_tokens_local": 340,
    "output_tokens_cloud": 180,
    "compression_ratio": 0.216
  },
  
  "quality_signals": {
    "user_accepted": true,
    "user_edited_output": false,
    "edit_distance_if_edited": 0,
    "user_re_prompted": false,
    "user_cancelled": false,
    "time_to_next_prompt_ms": 45000,
    "cloud_response_was_contextually_coherent": true
  },
  
  "cost_usd": 0.0024
}
```

**Learning mechanisms:**

1. **Latency model retraining (weekly):** Retrain XGBoost on rolling 30-day window. Split by intent category — if one category's MAE is >30%, train a specialized sub-model for it.

2. **Backend latency profile updates (daily):** Recompute per-backend p50/p95/TTFT from the last 7 days of observations. Detect if a backend has gotten consistently faster or slower (provider-side model updates, infrastructure changes).

3. **Output token estimator calibration (weekly):** Compare estimated vs actual output tokens. Update the intent-based priors and input-output ratios.

4. **Patience profile learning (continuous):** Track cancel rates, re-prompt rates, and edit distances across latency buckets. Shift patience thresholds toward observed behavior.

5. **Routing quality assessment (weekly):** For interactions where both local and cloud ran (race mode, or hybrid), compare quality. If local quality is improving for certain intent/language combinations, shrink the cloud band. If local quality is declining, expand it.

6. **Drift detection alerts:**
   - Latency prediction MAE exceeds 40% for 3+ consecutive days → alert, force recalibration
   - A backend's p95 exceeds 2x its configured profile → alert, temporarily deprioritize
   - Local model acceptance rate drops below 60% → alert, review routing thresholds
   - Cost budget consumption rate extrapolated to exceed daily limit → alert, tighten cloud routing

---

## Implementation Roadmap

### Phase 1: TUI Shell + Session Manager + Heuristic Router (Weeks 1-4)

**Deliverable:** A working Claude Code-style TUI with session persistence and heuristic latency routing.

```
$ lokahi                                    # Start TUI in current directory
$ lokahi --project ~/code/backend/          # Start in specific project
$ lokahi session resume auth-refactor       # Resume a named session
```

**Tasks:**
- [ ] Set up project structure (Rust or Python — Rust preferred for speed)
- [ ] **Build TUI shell with ratatui/crossterm (or textual for Python)**
- [ ] **Implement REPL input loop with streaming output**
- [ ] **Build Session Manager with SQLite persistence**
- [ ] **Implement sliding window context protocol (5-tier payload assembly)**
- [ ] **Implement turn importance scoring (heuristic-based initially)**
- [ ] **Build intervention summary generator (local model writes catch-up narratives)**
- [ ] Implement intake parser with regex + heuristic classifier (no ML yet)
- [ ] Build heuristic latency predictor (formula-based, see 2D bootstrap)
- [ ] Run startup calibration: benchmark local model tok/s, ping cloud endpoints
- [ ] Build rule-based prompt balancer with YAML config
- [ ] Integrate local model via llama.cpp / Ollama / vLLM
- [ ] Integrate Gemini CLI as first cloud backend **with session context passing**
- [ ] Integrate Claude Code (via `claude` CLI) as second backend **with thread reuse**
- [ ] Integrate Cursor Agent (via Cursor's API or CLI hooks) **with session context**
- [ ] **Implement per-backend thread tracking and delta context optimization**
- [ ] Build prompt compression pipeline (template-based initially)
- [ ] Add basic caching (prompt hash → response, with TTL)
- [ ] **TUI: status bar, route indicators, session depth display**
- [ ] **TUI: file tree sidebar with modification indicators**
- [ ] **Begin logging all latency observations to SQLite**

### Phase 2: Hybrid Pipeline + ML Latency Predictor + Smart Context (Weeks 5-7)

**Deliverable:** The draft-and-refine loop works with session context flowing correctly. Latency predictions replace heuristics.

**Tasks:**
- [ ] Implement local self-critique loop
- [ ] Build AST-aware code context extractor (tree-sitter based)
- [ ] Implement intelligent prompt rewriting (local model rewrites prompts for cloud)
- [ ] Train first XGBoost latency predictor on accumulated data (needs ~200+ observations)
- [ ] **Add session context tokens as latency prediction feature**
- [ ] Implement output token estimator with multi-signal approach
- [ ] Add backend selection scoring (latency-first)
- [ ] Build response assembler (merge local + cloud outputs)
- [ ] Implement streaming UX for hybrid route (progressive disclosure)
- [ ] Add diff-aware context: only send changed code + surrounding context
- [ ] Implement latency-aware compression (compress harder when near patience threshold)
- [ ] **Implement ML-based turn importance scoring (replace heuristics)**
- [ ] **Build summary compaction pipeline (merge old summaries when tier 2 overflows)**
- [ ] **TUI: session fork/rewind commands**
- [ ] **TUI: /session context preview (show what would be sent to cloud)**
- [ ] **TUI: git integration in sidebar**
- [ ] Build token counting and cost tracking dashboard
- [ ] **Add latency prediction vs actual comparison in logs**
- [ ] **Track context coherence: did cloud give gibberish? correlate with context quality**

### Phase 3: Learning Loop + IDE Integration (Weeks 8-11)

**Deliverable:** The system improves over time; works inside editors; latency predictions are accurate; session context is optimized by learning.

**Tasks:**
- [ ] Implement full feedback collection pipeline (SQLite → analytics)
- [ ] Build weekly latency model retraining pipeline
- [ ] Implement patience profile learning from user behavior
- [ ] Train distilled routing classifier on accumulated data
- [ ] **Train context quality model: which context payloads led to coherent cloud responses?**
- [ ] **Optimize sliding window parameters from data (tier sizes, importance thresholds)**
- [ ] Build VS Code extension (intercepts Copilot/Cursor-style triggers, shares session)
- [ ] Build Neovim plugin (for terminal-first workflows, shares session)
- [ ] Add A/B testing framework (route same prompt both ways, compare)
- [ ] Dashboard: latency prediction accuracy, cost savings, routing distribution
- [ ] **Dashboard: session analytics (avg depth, context efficiency, thread reuse rate)**
- [ ] Implement race mode for low-confidence predictions
- [ ] Add drift detection and alerting
- [ ] **TUI: Ctrl+R conversation search, keyboard shortcuts**
- [ ] **TUI: inline diff rendering for file modifications**

### Phase 4: Advanced Features (Weeks 12+)

- [ ] Multi-model local ensemble (2B for fast tasks, 7B for harder tasks, auto-selected by latency prediction)
- [ ] Speculative decoding integration for 2-3x local speedup
- [ ] Persistent KV cache across prompts (avoid re-prefilling same files)
- [ ] Codebase-aware fine-tuning pipeline (LoRA on your repo's patterns)
- [ ] Agentic mode: local model decomposes complex tasks into subtasks, predicts latency per subtask, routes each independently
- [ ] Semantic caching: cache not just exact prompts but semantically similar ones
- [ ] **Cross-session memory: extract persistent facts/conventions from old sessions into a project knowledge base that seeds new session system preambles**
- [ ] **Session branching UI: visual session tree with fork/merge/rewind**
- [ ] **Multi-user sessions: shared session state for pair programming (via shared SQLite or CRDT)**
- [ ] Team mode: shared routing profiles, latency observations, and cached responses
- [ ] Prompt replay: replay past cloud calls through local model to test if local latency is now sufficient
- [ ] Predictive prewarming: if user is editing a file, preload relevant context into KV cache before they even ask
- [ ] **Predictive context assembly: start building the cloud context payload before the user finishes typing, based on file-watcher signals and typing patterns**

---

## File / Directory Structure

```
lokahi/
├── Cargo.toml (or pyproject.toml)
├── config/
│   ├── default.yaml          # Default routing rules, patience profiles, thresholds
│   ├── backends.yaml          # Cloud backend profiles, latency baselines, credentials
│   ├── models.yaml            # Local model configs (paths, quantization, tok/s benchmarks)
│   └── session.yaml           # Session and context window configuration
├── src/
│   ├── main.rs                # Entrypoint — launches TUI or headless mode
│   ├── tui/                   # Claude Code-style terminal frontend
│   │   ├── app.rs             # Main TUI application loop
│   │   ├── input.rs           # REPL input handling, multi-line, history
│   │   ├── output.rs          # Streaming token renderer, syntax highlighting
│   │   ├── layout.rs          # Split pane layout (chat | sidebar)
│   │   ├── sidebar.rs         # File tree, git status, session info panels
│   │   ├── status_bar.rs      # Route indicator, latency, cost, session depth
│   │   ├── diff_view.rs       # Inline diff rendering for file modifications
│   │   ├── commands.rs        # /session, /route, /cost, /file, /model handlers
│   │   └── themes.rs          # Color schemes and styling
│   ├── session/               # Session Manager — owns all conversational state
│   │   ├── manager.rs         # Core session lifecycle (create, resume, commit, expire)
│   │   ├── turn.rs            # Turn data model and CRUD
│   │   ├── sliding_window.rs  # Tier 2/3 eviction, importance-based sliding window
│   │   ├── summarizer.rs      # Turn → summary compression (uses local model)
│   │   ├── importance.rs      # Turn importance scoring (heuristic + ML)
│   │   ├── context_builder.rs # 5-tier context payload assembly for any backend
│   │   ├── intervention.rs    # Intervention summary generator (catch-up narratives)
│   │   ├── thread_tracker.rs  # Per-backend thread state and delta optimization
│   │   ├── persistence.rs     # SQLite read/write for sessions, turns, summaries
│   │   └── preamble.rs        # System preamble auto-generation from project structure
│   ├── intake/
│   │   ├── parser.rs          # Prompt parsing and intent classification
│   │   ├── context.rs         # Code context extraction (tree-sitter)
│   │   ├── tokenizer.rs       # Token counting and output estimation
│   │   ├── coreference.rs     # Detect references to prior turns ("it", "that", etc.)
│   │   └── features.rs        # Feature vector construction for latency predictor
│   ├── latency/               # Latency prediction subsystem
│   │   ├── predictor.rs       # Main prediction interface (heuristic → ML transition)
│   │   ├── heuristic.rs       # Formula-based predictions (bootstrap phase)
│   │   ├── ml_model.rs        # XGBoost/LightGBM inference wrapper
│   │   ├── output_estimator.rs # Output token count estimation
│   │   ├── system_monitor.rs  # GPU/CPU/RAM/queue state sampling
│   │   ├── network_monitor.rs # RTT tracking, backend health checks
│   │   ├── calibrator.rs      # Prediction interval calibration
│   │   └── benchmarker.rs     # Startup calibration (measure tok/s, RTT, etc.)
│   ├── balancer/
│   │   ├── scorer.rs          # Route scoring (latency + quality + cost + reliability)
│   │   ├── router.rs          # Route decision logic + override rules
│   │   ├── patience.rs        # User patience profile management
│   │   ├── compressor.rs      # Prompt compression pipeline (works with session ctx)
│   │   └── rewriter.rs        # Prompt rewriting for cloud optimization
│   ├── local/
│   │   ├── inference.rs       # Local model inference (llama.cpp bindings)
│   │   ├── self_critique.rs   # Recursive self-evaluation loop
│   │   ├── ensemble.rs        # Multi-model coordination (2B fast / 7B quality)
│   │   └── kv_cache.rs        # Persistent KV cache management (session-aware)
│   ├── cloud/
│   │   ├── gemini.rs          # Gemini CLI integration (with session context)
│   │   ├── claude.rs          # Claude Code integration (with thread reuse)
│   │   ├── cursor.rs          # Cursor Agent integration (with session context)
│   │   ├── selector.rs        # Backend selection (latency-first scoring)
│   │   └── adapter.rs         # Common backend adapter trait (context payload interface)
│   ├── assembler/
│   │   ├── merge.rs           # Response merging (local + cloud)
│   │   ├── validate.rs        # Output validation (AST check, lint)
│   │   ├── cache.rs           # Semantic and exact-match caching
│   │   └── streaming.rs       # Progressive disclosure UX (draft → refinement)
│   └── feedback/
│       ├── collector.rs       # Interaction + latency + context quality logging
│       ├── analyzer.rs        # Prediction accuracy analysis, drift detection
│       ├── trainer.rs         # Latency model + classifier retraining
│       ├── patience_learner.rs # User patience profile updates
│       └── context_learner.rs  # Learn optimal context window sizes from outcomes
├── models/                    # Local model weights (gitignored)
├── data/
│   ├── sessions.db            # SQLite: sessions, turns, summaries, backend threads
│   ├── feedback.db            # SQLite: interactions, latency observations, quality signals
│   ├── cache.db               # Response cache
│   ├── latency_model.bin      # Trained XGBoost/LightGBM model
│   └── calibration.json       # Prediction calibration curves
├── plugins/
│   ├── vscode/                # VS Code extension (shares session state)
│   └── neovim/                # Neovim plugin (shares session state)
└── tests/
    ├── routing/               # Test that prompts route correctly
    ├── session/               # Test session persistence, sliding window, context assembly
    ├── context/               # Test context payload correctness for each backend
    ├── latency/               # Test latency prediction accuracy on fixtures
    ├── compression/           # Test prompt compression quality
    ├── tui/                   # TUI rendering and interaction tests
    └── integration/           # End-to-end with mock backends
```

---

## Configuration Example

```yaml
# config/default.yaml

general:
  log_level: info
  cache_ttl_hours: 24
  cost_budget_daily_usd: 2.00     # Hard cap on cloud spending

session:
  auto_resume: true                # Resume last session when starting in same project dir
  session_ttl_hours: 72            # Archive sessions after this much inactivity
  max_turns_before_compaction: 200 # Force-summarize if session gets very long
  
  sliding_window:
    tier1_system_tokens: 400
    tier2_summary_max_tokens: 1500
    tier3_recent_max_tokens: 6000
    tier4_code_max_tokens: 6000
    tier5_prompt_max_tokens: 2800
    latency_pressure_reduction: 0.6  # Cut budgets to 60% when near patience threshold
    
  importance:
    mode: "heuristic"              # "heuristic" or "ml" (after Phase 2 training)
    evict_below_score: 0.3         # Turns below this score are evicted first
    summarize_batch_size: 5        # Compress this many turns into one summary
    
  intervention_summary:
    enabled: true
    max_tokens: 400                # Cap on catch-up narrative length
    trigger_after_local_turns: 3   # Build summary when 3+ local turns precede cloud call
    
  thread_tracking:
    keep_alive_timeout_min: 30     # Consider backend thread dead after 30min silence
    prefer_thread_reuse: true      # Prefer backends with live threads (latency savings)
    thread_reuse_latency_bonus_ms: 1500  # Estimated savings from thread reuse

  persistence:
    db_path: "./data/sessions.db"
    export_format: "markdown"      # For /session export

local_model:
  primary: "qwen2.5-coder-7b-q4"
  fast: "codegemma-2b-q4"         # For sub-1s tasks
  fallback: "phi3-mini-q4"
  inference_engine: "ollama"       # or "llamacpp", "vllm"
  max_tokens: 4096
  temperature: 0.2
  gpu_layers: 35                   # Adjust for your VRAM

latency:
  predictor_mode: "heuristic"      # "heuristic" → "ml" once enough data
  ml_model_path: "./data/latency_model.bin"
  calibration_on_startup: true     # Run benchmarks at first launch
  recalibrate_interval_hours: 168  # Weekly recalibration
  system_monitor_interval_ms: 1000 # Sample GPU/CPU/RAM every second
  network_probe_interval_s: 60     # Ping cloud endpoints every minute
  prediction_confidence_threshold: 0.60  # Below this, trigger race mode
  
  # Output token estimation weights
  output_estimation:
    intent_prior_weight: 0.30
    input_ratio_weight: 0.25
    similar_prompt_weight: 0.35
    verbosity_cue_weight: 0.10

patience:
  profile: "default"               # "default", "impatient", "patient", or "auto"
  auto_learn: true                 # Adjust from observed behavior
  instant_threshold_ms: 2000
  acceptable_threshold_ms: 5000
  tolerable_threshold_ms: 15000
  abandon_threshold_ms: 30000
  streaming_patience_multiplier: 1.8
  ttft_critical_ms: 3000

routing:
  weights:
    latency: 0.40
    quality: 0.30
    cost: 0.15
    reliability: 0.15
  force_cloud_intents:
    - security_audit
    - architecture_design
  force_local_intents:
    - explain_code
    - format_code
    - write_docstring
    - git_commit_message
  race_mode_confidence_threshold: 0.60  # If latency confidence < this, race both

cloud_backends:
  preferred_order: [gemini_cli, claude_code, cursor_agent]
  gemini_cli:
    enabled: true
    command: "gemini"
    max_tokens: 8192
    timeout_ms: 20000
  claude_code:
    enabled: true
    command: "claude"
    max_tokens: 8192
    timeout_ms: 30000
  cursor_agent:
    enabled: false
    command: "cursor"
    max_tokens: 8192
    timeout_ms: 45000

compression:
  enabled: true
  strip_imports: true
  summarize_threshold_lines: 100
  max_context_tokens: 4000
  rewrite_prompts: true
  latency_aware: true              # Compress harder when near patience threshold
  aggressive_compression_headroom_ms: 1000  # If predicted latency is within 1s of threshold, go aggressive

feedback:
  enabled: true
  db_path: "./data/feedback.db"
  retrain_interval_days: 7
  min_samples_for_ml: 200          # Switch from heuristic to ML after this many observations
  drift_alert_mae_threshold: 0.40  # Alert if MAE > 40% for 3 days
```

---

## Key Design Principles

1. **Latency is the user experience.** Every architectural decision is evaluated through the lens of "how does this affect what the developer feels while waiting?" Predicted latency is the primary routing signal, not an afterthought.

2. **Session continuity is sacred.** The developer should never have to re-explain context. Whether a prompt is handled locally or by cloud, the response must reflect everything that happened earlier in the session. If the cloud model gives gibberish because it lacked context, that's a LOKAHI bug, not a user error.

3. **Local-first, cloud-smart.** Every prompt starts local. Cloud is earned, not default.

4. **The best token is the one you never send.** Compression isn't optional — it's the core value proposition. Session context is compressed, not omitted. Every cloud call should carry exactly enough history to be coherent, and not one token more.

5. **Streaming hides latency.** The hybrid route's superpower is that the user sees output in <200ms regardless of total wall-clock time. Design every UX path to minimize time-to-first-useful-output.

6. **Predict, measure, learn.** Every latency prediction is compared to reality. Every context payload's coherence is measured by whether the cloud response made sense. The system self-corrects weekly.

7. **Transparent routing.** The developer should always be able to see *why* a prompt was routed where it was, and *what context* was sent. `--explain` shows: predicted latencies, patience thresholds, scoring breakdown, and context payload preview.

8. **Graceful degradation.** If cloud is down or budget is exhausted, local handles everything with full session context from KV cache. If local model is overloaded, queue and warn.

9. **Privacy by default.** All session data stays local in SQLite. Cloud backends receive only the compressed, necessary context. No telemetry or conversation history leaves the machine unless explicitly opted in.

10. **Composable backends.** Adding a new cloud backend requires only implementing a thin adapter + providing a latency profile + supporting the context payload interface. The Session Manager handles the rest.

---

## Metrics to Track

| Metric | Target | Why it matters |
|---|---|---|
| Latency prediction MAE | <25% | Core predictor quality — everything depends on this |
| Latency prediction calibration | p50 within ±20% | Confidence intervals must be trustworthy |
| Local resolution rate | >65% | Core efficiency measure |
| Cloud token savings (vs. raw) | >50% | Compression effectiveness |
| **Context coherence rate** | **>95%** | **Cloud responses that correctly reference session history** |
| **Thread reuse rate** | **>60%** | **How often we avoid cold-start context payloads** |
| **Context tokens saved by thread reuse** | **>40%** | **Delta optimization effectiveness** |
| **Avg session depth at cloud call** | **monitor** | **How much history the system must manage** |
| User acceptance rate (local) | >80% | Quality of local routing |
| User acceptance rate (hybrid) | >90% | Hybrid pipeline effectiveness |
| P50 user-perceived-latency (all routes) | <2s | This is the number that matters for UX |
| P50 time-to-first-token (all routes) | <500ms | User should always see something fast |
| P95 total wall-clock (all routes) | <15s | Tail latency kills trust |
| User cancel/abandon rate | <5% | The ultimate UX failure metric |
| **"Re-explain" rate** | **<3%** | **How often users have to re-state context the system should know** |
| Daily cloud cost | <$2 | Budget discipline |
| Routing accuracy (F1) | >0.85 | Classifier quality |
| Patience threshold accuracy | ±15% of observed | User model is well-calibrated |

---

## Prompt Templates

### Local Self-Critique Prompt
```
You are reviewing code generated by a junior developer. Be strict but fair.

TASK: {{original_task}}
GENERATED CODE:
{{draft_code}}

Score each dimension 1-5:
- Correctness: Does it solve the stated task?
- Completeness: Are edge cases handled?
- Style: Does it follow the project's conventions?
- Security: Any obvious vulnerabilities?

OVERALL: Would you ship this? (YES / NEEDS_REFINEMENT / NO)
If NEEDS_REFINEMENT or NO, list the specific issues to fix.
Respond in JSON only.
```

### Cloud Escalation Prompt Template
```
CONTEXT: A local code assistant produced a draft that needs refinement.
TASK: {{compressed_task_description}}
DRAFT (with issues flagged):
{{draft_with_annotations}}
FLAGGED ISSUES:
{{self_critique_issues}}
INSTRUCTIONS: Fix only the flagged issues. Return only the corrected code sections. Do not re-explain the task or add commentary.
```

### Prompt Rewriter Template (Local model rewrites user prompts for cloud)
```
Rewrite this developer request into an optimal prompt for a code-generation AI.
Rules:
- Be specific and unambiguous
- Include only necessary context
- Specify output format
- Remove conversational filler
- Keep under {{token_budget}} tokens

ORIGINAL: {{raw_user_prompt}}
RELEVANT CODE CONTEXT: {{compressed_context}}
REWRITTEN:
```

### Latency Explanation Template (for --explain flag)
```
ROUTING DECISION for: "{{user_prompt_truncated}}"

Predicted latencies:
  LOCAL:          {{local_p50}}ms (p95: {{local_p95}}ms) — confidence {{local_conf}}
  HYBRID:         {{hybrid_p50}}ms (p95: {{hybrid_p95}}ms) — TTFT: {{hybrid_ttft}}ms
  CLOUD (gemini):  {{gemini_p50}}ms (p95: {{gemini_p95}}ms) — TTFT: {{gemini_ttft}}ms  
  CLOUD (claude):  {{claude_p50}}ms (p95: {{claude_p95}}ms) — TTFT: {{claude_ttft}}ms

Your patience profile: {{patience_profile_name}}
  Instant: <{{instant_ms}}ms | Acceptable: <{{acceptable_ms}}ms | Tolerable: <{{tolerable_ms}}ms

Route scores:
  LOCAL:   latency={{l_lat}} quality={{l_qual}} cost={{l_cost}} reliability={{l_rel}} → TOTAL={{l_total}}
  HYBRID:  latency={{h_lat}} quality={{h_qual}} cost={{h_cost}} reliability={{h_rel}} → TOTAL={{h_total}}
  CLOUD:   latency={{c_lat}} quality={{c_qual}} cost={{c_cost}} reliability={{c_rel}} → TOTAL={{c_total}}

DECISION: {{chosen_route}} ({{chosen_backend}})
REASON: {{human_readable_reason}}
```
