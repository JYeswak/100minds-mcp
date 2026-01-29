# Contributing to 100minds

Thank you for your interest in contributing to 100minds! This document provides guidelines and information for contributors.

## Code of Conduct

Be respectful, constructive, and kind. We're all here to build something useful.

## How to Contribute

### Reporting Issues

- Check existing issues first to avoid duplicates
- Include reproduction steps, expected vs actual behavior
- Specify your environment (OS, Rust version, etc.)

### Pull Requests

1. **Fork the repository** and create a feature branch
2. **Write tests** for new functionality
3. **Run the test suite**: `cargo test`
4. **Format code**: `cargo fmt`
5. **Run clippy**: `cargo clippy`
6. **Submit PR** with clear description of changes

### Areas for Contribution

#### Adding New Thinkers

The wisdom database can always grow. To add a new thinker:

1. Ensure they have published, citable work
2. Extract 3-5 core principles with clear application rules
3. Add to `src/bin/import.rs` or create a JSON import file
4. Include falsification criteria (how would we know the principle is wrong?)

**Template:**
```json
{
  "thinker": {
    "id": "firstname-lastname",
    "name": "Full Name",
    "domain": "primary-domain",
    "background": "Brief bio and why their wisdom matters"
  },
  "principles": [
    {
      "id": "principle-short-name",
      "name": "Principle Name",
      "description": "Clear explanation of the principle",
      "domain_tags": ["tag1", "tag2"],
      "application_rule": "When to apply this principle",
      "anti_pattern": "What happens when you ignore this",
      "falsification": "How to know if this principle is wrong"
    }
  ]
}
```

#### Improving Decision Templates

Templates in `src/templates.rs` match questions to relevant principles. To improve:

1. Add new trigger keywords
2. Improve scoring weights
3. Add new template categories
4. Write tests for edge cases

#### Expanding Evaluation Scenarios

Scenarios in `eval/scenarios/` test the system against ground truth:

```json
{
  "id": "scenario-id",
  "category": "architecture",
  "question": "The decision question",
  "expected_principles": ["principle-ids", "that-should-match"],
  "expected_thinkers": ["thinker-ids"],
  "anti_principles": ["principles-that-would-be-wrong"],
  "difficulty": 3
}
```

#### Building Integrations

We welcome integrations with:
- AI agent frameworks (LangChain, AutoGPT, etc.)
- CI/CD systems
- IDE plugins
- Chat interfaces

## Development Setup

```bash
# Clone your fork
git clone https://github.com/YOUR-USERNAME/100minds-mcp.git
cd 100minds-mcp

# Build
cargo build

# Run tests
cargo test

# Run with debug logging
RUST_LOG=debug cargo run --bin 100minds -- --counsel "test question"
```

### Dependencies

- Rust 1.75+
- SQLite (bundled via rusqlite)
- ONNX Runtime (optional, for semantic search)

### Project Structure

```
src/
├── main.rs          # CLI entry point
├── lib.rs           # Library exports
├── counsel.rs       # Core decision engine (start here!)
├── db.rs            # Database operations
├── templates.rs     # Decision templates
├── types.rs         # Data structures
├── provenance.rs    # Cryptographic signatures
├── outcome.rs       # Learning loop
├── embeddings.rs    # Semantic search
├── mcp.rs           # MCP protocol
└── eval/            # Evaluation framework
```

## Testing

```bash
# Run all tests
cargo test

# Run specific module tests
cargo test counsel

# Run with output
cargo test -- --nocapture

# Run ignored (slow) tests
cargo test -- --ignored
```

## Code Style

- Follow Rust standard style (`cargo fmt`)
- Use meaningful variable names
- Add doc comments for public APIs
- Keep functions focused and small
- Prefer explicit error handling over `.unwrap()`

## Commit Messages

Follow conventional commits:

```
feat: add new decision template for API design
fix: correct Thompson Sampling cold-start behavior
docs: improve README quick start section
test: add scenarios for database decisions
refactor: simplify counsel engine matching logic
```

## Questions?

Open an issue with the "question" label or start a discussion.

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
