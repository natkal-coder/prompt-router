# Installation Guide

Choose your preferred installation method.

## 1. Quick Install (Recommended)

```bash
bash <(curl -fsSL https://raw.githubusercontent.com/rickeshtn/lokahi/main/install.sh)
```

This will:
- Check Docker/Docker Compose
- Download pre-built binary (or build from source if needed)
- Install to `~/.local/bin`
- Create Docker wrapper script
- Set up PATH

Then run:
```bash
lokahi-run ~/Projects/my-project
```

## 2. NPM Install

For Node.js users:

```bash
npm install -g lokahi-cli
```

Usage:
```bash
lokahi       # Uses Docker automatically
```

## 3. Cargo Install (from crates.io)

For Rust developers:

```bash
cargo install lokahi
```

Binary goes to: `~/.cargo/bin/lokahi`

## 4. From Source (GitHub)

Clone and build:

```bash
git clone https://github.com/rickeshtn/lokahi.git
cd lokahi
cargo build --release
./target/release/lokahi
```

Or install directly:

```bash
cargo install --git https://github.com/rickeshtn/lokahi
```

## 5. Docker (No Installation)

Run directly without installing:

```bash
docker-compose run --rm lokahi
```

Requires `docker-compose.yml` in your project.

## System Requirements

### For All Methods
- **Docker** (for running Ollama)
- **Docker Compose** (for orchestration)

### For Source Installation
- **Rust 1.70+** ([install](https://rustup.rs/))

### For NPM Installation
- **Node.js 14+** (just for CLI wrapper)

## Verification

Verify installation:

```bash
lokahi --version
# or
lokahi-run --version
```

Try it:

```bash
cd ~/Projects/lokahi
lokahi-run .
```

Type a prompt:
```
❯ You: what's in this project?
```

## Troubleshooting

### Docker not found
```bash
# Install Docker Desktop from https://docker.com
# Then check:
docker --version
docker-compose --version
```

### Port 11434 in use
```bash
docker-compose down
lsof -i :11434 | grep LISTEN | awk '{print $2}' | xargs kill -9
lokahi-run .
```

### Binary fails to run
```bash
# Use Docker instead:
docker-compose run --rm lokahi

# Or build from source:
cargo install --git https://github.com/rickeshtn/lokahi
```

### Arrow keys showing escape codes
```bash
# Use docker-compose run instead of up:
cd ~/Projects/my-project
docker-compose run --rm lokahi
```

## Uninstall

### From Quick Install
```bash
rm ~/.local/bin/lokahi
rm ~/.local/bin/lokahi-run
```

### From NPM
```bash
npm uninstall -g lokahi-cli
```

### From Cargo
```bash
cargo uninstall lokahi
```

## Updating

### Quick Install
```bash
bash <(curl -fsSL https://raw.githubusercontent.com/rickeshtn/lokahi/main/install.sh)
```

### NPM
```bash
npm update -g lokahi-cli
```

### Cargo
```bash
cargo install --force lokahi
```

## Getting Help

- **GitHub Issues**: https://github.com/rickeshtn/lokahi/issues
- **Documentation**: https://github.com/rickeshtn/lokahi#readme
- **Discord**: (coming soon)

---

**Happy prompt routing!** 🚀
