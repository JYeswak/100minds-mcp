# AGENTS.md - AI Agent Integration Guide

This document helps AI coding agents (Claude Code, Cursor, Copilot, swarm workers) integrate with 100minds for decision intelligence.

## Quick Reference

```bash
# Build and run
cargo build --release
./target/release/100minds --stats

# Get counsel (CLI)
./target/release/100minds counsel "Should we use microservices?"

# Run as HTTP server (production mode)
./target/release/100minds --serve --port=3100

# Record outcome (CLI)
./target/release/100minds --outcome bead-josh-abc123 --success

# Validate PRD
./target/release/100minds --validate-prd path/to/prd.json
```

## Project Structure

### Core Components
- **`src/main.rs`**: CLI entry point with command-line interface
- **`src/lib.rs`**: Main library exposing all modules
- **`src/bin/import.rs`**: Data import utility for thinker/principle data

### Key Modules
| Module | Purpose |
|--------|---------|
| `counsel` | Adversarial decision engine (FOR/AGAINST/CHALLENGE) |
| `db` | SQLite database operations and schema |
| `embeddings` | ONNX Runtime semantic search (100x improvement) |
| `mcp` | MCP server protocol implementation |
| `outcome` | Learning loop - Thompson Sampling on decision outcomes |
| `prd` | PRD validation against philosophical frameworks |
| `provenance` | Ed25519 cryptographic signatures and audit trails |
| `templates` | Decision templates for common scenarios |
| `types` | Core data structures and enums |

### Dependencies
- **Async**: tokio (full features)
- **Database**: rusqlite (bundled SQLite)
- **Crypto**: ed25519-dalek, sha2 for provenance
- **ML**: ort (ONNX Runtime), tokenizers, ndarray for embeddings
- **Utils**: serde, uuid, chrono, dirs, walkdir

## Development Workflow

### Building
```bash
# Debug build
cargo build

# Release build (recommended for performance)
cargo build --release

# Run tests
cargo test

# Check and format
cargo check
cargo fmt
cargo clippy
```

### Key Commands

#### Core Commands
| Command | Purpose | Example |
|---------|---------|---------|
| `counsel` | Get adversarial wisdom council | `100minds counsel "Should we add caching?"` |
| `--serve` | HTTP server for MCP integration | `100minds --serve --port=3100` |
| `--outcome` | Record decision outcome for learning | `100minds --outcome bead-123 --success` |

#### Analysis Commands
| Command | Purpose | Example |
|---------|---------|---------|
| `--validate-prd` | Check PRD against frameworks | `100minds --validate-prd prd.json` |
| `--analyze-prd` | Enhanced PRD with 100minds metadata | `100minds --analyze-prd input.json output.json` |
| `--stats` | Database statistics | `100minds --stats` |
| `--learning-stats` | Thompson Sampling performance | `100minds --learning-stats` |

#### Development Commands
| Command | Purpose | Example |
|---------|---------|---------|
| `--benchmark` | Performance testing | `100minds --benchmark scenarios` |
| `--thompson` | Sampling statistics | `100minds --thompson stats` |
| `--compute-embeddings` | Generate semantic embeddings | `100minds --compute-embeddings` |

### Command Line Options

#### Counsel Command
```bash
100minds counsel "question" [--json] [--domain=category]
# --json: JSON output for automation
# --domain: software-development, architecture, performance, etc.
```

#### Server Mode
```bash
100minds --serve --port=3100
# Runs HTTP server for MCP integration
# Default port: 3100
```

#### Outcome Recording
```bash
100minds --outcome <decision-id> --success|--failure [--notes="details"]
# Critical for learning loop - updates Thompson posteriors
```

## Testing

### Test Structure
- **Unit tests**: In each module file (`src/*.rs`)
- **Integration tests**: In `tests/` directory (if present)
- **Dev dependencies**: tokio-test, tempfile

### Running Tests
```bash
# All tests
cargo test

# Specific test
cargo test test_name

# With output
cargo test -- --nocapture

# Benchmark tests
cargo test --bench
```

### Test Categories
- **Database tests**: SQLite operations and schema
- **Counsel tests**: Adversarial debate generation
- **Embedding tests**: ONNX Runtime semantic search
- **MCP tests**: JSON-RPC protocol handling
- **Outcome tests**: Thompson Sampling learning loop

