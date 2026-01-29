# AGENTS.md - AI Agent Integration Guide

This document helps AI coding agents (Claude Code, Cursor, Copilot, etc.) work effectively with 100minds.

## Quick Start for Agents

```bash
# Get counsel on a decision
100minds --counsel "Should we use microservices?" --json

# Record outcome after decision plays out
100minds --outcome <decision-id> --success --principles "yagni,strangler-fig"

# Search for relevant principles
100minds --search "database scaling" --json
```

## MCP Server Endpoints

100minds exposes these JSON-RPC methods when running as an MCP server:

| Method | Purpose | Key Parameters |
|--------|---------|----------------|
| `counsel` | Get adversarial positions on a decision | `question`, `context`, `depth` |
| `record_outcome` | Close the learning loop | `decision_id`, `success`, `principle_ids` |
| `sync_posteriors` | Get Thompson Sampling state | `since_ts`, `domain` |
| `validate_prd` | Check PRD against principles | `prd_json` |
| `check_blind_spots` | Find missing considerations | `question`, `positions` |

### Example: Get Counsel

```json
// Request
{
  "method": "counsel",
  "params": {
    "question": "Should we rewrite the authentication system?",
    "context": {"domain": "architecture", "team_size": 5},
    "depth": "standard"
  }
}

// Response
{
  "decision_id": "550e8400-e29b-41d4-a716-446655440000",
  "positions": [
    {
      "stance": "for",
      "thinker": "Martin Fowler",
      "principle": "Strangler Fig Pattern",
      "argument": "Incremental replacement reduces risk...",
      "confidence": 0.82,
      "falsifiable_if": "Migration takes >6 months"
    },
    {
      "stance": "against",
      "thinker": "Fred Brooks",
      "principle": "Second System Effect",
      "argument": "Rewrites tend to over-engineer...",
      "confidence": 0.78,
      "falsifiable_if": "Rewrite scope creeps beyond original requirements"
    }
  ],
  "challenge": {
    "missing_considerations": ["rollback plan", "team capacity", "feature freeze impact"]
  }
}
```

## CLI Flags for Agents

| Flag | Purpose |
|------|---------|
| `--json` | Output as JSON (default for MCP, optional for CLI) |
| `--depth quick\|standard\|deep` | Control number of positions returned |
| `--category [ARCH]\|[PERF]\|[TEAM]` | Hint at decision domain |
| `--quiet` | Suppress progress output |

## Integration Patterns

### Pre-Work Counsel

Before starting a task, query 100minds for relevant principles:

```bash
COUNSEL=$(100minds --counsel "Adding caching to API" --json --depth quick)
# Inject principles into prompt context
```

### Post-Work Outcome Recording

After task completion, record the outcome:

```bash
100minds --outcome "$DECISION_ID" \
  --success \
  --principles "yagni,premature-optimization" \
  --notes "Caching added, 3x latency improvement"
```

### Semantic Search

Find principles related to current work:

```bash
100minds --search "database migration strategy" --json --limit 5
```

## Data Locations

| Path | Contents |
|------|----------|
| `~/.local/share/100minds/wisdom.db` | SQLite database (decisions, principles, learning) |
| `~/.local/share/100minds/provenance.key` | Ed25519 signing key |
| `data/thinkers/` | Source JSON for all 100 thinkers |

## Schema Reference

### Principle Structure

```typescript
interface Principle {
  id: string;           // kebab-case identifier
  thinker_id: string;   // Reference to thinker
  name: string;         // Human-readable name
  description: string;  // What it teaches
  domain_tags: string[];// Categories: ["architecture", "testing", ...]
  falsification: string;// When this principle fails
  confidence: number;   // 0.0-1.0, learned from outcomes
}
```

### Decision Structure

```typescript
interface Decision {
  id: string;           // UUID
  question: string;     // Original question
  positions: Position[];// FOR/AGAINST/CHALLENGE
  provenance: {
    signature: string;  // Ed25519 signature
    content_hash: string;
    previous_hash: string;
  };
  outcome?: {
    success: boolean;
    principles_validated: string[];
    notes: string;
  };
}
```

## Quality Gates

Before committing changes influenced by 100minds counsel:

1. **Verify falsification criteria** - Did you check the "fails when" conditions?
2. **Record outcome** - Close the learning loop so confidence adjusts
3. **Check for blind spots** - Run `check_blind_spots` if uncertain

## Error Handling

| Error | Meaning | Action |
|-------|---------|--------|
| `NO_MATCHING_PRINCIPLES` | Query too specific | Broaden question or use different keywords |
| `DB_NOT_INITIALIZED` | First run | Run any command to auto-initialize |
| `INVALID_DECISION_ID` | Unknown decision | Check `--list-decisions` for valid IDs |

## Testing Integration

```bash
# Verify 100minds is working
100minds --stats

# Expected output includes:
# - Thinker count: 100
# - Principle count: 400+
# - Decision count: N
# - Outcome rate: X%
```

## Contributing New Thinkers

See `data/thinkers/README.md` for JSON schema and contribution guidelines.

---

*This document follows the AGENTS.md convention for AI-agent-friendly repositories.*
