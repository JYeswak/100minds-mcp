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

100minds channels 100 legendary thinkers—from Knuth to Schneier, Deming to Hinton—into an adversarial council that challenges your decisions before they fail in production.

[![CI](https://github.com/JYeswak/100minds-mcp/actions/workflows/ci.yml/badge.svg)](https://github.com/JYeswak/100minds-mcp/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.75+-orange.svg)](https://www.rust-lang.org/)
[![Thinkers](https://img.shields.io/badge/thinkers-100-green.svg)](data/thinkers/)
[![Principles](https://img.shields.io/badge/principles-354-blue.svg)](data/thinkers/)

## TL;DR

```bash
# Install (30 seconds)
cargo install --git https://github.com/JYeswak/100minds-mcp.git

# Get counsel
100minds counsel "Should we rewrite the legacy system?"
```

**Output:**
```
┌─────────────────────────────────────────────────────────────┐
│ 🧠 100MINDS DECISION INTELLIGENCE                           │
└─────────────────────────────────────────────────────────────┘

📋 Should we rewrite the legacy system?

┌─ IF YOU PROCEED ────────────────────────────────────────────
│  Fred Brooks says: [▓▓░]
│    Adding more engineers to a late project makes it later.
│    ⚠️ Falsifiable if: Small, independent tasks can parallelize
│
│  Kent Beck says: [▓▓░]
│    Do the simplest thing that could possibly work.
│    ⚠️ Falsifiable if: Simple solution can't meet requirements
│
├─ WATCH OUT FOR ──────────────────────────────────────────────
│  Donald Knuth says: [▓▓░]
│    Premature optimization is the root of all evil.
│    ⚠️ Falsifiable if: Performance is a hard requirement
│
└─ BEFORE DECIDING ─────────────────────────────────────────────
   🔍 Missing: rollback plan, team capacity, timeline constraints
```

## The Problem

AI agents make thousands of decisions. Most fail silently. By the time you notice, the damage is done.

| Approach | Result |
|----------|--------|
| **Hope for the best** | Debug after failure, lose time/money |
| **Ask ChatGPT** | Single perspective, no falsification criteria |
| **100minds** | Adversarial positions, testable advice, learns from outcomes |

## Quick Start

### Option 1: Cargo (Recommended)

```bash
cargo install --git https://github.com/JYeswak/100minds-mcp.git
100minds --stats
```

### Option 2: Docker

```bash
docker run -p 3100:3100 ghcr.io/jyeswak/100minds-mcp:latest
# Or build locally:
docker compose up
```

### Option 3: From Source

```bash
git clone https://github.com/JYeswak/100minds-mcp.git
cd 100minds-mcp
cargo build --release
./target/release/100minds --stats
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
│  │  FTS5 Search  │  │   Semantic    │  │   Template    │  │  Thompson   │  │
│  │  + Keywords   │  │   Matching    │  │   Detection   │  │  Sampling   │  │
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
│  │          │ │          │ │          │ │          │ │          │ │      │ │
│  │  Knuth   │ │  Deming  │ │  Drucker │ │  Hinton  │ │  Dennett │ │Schnei│ │
│  │  Fowler  │ │  Ohno    │ │  Thiel   │ │  LeCun   │ │  Bostrom │ │  er  │ │
│  │  Brooks  │ │  Senge   │ │  Graham  │ │  Pearl   │ │Hofstadter│ │Mitnick│
│  └──────────┘ └──────────┘ └──────────┘ └──────────┘ └──────────┘ └──────┘ │
└──────────────────────────────┬──────────────────────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│  ✅ FOR (confidence)  │  ⚠️ AGAINST (confidence)  │  🔍 CHALLENGE           │
│                       │                           │                         │
│  + Falsification      │  + Falsification          │  + Missing              │
│    criteria           │    criteria               │    considerations       │
└──────────────────────────────┬──────────────────────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│              Ed25519 Signature  │  SHA-256 Hash Chain  │  Audit Trail       │
└─────────────────────────────────────────────────────────────────────────────┘
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

## Benchmark Results

```
┌─────────────────────────────────────────────────────────────┐
│ 📊 100MINDS BENCHMARK (100 scenarios)                       │
└─────────────────────────────────────────────────────────────┘

Performance category:    P@3: 31%  ✅ Best
Testing category:        P@3: 13%
Build-vs-buy:           P@3: 13%
Anti-principle rate:     0.0%  ✅ Never gives bad advice
Average latency:         16ms  ✅ Fast

Top utilized thinkers:
  Fred Brooks          77.9% ███████████████
  Donald Knuth         76.0% ███████████████
  Kent Beck            47.1% █████████
  Sam Newman           42.3% ████████
```

## CLI Usage

```bash
# Get adversarial counsel
100minds counsel "Should we use microservices?"

# With JSON output for automation
100minds counsel "Should we add caching?" --json

# Record outcome (closes learning loop)
100minds --outcome <decision-id> --success --principles "yagni,kiss"

# View statistics
100minds --stats

# Run as MCP server
100minds --serve --port=3100

# Run benchmarks
100minds --benchmark scenarios
100minds --analyze coverage
```

## MCP Server Integration

100minds exposes JSON-RPC methods for AI agent integration:

```json
// Request counsel
{
  "method": "counsel",
  "params": {
    "question": "Should we rewrite the auth system?",
    "context": { "domain": "architecture" },
    "depth": "standard"
  }
}

// Record outcome
{
  "method": "record_outcome",
  "params": {
    "decision_id": "550e8400-e29b-41d4-a716-446655440000",
    "success": true,
    "principle_ids": ["strangler-fig", "yagni"]
  }
}
```

See [AGENTS.md](AGENTS.md) for complete API documentation.

## Features

### Adversarial Wisdom Council
- **100 thinkers, 354 principles** across 6 domains
- **FOR/AGAINST/CHALLENGE** positions force genuine consideration
- **Falsification criteria** make advice testable

### Thompson Sampling Learning
- **Asymmetric adjustments**: Failures hurt more (+0.05/-0.10)
- **Domain-specific**: "YAGNI works for architecture, not security"
- **Feel-Good sampling**: Optimism bonus prevents cold-start paralysis

### Cryptographic Provenance
- **Ed25519 signatures** on every decision
- **SHA-256 hash chain** for audit trails
- **Tamper detection** built-in

## Comparison

| Feature | 100minds | ChatGPT | Your Gut |
|---------|----------|---------|----------|
| Adversarial positions | ✅ FOR/AGAINST | ❌ Single answer | ❌ Confirmation bias |
| Falsification criteria | ✅ Built-in | ❌ None | ❌ None |
| Learns from outcomes | ✅ Thompson Sampling | ❌ No memory | ❌ Unreliable |
| Works offline | ✅ SQLite | ❌ Cloud-only | ✅ Always |
| Cryptographic audit | ✅ Ed25519 chain | ❌ No | ❌ No |
| 100 curated experts | ✅ Yes | ⚠️ General | ⚠️ Your experience |

## Limitations

- **Not a replacement for domain experts** — Provides frameworks, not authoritative answers
- **No real-time data** — Principles are timeless wisdom, not current events
- **English only** — Thinker content is currently in English
- **Requires feedback** — Learning loop needs recorded outcomes
- **Curated, not comprehensive** — 100 thinkers can't cover every domain

## FAQ

**Q: Why exactly 100 thinkers?**
A: Enough diversity to challenge most decisions from multiple angles, while maintaining quality. Each thinker is manually curated with verified principles.

**Q: How is this different from ChatGPT?**
A: 100minds provides *adversarial* positions (FOR/AGAINST/CHALLENGE), includes falsification criteria for each principle, learns from your outcomes via Thompson Sampling, works completely offline, and maintains cryptographic audit trails.

**Q: Can I add my own thinkers?**
A: Yes! Add JSON files to `data/thinkers/<domain>/` following the schema. Run `cargo run --bin import -- data/thinkers` to reimport.

**Q: What if principles conflict?**
A: That's the point! Conflicting principles force you to think through tradeoffs. The confidence scores help prioritize, but ultimately you decide.

## Troubleshooting

### "No matching principles"
Try broader terms or specify domain:
```bash
100minds counsel "Should we use Redis?" --domain performance
```

### "Database not found"
Import thinkers first:
```bash
cargo run --bin import -- data/thinkers
```

### ONNX runtime (semantic search)
```bash
# macOS
brew install onnxruntime

# Linux
apt install libonnxruntime-dev
```

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
