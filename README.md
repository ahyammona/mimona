# Mimona

> Run AI models locally. Pay per query with crypto. Earn SOL as a node provider.

```
mimona pull qwen2.5-coder:7b
mimona run qwen2.5-coder:7b
```

---

## What is Mimona?

Mimona is an open-source, Rust-native alternative to Ollama with blockchain payments built in.

- **Free models** — run locally, zero cost, no account needed
- **Paid models** — pay per query in SOL, only what you use
- **Node providers** — host models, earn SOL automatically
- **OpenAI-compatible API** — works with Open WebUI, Continue.dev, etc.

---

## Install

```bash
curl -fsSL https://mimona.io/install.sh | sh
```

Or build from source:

```bash
cargo install mimona
```

---

## Quick Start

```bash
# Download a model
mimona pull qwen2.5-coder:7b

# Chat interactively
mimona run qwen2.5-coder:7b

# Start API server (OpenAI-compatible on port 11435)
mimona serve

# See all downloaded models
mimona list
```

---

## Commands

| Command | Description |
|---|---|
| `mimona pull <model>` | Download a model |
| `mimona run <model>` | Run a model interactively |
| `mimona list` | List downloaded models |
| `mimona show <model>` | Show model details |
| `mimona rm <model>` | Remove a model |
| `mimona serve` | Start OpenAI-compatible API server |
| `mimona node start` | Become a node provider |
| `mimona node earnings` | View your earnings |
| `mimona wallet create` | Create a Solana wallet |
| `mimona wallet balance` | Check your SOL balance |

---

## Available Models

| Model | Size | RAM | Tier |
|---|---|---|---|
| tinyllama:1b | 0.7 GB | 2 GB | Free |
| qwen2.5-coder:3b | 2.0 GB | 4 GB | Free |
| qwen2.5-coder:7b | 4.7 GB | 7 GB | Free |
| qwen2.5-coder:14b | 9.0 GB | 14 GB | Free |
| llama3:8b | 4.9 GB | 8 GB | Free |
| llama3:70b | 42 GB | 48 GB | 0.002 SOL |
| mistral:7b | 4.4 GB | 6 GB | Free |
| phi3:mini | 2.2 GB | 4 GB | Free |
| deepseek-coder:6.7b | 4.1 GB | 6 GB | Free |
| deepseek-coder:33b | 20 GB | 24 GB | 0.001 SOL |

---

## Node Provider

Your machine qualifies as a node if it has 8+ GB RAM.

```bash
# Create wallet first
mimona wallet create

# Start earning SOL
mimona node start
```

Requests are automatically routed to your node. You earn SOL per query served.

---

## API

Mimona exposes an OpenAI-compatible API on `http://localhost:11435`.

```bash
# Chat completion
curl http://localhost:11435/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "qwen2.5-coder:7b",
    "messages": [{"role": "user", "content": "Hello!"}]
  }'

# List models
curl http://localhost:11435/v1/models
```

Works as a drop-in for **Open WebUI**, **Continue.dev**, **LiteLLM**, and any OpenAI-compatible tool.
Set the base URL to `http://localhost:11435`.

---

## Project Structure

```
mimona/
├── src/
│   ├── main.rs          # CLI entry point
│   ├── cli/             # All CLI commands
│   ├── inference/       # GGUF inference engine (llama.cpp)
│   ├── models/          # Registry, downloader, storage
│   ├── server/          # OpenAI-compatible REST API
│   ├── payment/         # Wallet + Solana payments
│   └── node/            # Node provider mode
├── registry.json        # Bundled model registry
├── frontend/            # Local web UI
└── install.sh           # One-line installer
```

---

## Build from Source

```bash
git clone https://github.com/yourusername/mimona
cd mimona

# Standard build (with inference engine)
cargo build --release

# Run
./target/release/mimona --help
```

**Requirements:**
- Rust 1.75+
- CMake (for llama.cpp compilation)
- 8 GB RAM minimum

---

## Roadmap

- [x] CLI skeleton (pull, run, list, serve, node, wallet)
- [x] HuggingFace model downloads with progress
- [x] OpenAI-compatible API server
- [x] Solana wallet management
- [ ] Native GGUF inference (llama.cpp integration)
- [ ] P2P model downloads from node network
- [ ] Real Solana transaction signing
- [ ] Node provider dashboard
- [ ] GPU support (CUDA, Metal)
- [ ] Vision models (multimodal)
- [ ] Model fine-tuning support

---

## License

MIT
