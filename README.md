# 100minds

```
    ╔══════════════════════════════════════════════════════════╗
    ║   ██╗ ██████╗  ██████╗ ███╗   ███╗██╗███╗   ██╗██████╗  ║
    ║  ███║██╔═████╗██╔═████╗████╗ ████║██║████╗  ██║██╔══██╗ ║
    ║  ╚██║██║██╔██║██║██╔██║██╔████╔██║██║██╔██╗ ██║██║  ██║ ║
    ║   ██║████╔╝██║████╔╝██║██║╚██╔╝██║██║██║╚██╗██║██║  ██║ ║
    ║   ██║╚██████╔╝╚██████╔╝██║ ╚═╝ ██║██║██║ ╚████║██████╔╝ ║
    ║   ╚═╝ ╚═════╝  ╚═════╝ ╚═╝     ╚═╝╚═╝╚═╝  ╚═══╝╚═════╝  ║
    ╚══════════════════════════════════════════════════════════╝
```

**Adversarial Decision Intelligence for AI Agents**

100minds channels 100 legendary thinkers—from Knuth to Schneier, Deming to Hinton—into an adversarial council that challenges decisions before they fail in production. Used in production by [Zesty](https://github.com/zeststream/swarm-daemon) to guide autonomous coding agents.

[![CI](https://github.com/JYeswak/100minds-mcp/actions/workflows/ci.yml/badge.svg)](https://github.com/JYeswak/100minds-mcp/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.75+-orange.svg)](https://www.rust-lang.org/)
[![Thinkers](https://img.shields.io/badge/thinkers-100-green.svg)](data/thinkers/)
[![Principles](https://img.shields.io/badge/principles-354-blue.svg)](data/thinkers/)

## What Makes This Different

Most "mental models" tools give you generic advice. 100minds is different:

| Feature | 100minds | Generic Tools |
|---------|----------|---------------|
| **Adversarial debate** | FOR/AGAINST/CHALLENGE positions | Single answer |
| **Falsification criteria** | "This is wrong if..." for every position | None |
| **Learns from outcomes** | Thompson Sampling on your results | No memory |
| **Production-tested** | 100k+ decisions in autonomous swarms | Demo only |
| **Cryptographic audit** | Ed25519 signatures, hash chains | None |

## Quick Start

```bash
# Install
cargo install --git https://github.com/JYeswak/100minds-mcp.git

# Get counsel
100minds counsel "Should we rewrite the legacy system?"

# Run as HTTP server (for swarm integration)
100minds --serve --port=3100
```

**Output:**
```
┌─────────────────────────────────────────────────────────────┐
│ 🧠 100MINDS DECISION INTELLIGENCE                           │
└─────────────────────────────────────────────────────────────┘

📋 Should we rewrite the legacy system?

┌─ FOR ───────────────────────────────────────────────────────
│  Kent Beck [confidence: 0.72]
│    "Do the simplest thing that could possibly work."
│    ⚠️ Falsifiable if: Simple solution can't meet requirements
│
├─ AGAINST ───────────────────────────────────────────────────
│  Fred Brooks [confidence: 0.68]
│    "Adding more engineers to a late project makes it later."
│    ⚠️ Falsifiable if: Small, independent tasks can parallelize
│
└─ CHALLENGE ─────────────────────────────────────────────────
   Devil's Advocate [confidence: 0.95]
   🔍 Missing: rollback plan, team capacity, timeline constraints
```

## Production Integration (Zesty Swarm)

100minds powers the decision layer for [Zesty](https://github.com/zeststream/swarm-daemon), an autonomous coding swarm. The feedback loop:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           FACTORIO FEEDBACK LOOP                            │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│   ┌──────────────┐     ┌──────────────┐     ┌──────────────┐              │
│   │   Worker     │     │   100minds   │     │   Thompson   │              │
│   │   Spawns     │────▶│   Counsel    │────▶│   Posteriors │              │
│   │              │     │              │     │   (α, β, ρ)  │              │
│   └──────────────┘     └──────────────┘     └──────────────┘              │
│          │                    │                    ▲                       │
│          │                    │ decision_id        │                       │
│          ▼                    │ principle_ids      │                       │
│   ┌──────────────┐           │                    │                       │
│   │   Worker     │           │                    │                       │
│   │   Executes   │           │                    │                       │
│   │   Task       │           │                    │                       │
│   └──────────────┘           │                    │                       │
│          │                   │                    │                       │
│          │ success/failure   │                    │                       │
│          ▼                   ▼                    │                       │
│   ┌──────────────┐     ┌──────────────┐          │                       │
│   │   Daemon     │────▶│   Record     │──────────┘                       │
│   │   Records    │     │   Outcome    │   Updates posteriors:            │
│   │   Outcome    │     │              │   success → α += 0.05            │
│   └──────────────┘     └──────────────┘   failure → β += 0.10            │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Integration code (Rust):**

```rust
use minds_mcp::{CounselEngine, CounselRequest};

// Pre-work: Get guidance before task
let engine = CounselEngine::new()?;
let counsel = engine.counsel(&CounselRequest {
    question: format!("Worker starting on task: {}", task_description),
    domain: Some("software-development".to_string()),
    decision_id: Some(format!("bead-{}", bead_id)),  // Links outcome back
    ..Default::default()
})?;

// Inject counsel into worker context
let guidance = format_counsel_for_worker(&counsel);

// ... worker executes task ...

// Post-work: Record outcome for learning
engine.record_outcome(&RecordOutcomeRequest {
    decision_id: format!("bead-{}", bead_id),
    success: task_succeeded,
    principle_ids: counsel.principle_ids(),
    confidence_score: Some(worker_confidence),
    ..Default::default()
})?;
```

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                            YOUR DECISION                                     │
│                  "Should we rewrite the legacy system?"                      │
└──────────────────────────────┬──────────────────────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                         100MINDS ENGINE                                      │
│  ┌───────────────┐  ┌───────────────┐  ┌───────────────┐  ┌─────────────┐  │
│  │  FTS5 Search  │  │   Neural      │  │   Template    │  │  Thompson   │  │
│  │  + Keywords   │  │   Posterior   │  │   Detection   │  │  Sampling   │  │
│  │  (SQLite)     │  │   (ONNX)      │  │  (12 types)   │  │  (α/β/ρ)    │  │
│  └───────────────┘  └───────────────┘  └───────────────┘  └─────────────┘  │
└──────────────────────────────┬──────────────────────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                 100 THINKERS  │  354 PRINCIPLES  │  6 DOMAINS               │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────┐ │
│  │ Software │ │ Systems  │ │ Business │ │ Decision │ │Philosophy│ │Secur-│ │
│  │    20    │ │    15    │ │    20    │ │    15    │ │    15    │ │ity 15│ │
│  │  Knuth   │ │  Deming  │ │  Drucker │ │  Hinton  │ │  Dennett │ │Schnei│ │
│  │  Fowler  │ │  Ohno    │ │  Thiel   │ │  LeCun   │ │  Bostrom │ │  er  │ │
│  │  Brooks  │ │  Senge   │ │  Graham  │ │  Pearl   │ │Hofstadter│ │Mitnick│
│  └──────────┘ └──────────┘ └──────────┘ └──────────┘ └──────────┘ └──────┘ │
└──────────────────────────────┬──────────────────────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│  ✅ FOR (confidence)  │  ⚠️ AGAINST (confidence)  │  🔍 CHALLENGE           │
│  + Falsification      │  + Falsification          │  + Missing              │
│    criteria           │    criteria               │    considerations       │
└──────────────────────────────┬──────────────────────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│              Ed25519 Signature  │  SHA-256 Hash Chain  │  Audit Trail       │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Neural Posterior (ONNX)

The neural posterior replaces simple Beta distributions with an ONNX model trained on 40k synthetic decision/outcome pairs:

- **MLP encoder** with self-attention on context features
- **64-dim embeddings** for principles and thinkers
- **Outputs**: success probability + epistemic uncertainty
- **UCB-style exploration**: `score = success_prob + exploration_weight * uncertainty`

This allows principle selection to consider context (domain, difficulty, urgency) rather than treating all decisions equally.

## MCP API Reference

100minds exposes 15 tools via JSON-RPC over HTTP:

### Core Tools

| Tool | Description |
|------|-------------|
| `counsel` | Get adversarial wisdom council on a decision. Returns FOR/AGAINST/CHALLENGE positions with falsification criteria. |
| `record_outcome` | Record success/failure for learning. Updates Thompson posteriors. **Critical for the feedback loop.** |
| `pre_work_context` | Get relevant frameworks BEFORE starting work. Use at task start. |

### Discovery Tools

| Tool | Description |
|------|-------------|
| `search_principles` | FTS5 full-text search across 354 principles |
| `get_decision_template` | Guided decision tree for 12 common decisions (monolith-vs-microservices, build-vs-buy, etc.) |
| `get_synergies` | Find principles that work well together |
| `get_tensions` | Find conflicting principles—you must choose |
| `check_blind_spots` | Identify what you might be missing |
| `detect_anti_patterns` | Check for known bad patterns |

### Learning Tools

| Tool | Description |
|------|-------------|
| `sync_posteriors` | Get Thompson Sampling α/β/ρ for all principles. Used by swarms to sync learning. |
| `record_outcomes_batch` | Bulk outcome recording for daemon restart recovery |
| `counterfactual_sim` | "What if we hadn't used these principles?" simulation |
| `wisdom_stats` | Statistics on principle track records |

### Validation Tools

| Tool | Description |
|------|-------------|
| `validate_prd` | Check PRDs against philosophical frameworks. Catches Brooks's Law violations, YAGNI issues, etc. |
| `audit_decision` | Full provenance chain with Ed25519 signatures |

### Example: Full JSON-RPC Call

```bash
curl -X POST http://localhost:3100/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "tools/call",
    "params": {
      "name": "counsel",
      "arguments": {
        "question": "Should we add caching to the API?",
        "domain": "architecture",
        "decision_id": "bead-josh-abc123"
      }
    }
  }'
```

## CLI Usage

```bash
# Get adversarial counsel
100minds counsel "Should we use microservices?"

# With JSON output for automation
100minds counsel "Should we add caching?" --json

# Specify domain for better matching
100minds counsel "Should we use Redis?" --domain=performance

# Record outcome (closes learning loop)
100minds --outcome <decision-id> --success

# View statistics
100minds --stats

# Run as HTTP server
100minds --serve --port=3100

# Validate a PRD
100minds --validate-prd path/to/prd.json

# Analyze PRD with 100minds metadata
100minds --analyze-prd path/to/prd.json

# Thompson Sampling stats
100minds --thompson stats

# Run benchmarks
100minds --benchmark scenarios
100minds --analyze coverage
```

## Installation

### Option 1: Cargo (Recommended)

```bash
cargo install --git https://github.com/JYeswak/100minds-mcp.git
100minds --stats
```

### Option 2: Docker

```bash
docker run -p 3100:3100 ghcr.io/jyeswak/100minds-mcp:latest
```

### Option 3: From Source

```bash
git clone https://github.com/JYeswak/100minds-mcp.git
cd 100minds-mcp
cargo build --release
./target/release/100minds --stats
```

### ONNX Runtime (for semantic search)

```bash
# macOS
brew install onnxruntime

# Linux
apt install libonnxruntime-dev
```

## The 100 Thinkers

| Domain | Count | Legends |
|--------|-------|---------|
| **Software** | 20 | Knuth, Fowler, Brooks, Beck, Hopper, Carmack, Dijkstra, Lamport |
| **Systems** | 15 | Deming, Ohno, Senge, Weinberg, Goldratt, Forrester, Ackoff |
| **Business** | 20 | Drucker, Thiel, Graham, Godin, Christensen, Porter, Blank |
| **Decision-Making** | 15 | Hinton, LeCun, Ng, Pearl, Sutton, Bengio, Goodfellow |
| **Philosophy** | 15 | Dennett, Hofstadter, Bostrom, Tegmark, Russell, Searle |
| **Security** | 15 | Schneier, Mitnick, Stamos, Tabriz, Ormandy, McGraw, Ranum |

## Limitations

- **Not a replacement for domain experts** — Provides frameworks, not authoritative answers
- **No real-time data** — Principles are timeless wisdom, not current events
- **English only** — Thinker content is currently in English
- **Requires feedback** — Learning loop needs recorded outcomes to improve
- **Curated, not comprehensive** — 100 thinkers can't cover every domain

## Development

```bash
cargo test                              # Run tests
cargo run --bin import -- data/thinkers # Import thinkers
cargo run --bin 100minds -- --stats     # Check stats
cargo run --bin 100minds -- --benchmark scenarios  # Run benchmarks
```

## Contributing

Contributions welcome:
- **New thinkers**: Add wisdom from underrepresented domains
- **Better templates**: Improve decision matching
- **Evaluation scenarios**: Expand benchmark coverage
- **Integrations**: Connect to more AI frameworks

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

MIT License - see [LICENSE](LICENSE) for details.

---

**"The purpose of abstraction is not to be vague, but to create a new semantic level in which one can be absolutely precise."** — Edsger Dijkstra

Built with care by [ZestStream](https://zeststream.ai)
