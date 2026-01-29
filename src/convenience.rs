//! Convenience functions for Zesty integration
//!
//! Two modes:
//! 1. Simple mode: `get_counsel()` - no provenance, fast queries
//! 2. Full mode: `ZestyEngine` - full provenance chain, stored decisions
//!
//! For production, use `ZestyEngine` to get cryptographic audit trail.

use crate::counsel::CounselEngine;
use crate::db;
use crate::outcome::{self, OutcomeResult};
use crate::provenance::Provenance;
use crate::types::{CounselRequest, CounselContext, CounselResponse};
use anyhow::Result;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::path::Path;

// ============================================================================
// FULL MODE: ZestyEngine with Provenance
// ============================================================================

/// Full 100minds engine for Zesty with cryptographic provenance
///
/// Initialize once at daemon startup, use for all counsel calls.
/// Decisions are stored with cryptographic chain for audit.
///
/// ```rust,ignore
/// // At daemon startup
/// let engine = ZestyEngine::init(
///     &data_dir.join("wisdom.db"),
///     &data_dir.join("zesty.key"),
/// )?;
///
/// // For each bead
/// let response = engine.counsel("should I add caching?", Some("architecture"))?;
/// // ... worker executes ...
/// engine.record_outcome(&response.decision_id, success, &principle_ids, notes)?;
/// ```
pub struct ZestyEngine {
    conn: Connection,
    provenance: Provenance,
}

impl ZestyEngine {
    /// Initialize the engine with database and key paths
    pub fn init(db_path: &Path, key_path: &Path) -> Result<Self> {
        let conn = db::init_db(db_path)?;
        let provenance = Provenance::init(key_path)?;
        Ok(Self { conn, provenance })
    }

    /// Get full counsel with provenance chain
    pub fn counsel(&self, question: &str, domain: Option<&str>) -> Result<CounselResponse> {
        let engine = CounselEngine::new(&self.conn, &self.provenance);
        let request = CounselRequest {
            question: question.to_string(),
            context: CounselContext {
                domain: domain.map(|s| s.to_string()),
                ..Default::default()
            },
        };
        engine.counsel(&request)
    }

    /// Record outcome with principle IDs
    pub fn record_outcome(
        &self,
        decision_id: &str,
        success: bool,
        principle_ids: &[String],
        notes: &str,
    ) -> Result<OutcomeResult> {
        outcome::record_outcome(&self.conn, decision_id, success, principle_ids, notes, None)
    }

    /// Record bead completion
    pub fn record_bead(
        &self,
        bead_id: &str,
        bead_title: &str,
        success: bool,
        principle_ids: &[String],
        notes: &str,
        category: Option<&str>,
    ) -> Result<OutcomeResult> {
        outcome::record_bead_outcome(&self.conn, bead_id, bead_title, success, principle_ids, notes, category)
    }

    /// Get learning summary
    pub fn learning_summary(&self, days: Option<i64>) -> Result<LearningSummary> {
        get_learning_summary(&self.conn, days)
    }

    /// Get the public key for verification
    pub fn public_key(&self) -> String {
        self.provenance.public_key_hex()
    }

    /// Access the database connection for advanced queries
    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    /// Extract all principle IDs from a counsel response (for outcome recording)
    pub fn extract_principle_ids(response: &CounselResponse) -> Vec<String> {
        let mut ids: Vec<String> = response.positions
            .iter()
            .flat_map(|p| p.principles_cited.clone())
            .collect();

        // Add challenge principles too
        ids.extend(response.challenge.principles_cited.clone());

        // Deduplicate
        ids.sort();
        ids.dedup();
        ids
    }
}

// ============================================================================
// SIMPLE MODE: No Provenance (for quick queries)
// ============================================================================

/// Simple counsel result without provenance overhead
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimpleCounsel {
    pub principles: Vec<CounselPrinciple>,
    pub blind_spots: Vec<String>,
    pub anti_patterns: Vec<String>,
}