## Deployment

### Production Build
```bash
# Optimized release build
cargo build --release

# Binary location
./target/release/100minds
```

### Docker Deployment
```bash
# Build container
docker build -t 100minds-mcp .

# Run with port mapping
docker run -p 3100:3100 100minds-mcp
```

### System Dependencies
- **ONNX Runtime**: Required for semantic embeddings
  ```bash
  # macOS
  brew install onnxruntime

  # Linux
  apt install libonnxruntime-dev
  ```

### Data Directory
- **Default location**: `~/Library/Application Support/100minds/` (macOS)
- **Alternative**: `~/.local/share/100minds/` (Linux)
- **Contents**: `wisdom.db`, `agent.key`, thinker data

## Library Usage

### Basic Integration
```rust
use minds_mcp::{init_db, CounselEngine, Provenance};

// Initialize
let conn = init_db(&db_path)?;
let provenance = Provenance::init(&key_path)?;
let engine = CounselEngine::new(&conn, &provenance);

// Get counsel
let response = engine.counsel(&request)?;

// Record outcome
outcome::record_outcome(&conn, decision_id, success, &principle_ids, notes, context)?;
```

### Zesty Convenience API
```rust
use minds_mcp::convenience::{ZestyEngine, get_counsel};

// Full mode with provenance
let engine = ZestyEngine::init(&db_path, &key_path)?;
let response = engine.counsel("Should we add caching?", Some("architecture"))?;
engine.record_outcome(&response.decision_id, true, &principle_ids, "Worked well")?;

// Simple mode
let counsel = get_counsel(&conn, "Should we refactor?", Some("code-quality"), 5)?;
```

## Common Patterns

### Decision Workflow
1. **Pre-work**: Get counsel before starting task
2. **Execute**: Implement the decision
3. **Post-work**: Record outcome for learning

### Swarm Integration
- Use `--serve` mode for HTTP API
- Record outcomes with `--outcome` to enable learning
- Decision IDs link counsel to outcomes

### Error Handling
- Check database initialization: `100minds --stats`
- Verify ONNX Runtime: embeddings require ORT
- Test counsel: `100minds counsel "test question"`

## Troubleshooting

### Build Issues
```bash
# Clean and rebuild
cargo clean
cargo build

# Update dependencies
cargo update
```

### Runtime Issues
```bash
# Check database
100minds --stats

# Test counsel
100minds counsel "test question"

# Check embeddings
100minds --compute-embeddings
```

### Performance Issues
- Use `--release` builds for production
- Embeddings require ONNX Runtime
- Database operations are SQLite-based

## HTTP Server Mode

For production swarm integration, run 100minds as an HTTP server:

```bash
100minds --serve --port=3100
```

### MCP Protocol Compliance

The server returns **MCP-compliant responses** for `tools/call`:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "content": [{ "type": "text", "text": "..." }],
    "structuredContent": { ... },
    "isError": false
  }
}
```

This ensures compatibility with Claude Code and other MCP clients.

### Example Request

The server accepts JSON-RPC 2.0 requests at `http://localhost:3100/mcp`:

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
        "question": "Should we add caching?",
        "domain": "architecture"
      }
    }
  }'
