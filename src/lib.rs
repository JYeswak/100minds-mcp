// Clippy allows for cleaner code
#![allow(clippy::too_many_arguments)]
#![allow(clippy::field_reassign_with_default)]

//! 100minds - Adversarial Wisdom Council
//!
//! An MCP server that provides AI agents with decision intelligence
//! through adversarial debate, cryptographic provenance, and outcome learning.
//!
//! # Philosophy
//!
//! Built on principles from the minds it contains:
//!
//! - **Taleb**: Antifragility - decisions that survive challenge are stronger
//! - **Dijkstra**: Simplicity - minimal API surface, maximum clarity
//! - **Feynman**: Explainability - if we can't explain it simply, we don't understand it
//! - **Popper**: Falsifiability - every recommendation includes its failure conditions
//!
//! # Quick Start for Zesty Integration
//!
//! ```rust,ignore
//! use minds_mcp::{init_db, CounselEngine, Provenance};
//! use minds_mcp::outcome::{record_outcome, get_learning_stats};
//! use minds_mcp::embeddings::{SemanticEngine, get_model_dir};
//!
//! // Initialize
//! let conn = init_db(&db_path)?;
//! let provenance = Provenance::init(&key_path)?;
//! let engine = CounselEngine::new(&conn, &provenance);
//!
//! // Get counsel
//! let response = engine.counsel(&request)?;
//!
//! // Record outcome (THE FLYWHEEL)
//! let result = record_outcome(&conn, decision_id, success, &principle_ids, notes, context)?;
//!
//! // Semantic search
//! let mut semantic = SemanticEngine::new(&get_model_dir())?;
//! semantic.load_embeddings(&conn)?;
//! let matches = semantic.hybrid_search(&conn, query, top_k, 0.6)?;
//! ```
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────┐
//! │                   AI Agent (Claude, etc.)            │
//! └─────────────────────┬───────────────────────────────┘
//!                       │ MCP Protocol
//!                       ▼
//! ┌─────────────────────────────────────────────────────┐
//! │              100minds MCP Server                     │
//! │  counsel() → Adversarial debate                      │
//! │  record_outcome() → Learning loop                    │
//! │  audit() → Provenance chain                          │
//! └─────────────────────────────────────────────────────┘
//! ```

pub mod convenience;
pub mod counsel;
pub mod db;
pub mod embeddings;
pub mod eval;
pub mod mcp;
pub mod neural_posterior;
pub mod outcome;
pub mod prd;
pub mod provenance;
pub mod templates;
pub mod types;

// Core types
pub use counsel::CounselEngine;
pub use db::{init_db, PrincipleMatch};
pub use provenance::Provenance;
pub use types::*;

// PRD validation
pub use mcp::{check_blind_spots, get_matching_templates, get_pre_work_context};
pub use mcp::{validate_prd, PrdValidation};

// Outcome recording (THE FLYWHEEL)
pub use outcome::{
    get_learning_stats, record_bead_outcome, record_outcome, LearningStats, OutcomeResult,
    PrincipleAdjustment,
};

// Semantic search
pub use embeddings::{
    get_model_dir, init_embedding_schema, HybridMatch, SemanticEngine, SemanticMatch, EMBEDDING_DIM,
};

// Decision templates
pub use templates::{
    get_templates, AntiPattern, BlindSpot, DecisionTemplate, DecisionTree, PrincipleSynergy,
};

// Convenience API for Zesty
pub use convenience::ZestyEngine; // Full mode with provenance
pub use convenience::{get_counsel, get_learning_summary, record_bead_completion}; // Simple mode
pub use convenience::{CounselPrinciple, LearningSummary, PrincipleProgress, SimpleCounsel};

// Neural posterior (2026 SOTA principle selector)
pub use neural_posterior::{NeuralPosterior, NeuralVocab, PosteriorResult, ScoringContext};