/// A principle with actionable guidance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CounselPrinciple {
    pub id: String,
    pub name: String,
    pub thinker: String,
    pub description: String,
    pub confidence: f64,
    pub action: String,
}

/// Get counsel without full engine setup (no provenance, no storage)
///
/// This is the simple API for Zesty workers:
/// ```rust,ignore
/// let counsel = get_counsel(&conn, "should I add caching?", Some("architecture"), 5)?;
/// for p in counsel.principles {
///     println!("{} says: {}", p.thinker, p.action);
/// }
/// ```
pub fn get_counsel(
    conn: &Connection,
    query: &str,
    category: Option<&str>,
    limit: usize,
) -> Result<SimpleCounsel> {
    // Search for relevant principles
    let mut principles = db::search_principles(conn, query, limit * 2)?;

    // If category specified, also search by domain
    if let Some(cat) = category {
        let domain = category_to_domain(cat);
        let domain_principles = db::get_principles_by_domain(conn, domain)?;
        principles.extend(domain_principles);
    }

    // Deduplicate by ID
    let mut seen = std::collections::HashSet::new();
    principles.retain(|p| seen.insert(p.id.clone()));

    // Sort by confidence and take top N
    principles.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal));
    principles.truncate(limit);

    // Convert to simple format with thinker name lookup
    let counsel_principles: Vec<CounselPrinciple> = principles
        .into_iter()
        .map(|p| {
            let thinker_name = get_thinker_name(conn, &p.thinker_id).unwrap_or_else(|_| p.thinker_id.clone());
            CounselPrinciple {
                id: p.id,
                name: p.name.clone(),
                thinker: thinker_name,
                description: p.description.clone(),
                confidence: p.confidence,
                action: generate_action(&p.name, &p.description),
            }
        })
        .collect();

    // Generate blind spots based on query keywords
    let blind_spots = generate_blind_spots(query);
    let anti_patterns = generate_anti_patterns(query);

    Ok(SimpleCounsel {
        principles: counsel_principles,
        blind_spots,
        anti_patterns,
    })
}

/// Record bead completion with outcome
///
/// This is THE function that activates the learning flywheel.
/// Call this when a bead closes (success or failure).
pub fn record_bead_completion(
    conn: &Connection,
    bead_id: &str,
    bead_title: &str,
    success: bool,
    principle_ids: &[String],
    notes: &str,
    category: Option<&str>,
) -> Result<OutcomeResult> {
    outcome::record_bead_outcome(conn, bead_id, bead_title, success, principle_ids, notes, category)
}

/// Get learning summary with optional time window
pub fn get_learning_summary(conn: &Connection, days: Option<i64>) -> Result<LearningSummary> {
    let stats = outcome::get_learning_stats(conn)?;

    // If days specified, filter to recent timeframe
    let (recent_outcomes, recent_success_rate) = if let Some(d) = days {
        get_recent_stats(conn, d)?
    } else {
        (stats.total_outcomes, stats.success_rate)
    };

    Ok(LearningSummary {
        total_outcomes: stats.total_outcomes,
        recent_outcomes,
        success_rate: stats.success_rate,
        recent_success_rate,
        total_adjustments: stats.total_adjustments,
        principles_learning: stats.principles_with_learning,
        top_improved: stats.top_improved.into_iter().take(5).map(|(name, delta, count)| {
            PrincipleProgress { name, delta, adjustment_count: count }
        }).collect(),
        top_declined: stats.top_declined.into_iter().take(5).map(|(name, delta, count)| {
            PrincipleProgress { name, delta, adjustment_count: count }
        }).collect(),
        flywheel_active: stats.total_outcomes > 0,
    })
}