```

## Complete MCP Tool Reference

### counsel

Get adversarial wisdom council on a decision. Returns FOR/AGAINST/CHALLENGE positions.

**Arguments:**
| Name | Type | Required | Description |
|------|------|----------|-------------|
| `question` | string | Yes | The decision question |
| `domain` | string | No | Hint domain: architecture, testing, performance, security, scaling, rewrite |
| `depth` | string | No | quick (2 positions), standard (4), deep (6) |
| `decision_id` | string | No | Custom ID for outcome linking (default: UUID) |

**Response:**
```json
{
  "decision_id": "bead-josh-abc123",
  "positions": [
    {
      "thinker": "Kent Beck",
      "thinker_id": "kent-beck",
      "stance": "for",
      "argument": "Do the simplest thing that could possibly work...",
      "confidence": 0.72,
      "principles_cited": ["kiss", "yagni"],
      "falsifiable_if": "Simple solution can't meet requirements"
    }
  ],
  "challenge": {
    "thinker": "Devil's Advocate",
    "argument": "Missing considerations: rollback plan, team capacity",
    "confidence": 0.95
  },
  "causal_hints": ["Kent Beck cites kent-beck-4 for FOR stance"]
}
```

---

### record_outcome

Record the outcome of a decision for Thompson Sampling learning. **Critical for the feedback loop.**

**Arguments:**
| Name | Type | Required | Description |
|------|------|----------|-------------|
| `decision_id` | string | Yes | The decision ID from counsel |
| `success` | boolean | Yes | Whether the decision led to success |
| `notes` | string | No | Optional notes about what happened |

**Response:**
```json
{
  "decision_id": "bead-josh-abc123",
  "principles_adjusted": ["yagni", "kiss"],
  "new_confidences": [0.74, 0.71]
}
```

**Learning rates:**
- Success: α += 0.05
- Failure: β += 0.10 (asymmetric - failures hurt more)

---

### pre_work_context

Get relevant frameworks BEFORE starting work on a task. Use at task start to inject wisdom.

**Arguments:**
| Name | Type | Required | Description |
|------|------|----------|-------------|
| `task` | string | Yes | Description of the task |
| `domain` | string | No | Task domain |

**Response:**
```json
{
  "frameworks": [
    {
      "principle": "YAGNI",
      "thinker": "Kent Beck",
      "relevance": 0.89,
      "guidance": "Don't add features until needed"
    }
  ],
  "anti_patterns": ["gold-plating", "speculative-generality"],
  "suggested_approach": "Start with minimal implementation, iterate"
}
```

---

### search_principles

FTS5 full-text search across 354 principles.

**Arguments:**
| Name | Type | Required | Description |
|------|------|----------|-------------|
| `query` | string | Yes | Search query |
| `domain` | string | No | Filter by domain |
| `limit` | integer | No | Max results (default: 10) |

**Response:**
```json
{
  "results": [
    {
      "principle_id": "strangler-fig",
      "name": "Strangler Fig Pattern",
      "thinker": "Martin Fowler",
      "description": "Incrementally replace legacy systems...",
      "confidence": 0.82,
      "relevance_score": 0.94
    }
  ]
}
```

---

### get_decision_template

Get a guided decision tree for common decisions.

**Arguments:**
| Name | Type | Required | Description |
|------|------|----------|-------------|
| `template_id` | string | Yes | One of: monolith-vs-microservices, rewrite-vs-refactor, build-vs-buy, scale-team, technical-debt, mvp-scope, architecture-migration, database-choice |

**Response:**
```json
{
  "template_id": "rewrite-vs-refactor",
  "questions": [
    {"q": "Is the codebase well-tested?", "weight": 0.3},
    {"q": "Is the team familiar with the code?", "weight": 0.25}
  ],
  "recommendations": {
    "rewrite_if": ["Test coverage <20%", "No original developers"],
    "refactor_if": ["Working software", "Incremental improvement possible"]
  },
  "synergies": ["strangler-fig + feature-flags"],
  "tensions": ["big-bang vs incremental"]
}
```

---

### check_blind_spots

Proactively identify what you might be missing.

**Arguments:**
| Name | Type | Required | Description |
|------|------|----------|-------------|
| `context` | string | Yes | Description of current approach |
| `template_id` | string | No | Optional template for targeted checks |

**Response:**
```json
{
  "blind_spots": [
    {
      "severity": "critical",
      "area": "rollback-plan",
      "check_question": "What happens if deployment fails?",
      "principle": "defense-in-depth"
    },
    {
      "severity": "high",
      "area": "team-capacity",
      "check_question": "Does the team have bandwidth?",
      "principle": "brooks-law"
    }
  ]
}
```

---

### detect_anti_patterns

Check for known bad patterns in your approach.

**Arguments:**
| Name | Type | Required | Description |
|------|------|----------|-------------|
| `description` | string | Yes | Description of the approach |
| `domain` | string | No | Filter by domain |

**Response:**
```json
{
  "anti_patterns": [
    {
      "name": "Second System Effect",
      "symptoms": ["Scope creep", "Over-engineering"],
      "thinker": "Fred Brooks",
      "cure": "Stick to original requirements"
    }
  ]
}
```

---

### validate_prd

Validate a PRD against philosophical frameworks.

**Arguments:**
| Name | Type | Required | Description |
|------|------|----------|-------------|
| `prd_json` | string | Yes | PRD as JSON string |

**Response:**
```json
{
  "score": 72,
  "warnings": [
    {"level": "error", "code": "BROOKS_LAW", "message": ">5 stories increases coordination overhead"},
    {"level": "warning", "code": "YAGNI", "message": "Speculative language detected: 'might need'"}
  ],
  "suggestions": [
    {"thinker": "Kent Beck", "suggestion": "Split into smaller PRDs"}
  ],
  "principles_applied": ["brooks-law", "yagni", "conceptual-integrity"]
}
```

---

### get_synergies

Find principles that work well together.

**Arguments:**
| Name | Type | Required | Description |
|------|------|----------|-------------|
| `principle_ids` | array | Yes | List of principle IDs |

**Response:**
```json
{
  "synergies": [
    {
      "pair": ["strangler-fig", "feature-flags"],
      "combined_power": "Incremental migration with instant rollback"
    }
  ]
}
```

---

### get_tensions

Find principles that conflict—you must choose.

**Arguments:**
| Name | Type | Required | Description |
|------|------|----------|-------------|
| `principle_ids` | array | Yes | List of principle IDs |

**Response:**
```json
{
  "tensions": [
    {
      "pair": ["move-fast", "measure-twice"],
      "pick_a_when": "Exploring new territory, low stakes",
      "pick_b_when": "Production system, high stakes"
    }
  ]
}
```

---

### wisdom_stats

Get statistics on decision outcomes. Which principles have the best track record?

**Arguments:**
| Name | Type | Required | Description |
|------|------|----------|-------------|
| `domain` | string | No | Filter by domain |

**Response:**
```json
{
  "total_decisions": 105776,
  "with_outcomes": 23,
  "success_rate": 0.20,
  "top_principles": [
    {"id": "yagni", "success_rate": 0.85, "applications": 47},
    {"id": "kiss", "success_rate": 0.82, "applications": 39}
  ],
  "bottom_principles": [
    {"id": "big-bang-rewrite", "success_rate": 0.12, "applications": 8}
  ]
}
```

---

### audit_decision

Get full provenance chain for a decision. Ed25519 signatures + SHA-256 hash chain.

**Arguments:**
| Name | Type | Required | Description |
|------|------|----------|-------------|
| `decision_id` | string | Yes | Decision ID to audit |

**Response:**
```json
{
  "decision_id": "bead-josh-abc123",
  "created_at": "2026-01-29T20:14:34Z",
  "provenance": {
    "agent_pubkey": "ed25519:abc123...",
    "content_hash": "sha256:def456...",
    "previous_hash": "sha256:789abc...",
    "signature": "ed25519sig:..."
  },
  "chain_valid": true
}
```

---

### sync_posteriors

Get Thompson Sampling posteriors for all principles. Used by swarm daemons to synchronize learning across workers.

**Arguments:**
| Name | Type | Required | Description |
|------|------|----------|-------------|
| `since_ts` | string | No | Only return updates since timestamp |
| `domain` | string | No | Filter by domain |

**Response:**
```json
{
  "posteriors": [
    {
      "principle_id": "yagni",
      "alpha": 12.5,
      "beta": 2.3,
      "rho": 0.845,
      "pulls": 47,
      "last_updated": "2026-01-29T20:30:00Z"
    }
  ],
  "total_principles": 354
}
```

---

### record_outcomes_batch

Record multiple decision outcomes in batch. Used for offline worker catch-up or daemon restart recovery.

**Arguments:**
| Name | Type | Required | Description |
|------|------|----------|-------------|
| `outcomes` | array | Yes | Array of outcome objects |

**Outcome object:**
```json
{
  "decision_id": "bead-josh-abc123",
  "success": true,
  "principle_ids": ["yagni", "kiss"],
  "domain": "architecture",
  "confidence_score": 0.85,
  "failure_stage": null
}
```

---

### counterfactual_sim

Simulate counsel response excluding specific principles. "What if we hadn't used these principles?"

**Arguments:**
| Name | Type | Required | Description |
|------|------|----------|-------------|
| `question` | string | Yes | Original question |
| `excluded_principles` | array | Yes | Principles to exclude |
| `domain` | string | No | Domain context |

**Response:**
```json
{
  "original_principle_ids": ["yagni", "kiss"],
  "excluded_count": 2,
  "alternative_positions": [...],
  "new_principle_ids": ["premature-optimization", "strangler-fig"],
  "diversity_delta": 0.67
}
```

---

## Swarm Integration Patterns

### Pre-Work Counsel (Zesty Worker)

```rust
// In swarm-daemon worker spawn
let counsel = counsel_for_bead(
    bead_id,
    &format!("Worker starting on: {}", task_description),
    Some("software-development"),
    SwarmUrgency::Normal,
);

