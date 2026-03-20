# Running LOKAHI

## Quick Start (Recommended)

```bash
cd /home/rickeshtn/Projects/lokahi

# Start both Ollama and LOKAHI with interactive input
docker-compose run --rm lokahi

# The prompt will appear:
# 👤 You: (type your prompt here)
```

The interactive shell will launch. You'll see:
```
✅ Ollama is healthy
🚀 LOKAHI - Intelligent Prompt Router
=====================================
Connected to: http://ollama:11434
Model: tinyllama
Session: [session-id]

Commands:
  /quit      - Exit
  /history   - Show conversation history
  /clear     - Clear history
  /status    - Show system status

👤 You: (waiting for input)
```

## Stopping

Press `/quit` in the prompt or `Ctrl+C` to exit LOKAHI. Ollama will continue running.

To stop Ollama completely:
```bash
docker-compose down
```

---

## What Happens on Startup

1. **Ollama container starts** on port 11435 (host) → 11434 (container)
   - Pulls the `ollama/ollama:latest` image
   - Listens on all interfaces
   - Persists models in Docker volume `ollama_data`

2. **LOKAHI container starts** and connects to Ollama
   - Mounts project directory at `/work`
   - Detects Ollama is healthy via HTTP
   - Pulls `mistral` model automatically
   - Launches TUI shell
   - Persists sessions in Docker volume `lokahi_sessions`

---

## Architecture

```
Host                          Docker
├── :11435 (localhost)    →   Ollama:11434
│
└── /home/rickeshtn/Projects/lokahi
    ├── /work (mounted)   →   LOKAHI app
    └── sessions.db       ←   Persisted in volume
```

## Volumes

- `ollama_data` - Stores downloaded Ollama models (persistent)
- `lokahi_sessions` - Stores LOKAHI session databases (persistent)

These persist across container restarts.

---

## Troubleshooting

### Port 11434/11435 already in use

If you get `address already in use`:
```bash
docker-compose down
# Kill any lingering processes on 11435:
lsof -i :11435 | grep LISTEN | awk '{print $2}' | xargs kill -9
docker-compose up
```

### Interactive input not working

Make sure you're running:
```bash
docker-compose run --rm lokahi  # NOT docker-compose up
```

The `docker-compose run` command properly allocates stdin/stdout/TTY for interactive input.
Using `docker-compose up` does not forward stdin correctly.

### Ollama model not pulling

Check logs:
```bash
docker-compose logs ollama | tail -20
```

The mistral model will auto-pull on first startup (several GB download).

---

## First Commands to Try

Once TUI is running, type:

```
explain how the router module works
```
Press **Enter** → LOKAHI routes to LOCAL (Ollama) → streams response

```
security audit src/main.rs
```
Press **Enter** → LOKAHI routes to CLOUD (claude/gemini) → escalates

---

## File Structure

```
/home/rickeshtn/Projects/lokahi/
├── docker-compose.yml        ← Orchestrates Ollama + LOKAHI
├── Dockerfile.lokahi         ← LOKAHI runtime image
├── target/release/lokahi     ← Pre-built binary (mounted)
├── src/                       ← Source code (mounted)
├── sessions.db               ← Created on first run
└── data/                      ← Session volumes mounted here
```

---

## Stopping & Cleanup

**Graceful stop:**
```bash
docker-compose down
```

**Force stop (if hung):**
```bash
docker-compose kill
```

**Remove volumes (delete all data):**
```bash
docker-compose down -v
```

---

## Production Notes

- Port 11435 is exposed on `0.0.0.0` (all interfaces)
- To restrict to localhost only, edit docker-compose.yml:
  ```yaml
  ports:
    - "127.0.0.1:11435:11434"  # Only localhost
  ```

- Database is in `/work/data/` (mounted from host)
- All data persists in Docker volumes
- Rebuild binary with: `cargo build --release` (before docker-compose up)

---

**Status**: ✅ Ready to run
**Date**: 2026-03-20