/// Simplified learning summary for Zesty
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningSummary {
    pub total_outcomes: i64,
    pub recent_outcomes: i64,
    pub success_rate: f64,
    pub recent_success_rate: f64,
    pub total_adjustments: i64,
    pub principles_learning: i64,
    pub top_improved: Vec<PrincipleProgress>,
    pub top_declined: Vec<PrincipleProgress>,
    pub flywheel_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrincipleProgress {
    pub name: String,
    pub delta: f64,
    pub adjustment_count: i64,
}

// Helper functions

fn get_thinker_name(conn: &Connection, thinker_id: &str) -> Result<String> {
    conn.query_row(
        "SELECT name FROM thinkers WHERE id = ?1",
        [thinker_id],
        |row| row.get(0),
    ).map_err(|e| anyhow::anyhow!("Thinker lookup failed: {}", e))
}

fn category_to_domain(category: &str) -> &'static str {
    match category.to_uppercase().as_str() {
        s if s.contains("FIX") => "software-design",
        s if s.contains("FEATURE") => "systems-thinking",
        s if s.contains("HEALING") => "resilience",
        s if s.contains("CI") || s.contains("TEST") => "quality",
        s if s.contains("AUDIT") => "entrepreneurship",
        s if s.contains("SCALE") => "software-architecture",
        s if s.contains("PERF") => "performance",
        s if s.contains("REFACTOR") => "software-design",
        s if s.contains("ARCH") => "software-architecture",
        _ => "general",
    }
}

fn generate_action(name: &str, description: &str) -> String {
    // Generate actionable guidance from principle
    let first_sentence = description.split('.').next().unwrap_or(description);
    format!("Apply {}: {}", name, first_sentence.trim())
}

fn generate_blind_spots(query: &str) -> Vec<String> {
    let query_lower = query.to_lowercase();
    let mut spots = Vec::new();

    if query_lower.contains("add") || query_lower.contains("new") {
        spots.push("Have you considered if this is truly needed? (YAGNI)".to_string());
    }
    if query_lower.contains("scale") || query_lower.contains("performance") {
        spots.push("Have you measured the actual bottleneck first?".to_string());
    }
    if query_lower.contains("rewrite") || query_lower.contains("replace") {
        spots.push("Could incremental migration (Strangler Fig) work instead?".to_string());
    }
    if query_lower.contains("team") || query_lower.contains("people") {
        spots.push("Adding people to a late project makes it later (Brooks's Law)".to_string());
    }

    spots
}

fn generate_anti_patterns(query: &str) -> Vec<String> {
    let query_lower = query.to_lowercase();
    let mut patterns = Vec::new();

    if query_lower.contains("rewrite") {
        patterns.push("Big-bang rewrite: high risk, often fails".to_string());
    }
    if query_lower.contains("microservice") {
        patterns.push("Premature decomposition: start with monolith first".to_string());
    }
    if query_lower.contains("cache") {
        patterns.push("Cache invalidation is one of the two hard problems".to_string());
    }

    patterns
}

fn get_recent_stats(conn: &Connection, days: i64) -> Result<(i64, f64)> {
    let cutoff = chrono::Utc::now() - chrono::Duration::days(days);
    let cutoff_str = cutoff.to_rfc3339();

    let recent_total: i64 = conn.query_row(
        "SELECT COUNT(*) FROM decisions WHERE outcome_recorded_at >= ?1",
        [&cutoff_str],
        |row| row.get(0),
    ).unwrap_or(0);

    let recent_success: i64 = conn.query_row(
        "SELECT COUNT(*) FROM decisions WHERE outcome_success = 1 AND outcome_recorded_at >= ?1",
        [&cutoff_str],
        |row| row.get(0),
    ).unwrap_or(0);

    let rate = if recent_total > 0 {
        recent_success as f64 / recent_total as f64
    } else {
        0.0
    };

    Ok((recent_total, rate))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_category_to_domain() {
        assert_eq!(category_to_domain("[SWARM-FIX]"), "software-design");
        assert_eq!(category_to_domain("AUDIT"), "entrepreneurship");
        assert_eq!(category_to_domain("architecture"), "software-architecture");
    }

    #[test]
    fn test_generate_blind_spots() {
        let spots = generate_blind_spots("should I add a new caching layer?");
        assert!(spots.iter().any(|s| s.contains("YAGNI")));
    }
}
