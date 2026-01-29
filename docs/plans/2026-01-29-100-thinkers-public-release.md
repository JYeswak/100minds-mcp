# 100minds: Complete Public Release Design

**Date:** 2026-01-29
**Status:** Approved
**Goal:** Reach 100 thinkers + fix all README gaps for true public release

---

## Problem Statement

1. "100minds" has only 70 thinkers - misleading name
2. README lacks: AGENTS.md, quick example, architecture diagram, limitations, FAQ
3. No CI validation that thinker count stays at 100

---

## Solution Overview

### Part 1: Add 30 Thinkers

**Data Structure:** JSON files in `data/thinkers/<domain>/<id>.json`

```json
{
  "id": "grace-hopper",
  "name": "Grace Hopper",
  "domain": "software",
  "background": "Computer scientist, US Navy rear admiral...",
  "principles": [
    {
      "name": "Principle Name",
      "description": "What it teaches",
      "domain_tags": ["tag1", "tag2"],
      "falsification": "When this principle fails"
    }
  ]
}
```

**Validation Rules:**
- `id`: lowercase kebab-case, unique across all thinkers
- `domain`: one of `software`, `systems`, `philosophy`, `business`, `decision-making`, `security`
- `principles`: 2-6 per thinker
- Each principle requires: `name`, `description`, `domain_tags[]`

### Part 2: The 30 New Thinkers

| Domain | Thinkers (5 each) |
|--------|-------------------|
| **software** | Grace Hopper, Barbara Liskov, Leslie Lamport, John Carmack, James Gosling |
| **systems** | Jay Forrester, Stafford Beer, Russell Ackoff, Peter Checkland, W. Ross Ashby |
| **philosophy** | Bertrand Russell, Ludwig Wittgenstein, Thomas Kuhn, Imre Lakatos, Paul Feyerabend |
| **business** | Peter Drucker, Clayton Christensen, W. Edwards Deming, Eli Goldratt, Andy Grove |
| **decision-making** | Gary Klein, Gerd Gigerenzer, Philip Tetlock, Annie Duke, Herbert Simon |
| **security** | Bruce Schneier, Ross Anderson, Adam Shostack, Gene Spafford, Dan Geer |

**Total:** 70 existing + 30 new = 100 thinkers

### Part 3: Enhanced Import Tool

Update `src/bin/import.rs` with new flags:

```
--validate-only    # Check JSON validity without importing
--require-count N  # Fail if total thinkers != N
--test-after       # Run cargo test after import
--stats            # Print detailed import statistics
```

### Part 4: CI Validation

New workflow `.github/workflows/validate-thinkers.yml`:

```yaml
name: Validate Thinkers
on: [push, pull_request]
jobs:
  validate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Validate 100 thinkers
        run: |
          COUNT=$(find data/thinkers -name "*.json" | wc -l)
          [ "$COUNT" -eq 100 ] || exit 1
      - name: Import and test
        run: |
          cargo run --bin import -- data/thinkers --validate-only
          cargo test
```

### Part 5: README Improvements

**New Files:**

| File | Content |
|------|---------|
| `AGENTS.md` | MCP endpoints, JSON output format, CLI schema for AI agents |
| `docs/CLI_SCHEMA.json` | Machine-readable CLI interface |
| `docs/LIMITATIONS.md` | Honest limitations and non-goals |

**README Additions:**

1. **Quick Example** - 5-line install + use before diving into details
2. **ASCII Architecture Diagram** - Visual flow from question to counsel
3. **Feature Comparison Table** - vs ChatGPT, Stack Overflow, etc.
4. **Limitations Section** - What 100minds doesn't do
5. **FAQ Section** - Common questions
6. **Troubleshooting Section** - Common issues and fixes

---

## Implementation Tasks

### Phase A: Thinker Data (30 files)

1. Create `data/thinkers/` directory structure
2. Create 5 software thinker JSON files
3. Create 5 systems thinker JSON files
4. Create 5 philosophy thinker JSON files
5. Create 5 business thinker JSON files
6. Create 5 decision-making thinker JSON files
7. Create 5 security thinker JSON files

### Phase B: Import Tool Enhancement

8. Add `--validate-only` flag to import.rs
9. Add `--require-count N` flag
10. Add `--test-after` flag
11. Add JSON schema validation
12. Add detailed stats output

### Phase C: CI Validation

13. Create `validate-thinkers.yml` workflow
14. Add thinker count check
15. Add import validation step

### Phase D: Documentation

16. Create AGENTS.md
17. Create docs/CLI_SCHEMA.json
18. Create docs/LIMITATIONS.md
19. Update README with quick example
20. Add ASCII architecture diagram
21. Add feature comparison table
22. Add limitations section
23. Add FAQ section
24. Add troubleshooting section

### Phase E: Verification

25. Run full test suite
26. Verify 100 thinkers imported correctly
27. Test MCP server with new thinkers
28. Push to GitHub and verify CI passes

---

## Success Criteria

- [ ] Exactly 100 thinkers in `data/thinkers/`
- [ ] All 100 import successfully with no errors
- [ ] `cargo test` passes (151+ tests)
- [ ] CI validates thinker count on every push
- [ ] README has all sections from beads_rust comparison
- [ ] AGENTS.md exists and documents MCP interface
- [ ] CLI_SCHEMA.json is valid and complete

---

## File Changes Summary

**New Files:**
- `data/thinkers/**/*.json` (30 files)
- `AGENTS.md`
- `docs/CLI_SCHEMA.json`
- `docs/LIMITATIONS.md`
- `.github/workflows/validate-thinkers.yml`

**Modified Files:**
- `src/bin/import.rs` (new flags)
- `README.md` (new sections)

---

## Notes

- Thinkers are curated manually for quality - no LLM generation
- Each thinker's principles should be directly attributable to their published work
- Falsification criteria make principles testable (Popper's influence)
