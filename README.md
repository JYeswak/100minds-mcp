# 100minds

**Adversarial Decision Intelligence for AI Agents**

100minds channels 30 legendary thinkers (growing toward 100)—from Hopper to Schneier, Drucker to Taleb—into an adversarial council that challenges your decisions before they fail in production.

[![CI](https://github.com/JYeswak/100minds-mcp/actions/workflows/ci.yml/badge.svg)](https://github.com/JYeswak/100minds-mcp/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.75+-orange.svg)](https://www.rust-lang.org/)

## Quick Example

```bash
# Install
cargo install --git https://github.com/JYeswak/100minds-mcp.git

# Get counsel on a decision
100minds --counsel "Should we rewrite the legacy system?"

# Output:
# ┌─────────────────────────────────────────────────────────────┐
# │ FOR: Martin Fowler (Strangler Fig Pattern)                  │
# │   "Incremental replacement reduces risk."                   │
# │   Confidence: 0.82                                          │
# ├─────────────────────────────────────────────────────────────┤
# │ AGAINST: Fred Brooks (Second System Effect)                 │
# │   "The second system is the most dangerous."                │
# │   Confidence: 0.78                                          │
# ├─────────────────────────────────────────────────────────────┤
# │ CHALLENGE: What's your rollback plan if this fails?         │
# └─────────────────────────────────────────────────────────────┘
```

## The Problem

AI agents make thousands of decisions. Most fail silently. By the time you notice, the damage is done.

**Traditional approach:** Hope for the best, debug after failure.

**100minds approach:** Every decision faces adversarial scrutiny from the world's greatest minds *before* execution.

## Architecture

```
┌──────────────────────────────────────────────────────────────────┐
│                       Your Question                               │
│            "Should we add caching to the API?"                    │
└──────────────────────────┬───────────────────────────────────────┘
                           ▼
┌──────────────────────────────────────────────────────────────────┐
│                  100minds Counsel Engine                          │
│  ┌──────────────┐  ┌───────────────┐  ┌─────────────────┐       │
│  │ FTS5 Search  │  │   Thompson    │  │    Template     │       │
│  │  + Semantic  │  │   Sampling    │  │    Matching     │       │
│  │   Matching   │  │   Selection   │  │   (12 types)    │       │
│  └──────────────┘  └───────────────┘  └─────────────────┘       │
└──────────────────────────┬───────────────────────────────────────┘
                           ▼
┌──────────────────────────────────────────────────────────────────┐
│          100 Thinkers  │  400+ Principles  │  12 Templates       │
│  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐    │
│  │Software │ │ Systems │ │Business │ │Decision │ │Security │    │
│  │  (25)   │ │  (20)   │ │  (20)   │ │  (20)   │ │  (15)   │    │
│  └─────────┘ └─────────┘ └─────────┘ └─────────┘ └─────────┘    │
└──────────────────────────┬───────────────────────────────────────┘
                           ▼
┌──────────────────────────────────────────────────────────────────┐
│     FOR (with confidence)  │  AGAINST (with confidence)          │
│              │             │              │                       │
│              └─────────────┴──────────────┘                       │
│                            │                                      │
│                   CHALLENGE + Blind Spots                         │
│                            │                                      │
│              Falsification Criteria (when advice fails)           │
└──────────────────────────────────────────────────────────────────┘
                           ▼
┌──────────────────────────────────────────────────────────────────┐
│                    Ed25519 Signed Decision                        │
│                 + SHA-256 Hash Chain Link                         │
└──────────────────────────────────────────────────────────────────┘
```

## Comparison

| Feature | 100minds | ChatGPT | Stack Overflow | Your Gut |
|---------|----------|---------|----------------|----------|
| Adversarial positions | ✅ FOR/AGAINST/CHALLENGE | ❌ Single answer | ❌ Varies | ❌ Confirmation bias |
| Falsification criteria | ✅ Built-in | ❌ None | ❌ None | ❌ None |
| Learning from outcomes | ✅ Thompson Sampling | ❌ No memory | ❌ No | ❌ Unreliable |
| Offline/local | ✅ SQLite | ❌ Cloud-only | ❌ Cloud-only | ✅ Always |
| Cryptographic audit | ✅ Ed25519 chain | ❌ No | ❌ No | ❌ No |
| Domain expertise | ✅ 100 curated thinkers | ⚠️ General | ⚠️ Crowdsourced | ⚠️ Your experience |

## Features

### Adversarial Wisdom Council
- **100 thinkers, 400+ principles** across software, systems, business, decision-making, philosophy, and security
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

## Installation

```bash
# One-line install
cargo install --git https://github.com/JYeswak/100minds-mcp.git

# Or clone and build
git clone https://github.com/JYeswak/100minds-mcp.git
cd 100minds-mcp
cargo build --release
```

## CLI Usage

```bash
# Get counsel on a decision
100minds --counsel "Should we add caching?"

# With category hint for better matching
100minds --counsel "Should we add caching?" --category "[PERF]"

# Record an outcome (closes the learning loop)
100minds --outcome <decision-id> --success --principles "id1,id2"

# View learning statistics
100minds --stats

# Start MCP server
100minds --serve 3100
```

## The 100 Thinkers

100minds draws from masters across six domains:

| Domain | Thinkers | Example Principles |
|--------|----------|-------------------|
| **Software** | Dijkstra, Knuth, Brooks, Fowler, Beck, Hopper, Liskov, Lamport | YAGNI, DRY, Strangler Fig, LSP |
| **Systems** | Meadows, Gall, Forrester, Beer, Ashby, Ackoff | Feedback loops, Requisite variety, POSIWID |
| **Business** | Drucker, Christensen, Deming, Goldratt, Grove | Jobs to be done, Theory of Constraints |
| **Decision-Making** | Kahneman, Taleb, Tetlock, Klein, Simon, Duke | Antifragility, Bounded rationality, Premortems |
| **Philosophy** | Popper, Feynman, Russell, Kuhn, Wittgenstein | Falsifiability, Paradigm shifts |
| **Security** | Schneier, Anderson, Shostack, Spafford, Geer | STRIDE, Defense in depth, Monoculture risk |

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

See [AGENTS.md](AGENTS.md) for complete API documentation.

## Limitations

100minds is powerful but not magic:

- **Not a replacement for domain experts** — 100minds provides frameworks and challenges, not authoritative answers
- **No real-time data** — Principles are timeless wisdom, not current events or market data
- **English only** — Thinker content is currently in English
- **Requires outcome feedback** — The learning loop only works if you record outcomes
- **Curated, not comprehensive** — 100 thinkers can't cover every domain; suggestions welcome

## FAQ

**Q: Why "100minds" specifically?**
A: 100 represents enough diversity to challenge most decisions from multiple angles. We curated thinkers across 6 domains to maximize coverage while maintaining quality.

**Q: How is this different from asking ChatGPT?**
A: 100minds provides *adversarial* positions (FOR/AGAINST/CHALLENGE), includes falsification criteria, learns from your outcomes, works offline, and maintains cryptographic audit trails.

**Q: Can I add my own thinkers?**
A: Yes! Add JSON files to `data/thinkers/<domain>/` following the schema in existing files. See [CONTRIBUTING.md](CONTRIBUTING.md).

**Q: What if the principles conflict?**
A: That's the point! Conflicting principles force you to think through tradeoffs. The confidence scores help, but ultimately you decide.

**Q: How does the learning work?**
A: Thompson Sampling adjusts confidence scores based on recorded outcomes. Success increases confidence (+0.05), failure decreases it more (-0.10). This asymmetry encodes "trust but verify."

## Troubleshooting

### "No matching principles found"
Your question might be too specific. Try broader terms or add a `--category` hint:
```bash
100minds --counsel "Should we use Redis?" --category "[PERF]"
```

### "Database not found"
Run any command to auto-initialize, or set `MINDS_DB_PATH`:
```bash
export MINDS_DB_PATH=~/.local/share/100minds/wisdom.db
100minds --stats
```

### "ONNX runtime not found" (semantic search)
Install ONNX runtime:
```bash
# macOS
brew install onnxruntime

# Linux
apt install libonnxruntime-dev
```

### CI validation failing
Ensure exactly 100 thinker JSON files exist:
```bash
find data/thinkers -name "*.json" | wc -l  # Should be 100
```

## Development

```bash
# Run tests
cargo test

# Run with verbose logging
RUST_LOG=debug 100minds --counsel "test"

# Check formatting
cargo fmt --check

# Run clippy
cargo clippy

# Import thinkers from data/
cargo run --bin import -- data/thinkers
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