// Format for worker context injection
let guidance = format!(
    "## 100minds Pre-Work Guidance\n\n{}\n\n*Decision ID: {} for outcome tracking*",
    format_positions(&counsel.positions),
    counsel.decision_id
);
```

### Post-Work Outcome Recording

```rust
// In swarm-daemon worker completion
record_worker_outcome(
    &decision_id,           // From pre-work counsel
    &principle_ids,         // Extracted from counsel
    task_succeeded,         // bool
    domain,                 // "software-development"
    confidence,             // Worker self-assessment 0.0-1.0
    failure_stage.as_deref() // None or Some("execution")
)?;
```

### Stuck Worker Counsel

```rust
// When worker exceeds 20min without progress
let counsel = counsel_for_bead(
    bead_id,
    &format!("Worker stuck on: {}. Last activity: {}", task, last_output),
    Some(domain),
    SwarmUrgency::Stuck,
);
```

## Data Locations

| Path | Contents |
|------|----------|
| `~/Library/Application Support/100minds/wisdom.db` | SQLite database (macOS) |
| `~/.local/share/100minds/wisdom.db` | SQLite database (Linux) |
| `~/.local/share/100minds/agent.key` | Ed25519 signing key |
| `data/thinkers/` | Source JSON for all 100 thinkers |

## Database Schema

### decisions table
```sql
CREATE TABLE decisions (
    id TEXT PRIMARY KEY,
    question TEXT NOT NULL,
    counsel_json TEXT,           -- Full counsel response
    outcome_success INTEGER,     -- 1=success, 0=failure, NULL=pending
    outcome_notes TEXT,
    outcome_recorded_at TEXT,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP
);
```

### thompson_arms table
```sql
CREATE TABLE thompson_arms (
    principle_id TEXT PRIMARY KEY,
    alpha REAL NOT NULL DEFAULT 1.0,
    beta REAL NOT NULL DEFAULT 1.0,
    pulls INTEGER NOT NULL DEFAULT 0,
    updated_at TEXT DEFAULT CURRENT_TIMESTAMP
);
```

### contextual_arms table
```sql
CREATE TABLE contextual_arms (
    id INTEGER PRIMARY KEY,
    principle_id TEXT NOT NULL,
    domain TEXT NOT NULL,
    alpha REAL DEFAULT 1.0,
    beta REAL DEFAULT 1.0,
    sample_count INTEGER DEFAULT 0,
    last_updated TEXT,
    UNIQUE(principle_id, domain)
);
```

## Error Handling

| Error | Meaning | Action |
|-------|---------|--------|
| `NO_MATCHING_PRINCIPLES` | Query too specific | Broaden question |
| `DB_NOT_INITIALIZED` | First run | Run `100minds --stats` to auto-initialize |
| `INVALID_DECISION_ID` | Unknown decision | Check decision exists before recording |
| `MCP_UNAVAILABLE` | Server not running | Start with `100minds --serve --port=3100` |

## Auto-Start Configuration (macOS)

For persistent operation, install the launchd plist:

```bash
# Location: ~/Library/LaunchAgents/com.zeststream.100minds.plist
# Features: RunAtLoad, KeepAlive (auto-restart on crash)
# Logs: ~/.zesty/100minds.log
```

## Testing Integration

```bash
# Verify server is running
curl http://localhost:3100/health

# Check stats
100minds --stats

# Expected output:
#   Thinkers: 100
#   Principles: 354
#   Decisions: N
#   With outcomes: M
#   Success rate: X%
```

---

*This document follows the AGENTS.md convention for AI-agent-friendly repositories.*

Built with care by [ZestStream](https://zeststream.ai)
