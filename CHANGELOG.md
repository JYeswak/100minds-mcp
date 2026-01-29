# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-01-29

### Added
- Initial public release
- **Adversarial Wisdom Council**: 70 thinkers, 345 principles across software, systems, and entrepreneurship domains
- **MCP Server**: JSON-RPC interface for AI agent integration
  - `counsel` - Get adversarial positions on decisions
  - `record_outcome` - Close the learning loop
  - `sync_posteriors` - Thompson Sampling state sync
  - `counterfactual_sim` - "What if" analysis
- **Thompson Sampling**: Feel-Good TS with domain-specific learning
  - Asymmetric adjustments (+0.05 success, -0.10 failure)
  - Optimism bonus for cold-start exploration
  - Per-domain contextual arms
- **Cryptographic Provenance**: Ed25519 signatures + SHA-256 hash chain
- **Decision Templates**: 12 pre-built templates for common decisions
- **Semantic Search**: all-MiniLM-L6-v2 embeddings for vocabulary-mismatch-proof matching
- **Evaluation Framework**:
  - Scenario benchmarks with ground truth
  - Monte Carlo simulation
  - LLM-as-Judge (Claude Haiku)
  - Thinker coverage analysis
- **CLI**: Full command-line interface for all operations
- **Library**: Rust crate for embedding in other projects
- 151 tests

### Security
- No hardcoded secrets
- Environment-based API key configuration
- Restrictive key file permissions

[Unreleased]: https://github.com/zeststream/100minds-mcp/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/zeststream/100minds-mcp/releases/tag/v0.1.0
