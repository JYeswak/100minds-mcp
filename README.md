# 100minds

**Adversarial Decision Intelligence for AI Agents**

100minds channels 70 legendary thinkers—from Dijkstra to Taleb, Feynman to Brooks—into an adversarial council that challenges your decisions before they fail in production.

[![CI](https://github.com/zeststream/100minds-mcp/actions/workflows/ci.yml/badge.svg)](https://github.com/zeststream/100minds-mcp/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/minds-mcp.svg)](https://crates.io/crates/minds-mcp)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.75+-orange.svg)](https://www.rust-lang.org/)

## One-Line Install

```bash
cargo install --git https://github.com/zeststream/100minds-mcp.git
```

Or from crates.io (when published):
```bash
cargo install minds-mcp
```

## The Problem

AI agents make thousands of decisions. Most fail silently. By the time you notice, the damage is done.

**Traditional approach:** Hope for the best, debug after failure.

**100minds approach:** Every decision faces adversarial scrutiny from the world's greatest minds *before* execution.

## How It Works

```
Your Question: "Should we rewrite the authentication system?"

100minds Response:
┌─────────────────────────────────────────────────────────────┐
│ FOR: Martin Fowler (Strangler Fig Pattern)                  │
│   "Incremental replacement reduces risk. Start with the     │
│    highest-value, lowest-risk component."                   │
│   Confidence: 0.82                                          │
├─────────────────────────────────────────────────────────────┤
│ AGAINST: Fred Brooks (Second System Effect)                 │
│   "The second system is the most dangerous. Tendency to     │
│    over-engineer what was learned from the first."          │
│   Confidence: 0.78                                          │
├─────────────────────────────────────────────────────────────┤
│ CHALLENGE: Nassim Taleb (Antifragility)                     │
│   "Missing considerations: What breaks if this fails?       │
│    Where are the hidden dependencies? Have you tested       │
│    failure scenarios?"                                      │
└─────────────────────────────────────────────────────────────┘

Falsifiable if: Rewrite takes >2x estimated time OR
                introduces >3 P0 bugs in first month
```

## Features

### Adversarial Wisdom Council
- **70 thinkers, 345 principles** across software architecture, systems thinking, entrepreneurship, and more
- **FOR/AGAINST/CHALLENGE** positions force genuine consideration of tradeoffs
- **Falsification criteria** make advice testable (Popper would approve)

### Thompson Sampling Learning
- **Asymmetric adjustments**: Failures hurt more than successes help (+0.05/-0.10)
- **Domain-specific learning**: "YAGNI works for architecture, not security"
- **Feel-Good Thompson Sampling**: Optimism bonus prevents cold-start paralysis

### Cryptographic Provenance
- **Ed25519 signatures** on every decision
- **SHA-256 hash chain** links decisions for audit trails
- **Tamper detection** built-in

### MCP Server Protocol
- **JSON-RPC interface** for AI agent integration
- **Real-time counsel** during task execution
- **Outcome recording** closes the learning loop

## Quick Start

### Installation

```bash
# One-line install
cargo install --git https://github.com/zeststream/100minds-mcp.git

# Or clone and build
git clone https://github.com/zeststream/100minds-mcp.git
cd 100minds-mcp
cargo build --release
```

### CLI Usage

```bash
# Get counsel on a decision
cargo run --bin 100minds -- --counsel "Should we add caching?"

# With category hint for better matching
cargo run --bin 100minds -- --counsel "Should we add caching?" --category "[PERF]"

# Record an outcome (closes the learning loop)
cargo run --bin 100minds -- --outcome <decision-id> --success --principles "id1,id2"

# View learning statistics
cargo run --bin 100minds -- --stats
```

### MCP Server Mode

```bash
# Start HTTP server on port 3100
cargo run --release --bin 100minds -- --serve 3100
```

### As a Library

```rust
use minds_mcp::{db, counsel, CounselRequest};

// Initialize database
let conn = db::init_db(&db_path)?;

// Get counsel
let request = CounselRequest {
    question: "Should we use microservices?".to_string(),
    context: None,
    depth: None,
};

let response = counsel::get_counsel(&conn, &provenance, &request)?;

for position in &response.positions {
    println!("{}: {} ({})", position.stance, position.thinker, position.argument);
}
```

## Architecture

```
100minds-mcp/
├── src/
│   ├── main.rs          # CLI + MCP server
│   ├── lib.rs           # Library exports
│   ├── counsel.rs       # Core decision engine
│   ├── db.rs            # SQLite + FTS5 storage
│   ├── provenance.rs    # Cryptographic signatures
│   ├── templates.rs     # 12 decision templates
│   ├── types.rs         # Core data structures
│   ├── outcome.rs       # Learning loop
│   ├── embeddings.rs    # Semantic search (MiniLM)
│   ├── mcp.rs           # MCP protocol handlers
│   └── eval/            # Evaluation framework
│       ├── thompson.rs  # Thompson Sampling
│       ├── scenarios.rs # Benchmark suite
│       ├── llm_judge.rs # LLM-as-Judge
│       └── coverage.rs  # Thinker analysis
└── eval/
    └── scenarios/       # Test scenarios with ground truth
```

## MCP API Reference

### `counsel` - Get adversarial wisdom

```json
{
  "method": "counsel",
  "params": {
    "question": "Should we rewrite the legacy system?",
    "context": { "domain": "architecture" },
    "depth": "standard"
  }
}
```

**Response:**
```json
{
  "decision_id": "550e8400-e29b-41d4-a716-446655440000",
  "positions": [
    {
      "stance": "for",
      "thinker": "Martin Fowler",
      "argument": "Strangler Fig Pattern enables incremental migration...",
      "principles_cited": ["strangler-fig-pattern"],
      "confidence": 0.82,
      "falsifiable_if": "Migration takes >6 months"
    }
  ],
  "challenge": {
    "stance": "challenge",
    "thinker": "Devil's Advocate",
    "argument": "Missing considerations: rollback plan, team capacity..."
  },
  "provenance": {
    "signature": "...",
    "content_hash": "...",
    "previous_hash": "..."
  }
}
```

### `record_outcome` - Close the learning loop

```json
{
  "method": "record_outcome",
  "params": {
    "decision_id": "550e8400-e29b-41d4-a716-446655440000",
    "success": true,
    "principle_ids": ["strangler-fig-pattern", "yagni"],
    "notes": "Migration completed in 3 months, no P0 bugs"
  }
}
```

### `sync_posteriors` - Get Thompson Sampling state

```json
{
  "method": "sync_posteriors",
  "params": {
    "since_ts": 1706500000,
    "domain": "architecture"
  }
}
```

## Decision Templates

100minds includes 12 pre-built templates for common decisions:

| Template | Triggers | Key Thinkers |
|----------|----------|--------------|
| Monolith vs Microservices | "microservice", "monolith" | Fowler, Newman |
| Build vs Buy | "build or buy", "vendor" | Porter, Spolsky |
| Rewrite vs Refactor | "rewrite", "from scratch" | Brooks, Fowler |
| Scale Team | "hire", "add engineers" | Brooks, Bezos |
| Add Caching | "cache", "performance" | Knuth, Spolsky |
| SQL vs NoSQL | "database", "nosql" | Codd, Stonebraker |
| TDD Adoption | "tdd", "test first" | Beck, Fowler |
| Technical Debt | "tech debt", "refactor" | Cunningham, Martin |
| Premature Optimization | "optimize", "performance" | Knuth, Wirth |
| Conway's Law | "team structure", "org" | Conway, Brooks |
| YAGNI | "might need", "future" | Beck, Jeffries |
| Simple Thing | "complex", "abstraction" | Dijkstra, Pike |

## Evaluation Framework

### Run Benchmarks

```bash
# Scenario benchmarks (precision, recall, NDCG)
cargo run --bin 100minds -- --benchmark scenarios

# Monte Carlo simulation (10K runs)
cargo run --bin 100minds -- --benchmark monte-carlo --runs 10000

# Thinker coverage analysis
cargo run --bin 100minds -- --benchmark coverage

# Full evaluation suite
cargo run --bin 100minds -- --benchmark all
```

### Current Metrics

| Metric | Value | Target |
|--------|-------|--------|
| Precision@3 | 0.72 | >0.70 |
| Recall | 0.81 | >0.80 |
| NDCG | 0.76 | >0.75 |
| Tests | 151 | — |

## Configuration

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `ANTHROPIC_API_KEY` | For LLM-as-Judge evaluation | — |
| `ORT_DYLIB_PATH` | ONNX Runtime library path | System default |
| `MINDS_DB_PATH` | Database location | `~/.local/share/100minds/minds.db` |

### Semantic Search Setup

100minds uses all-MiniLM-L6-v2 for vocabulary-mismatch-proof search:

```bash
# Install ONNX Runtime (macOS)
brew install onnxruntime

# Model downloads automatically on first use (~22MB)
```

## The Thinkers

100minds draws from masters across domains:

**Software Engineering:** Kent Beck, Martin Fowler, Robert Martin, Fred Brooks, Donald Knuth

**Systems Thinking:** Donella Meadows, John Gall, W. Edwards Deming

**Decision Making:** Daniel Kahneman, Nassim Taleb, Charlie Munger

**Entrepreneurship:** Paul Graham, Eric Ries, Marc Andreessen

**Philosophy of Science:** Karl Popper, Richard Feynman, Claude Shannon

[View all 70 thinkers →](docs/THINKERS.md)

## Integration Examples

### With Claude Code

```bash
# In your CLAUDE.md
100minds counsel available at localhost:3100
Before major decisions, query: curl -X POST localhost:3100/mcp ...
```

### With Swarm Orchestrators

```rust
// Pre-work counsel
let counsel = client.counsel(&question, Some(category)).await?;
inject_into_worker_prompt(counsel);

// On completion
client.record_outcome(&decision_id, success, &principle_ids).await?;
```

## Development

```bash
# Run tests
cargo test

# Run with verbose logging
RUST_LOG=debug cargo run --bin 100minds -- --counsel "test"

# Check formatting
cargo fmt --check

# Run clippy
cargo clippy
```

## Contributing

Contributions welcome! Areas of interest:

- **New thinkers**: Add wisdom from underrepresented domains
- **Better templates**: Improve decision matching heuristics
- **Evaluation scenarios**: Expand benchmark coverage
- **Integrations**: Connect to more AI agent frameworks

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

MIT License - see [LICENSE](LICENSE) for details.

## Acknowledgments

Built on the shoulders of giants. Every principle in 100minds traces back to published work by the thinkers cited. This project is a tribute to their wisdom.

---

**"The purpose of abstraction is not to be vague, but to create a new semantic level in which one can be absolutely precise."** — Edsger Dijkstra
