# Testing LOKAHI

## Quick Test
```bash
docker-compose up -d
docker exec lokahi_app timeout 30 ./target/release/lokahi <<< "explain the router module"
```

Expected: Routes to CLOUD:claude and shows Claude response

## Test Scenarios

### Simple Query (Cloud)
```bash
docker exec lokahi_app timeout 30 ./target/release/lokahi <<< "what is 2+2?"
```
Expected: Routes to CLOUD:claude, responds "4"

### Complex Query (Cloud)
```bash
docker exec lokahi_app timeout 60 ./target/release/lokahi <<< "explain the session management architecture in detail"
```
Expected: Multi-paragraph detailed response from Claude

### Multiple Queries
```bash
docker exec lokahi_app timeout 60 ./target/release/lokahi <<'EOF'
what is 2+2?
explain the router briefly
/quit
EOF
```

## Build & Test
```bash
# Full test suite
cargo test
# Expected: 16/16 pass

# Build release
cargo build --release

# Docker
docker-compose up -d --build
docker exec lokahi_app ./target/release/lokahi <<< "test query"
```

## Verify Backends

```bash
# Claude
docker exec lokahi_app bash -c "echo 'Explain 2+2' | claude -p"

# Gemini
docker exec lokahi_app bash -c "echo 'Explain 2+2' | gemini"

# Ollama
docker exec lokahi_app bash -c "curl -X POST http://ollama:11434/api/generate -d '{\"model\":\"tinyllama\",\"prompt\":\"2+2\"}' | jq '.response'"
```

## Debugging

If Claude fails ("Not logged in"):
```bash
# Check HOME env
docker exec lokahi_app env | grep HOME
# Should show: HOME=/home/rickeshtn

# Check credentials mounted
docker exec lokahi_app ls -la /home/rickeshtn/.claude
# Should show: .credentials.json exists
```

If timeouts occur:
- Increase timeout value (queries can take 5-30s)
- Check logs: `docker-compose logs lokahi_app`
- Verify status: `docker-compose ps`

## Key Facts

- Simple queries: 2-3 seconds
- Complex queries: 10-30 seconds
- Routing decision shown immediately
- Streaming responses (line-by-line)
- Session persisted to SQLite
- Auto-fallback to Ollama if Claude unavailable
