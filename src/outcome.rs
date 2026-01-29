//! Outcome Recording - The Flywheel Activator
//!
//! This module closes the learning loop by:
//! 1. Recording decision outcomes (success/failure)
//! 2. Adjusting principle confidences via Thompson Sampling
//! 3. Tracking framework adjustments for analysis
//!
//! The flywheel only spins if outcomes are recorded.
//! Without this, learned_confidence never changes.

use crate::eval::thompson::init_thompson_schema;
use anyhow::Result;
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use chrono::Utc;

/// Outcome recording result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutcomeResult {
    pub decision_id: String,
    pub principles_adjusted: Vec<PrincipleAdjustment>,
    pub new_confidences: Vec<(String, f64)>,
}

/// Individual principle adjustment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrincipleAdjustment {
    pub principle_id: String,
    pub principle_name: String,
    pub old_confidence: f64,
    pub new_confidence: f64,
    pub delta: f64,
}

/// Record an outcome for a decision
///
/// This is THE critical function that activates the learning flywheel.
pub fn record_outcome(
    conn: &Connection,
    decision_id: &str,
    success: bool,
    applied_principles: &[String],
    notes: &str,
    context_pattern: Option<&str>,
) -> Result<OutcomeResult> {
    // Initialize Thompson schema if needed
    init_thompson_schema(conn)?;

    // 1. Update the decision with outcome
    let rows_updated = conn.execute(
        "UPDATE decisions
         SET outcome_success = ?2,
             outcome_notes = ?3,
             outcome_recorded_at = ?4
         WHERE id = ?1",
        params![
            decision_id,
            success as i32,
            notes,
            Utc::now().to_rfc3339(),
        ],
    )?;

    if rows_updated == 0 {
        // Decision doesn't exist - create a placeholder
        conn.execute(
            "INSERT INTO decisions (id, question, counsel_json, content_hash, signature, agent_pubkey, outcome_success, outcome_notes, outcome_recorded_at)
             VALUES (?1, ?2, '{}', 'outcome-only', 'none', 'outcome-recorder', ?3, ?4, ?5)",
            params![
                decision_id,
                format!("Outcome recorded for: {}", decision_id),
                success as i32,
                notes,
                Utc::now().to_rfc3339(),
            ],
        )?;
    }

    // 2. Adjust principle confidences (THE KEY PART)
    let mut adjustments = Vec::new();

    // Asymmetric learning: failures hurt more than successes help
    // This implements Taleb's "skin in the game" - bad advice is penalized heavily
    let delta = if success { 0.05 } else { -0.10 };

    for principle_id in applied_principles {
        // Get current confidence
        let current: f64 = conn.query_row(
            "SELECT learned_confidence FROM principles WHERE id = ?1",
            [principle_id],
            |row| row.get(0),
        ).unwrap_or(0.5);

        // Calculate new confidence (clamped to 0.1-0.95)
        let new_confidence = (current + delta).max(0.1).min(0.95);

        // Update principle confidence
        conn.execute(
            "UPDATE principles
             SET learned_confidence = ?2
             WHERE id = ?1",
            params![principle_id, new_confidence],
        )?;

        // Record the adjustment for tracking
        conn.execute(
            "INSERT INTO framework_adjustments
             (principle_id, context_pattern, adjustment, decision_id)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                principle_id,
                context_pattern.unwrap_or("{}"),
                delta,
                decision_id,
            ],
        )?;

        // Update Thompson Sampling parameters
        update_thompson_params(conn, principle_id, success, context_pattern)?;

        // Get principle name for reporting
        let name: String = conn.query_row(
            "SELECT name FROM principles WHERE id = ?1",
            [principle_id],
            |row| row.get(0),
        ).unwrap_or_else(|_| principle_id.to_string());

        adjustments.push(PrincipleAdjustment {
            principle_id: principle_id.clone(),
            principle_name: name,
            old_confidence: current,
            new_confidence,
            delta,
        });
    }

    // 3. Build result
    let new_confidences: Vec<(String, f64)> = adjustments
        .iter()
        .map(|a| (a.principle_id.clone(), a.new_confidence))
        .collect();

    Ok(OutcomeResult {
        decision_id: decision_id.to_string(),
        principles_adjusted: adjustments,
        new_confidences,
    })
}

/// Update Thompson Sampling parameters for a principle
fn update_thompson_params(
    conn: &Connection,
    principle_id: &str,
    success: bool,
    context_pattern: Option<&str>,
) -> Result<()> {
    // Update global Thompson parameters
    let (alpha_delta, beta_delta) = if success { (1.0, 0.0) } else { (0.0, 1.0) };

    conn.execute(
        "INSERT INTO thompson_arms (principle_id, alpha, beta)
         VALUES (?1, ?2, ?3)
         ON CONFLICT(principle_id) DO UPDATE SET
            alpha = alpha + ?2,
            beta = beta + ?3,
            updated_at = CURRENT_TIMESTAMP",
        params![principle_id, alpha_delta, beta_delta],
    )?;

    // Update domain-specific Thompson parameters if context provided
    if let Some(ctx) = context_pattern {
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(ctx) {
            if let Some(domain) = parsed.get("domain").and_then(|d| d.as_str()) {
                conn.execute(
                    "INSERT INTO thompson_domain_arms (principle_id, domain, alpha, beta)
                     VALUES (?1, ?2, ?3, ?4)
                     ON CONFLICT(principle_id, domain) DO UPDATE SET
                        alpha = alpha + ?3,
                        beta = beta + ?4,
                        updated_at = CURRENT_TIMESTAMP",
                    params![principle_id, domain, alpha_delta, beta_delta],
                )?;
            }
        }
    }

    Ok(())
}

/// Batch record outcomes from a bead close
pub fn record_bead_outcome(
    conn: &Connection,
    bead_id: &str,
    bead_title: &str,
    success: bool,
    applied_principles: &[String],
    notes: &str,
    category: Option<&str>,
) -> Result<OutcomeResult> {
    let decision_id = format!("bead-{}", bead_id);

    let context = category.map(|cat| {
        serde_json::json!({
            "bead_id": bead_id,
            "category": cat,
            "domain": category_to_domain(cat),
        }).to_string()
    });

    record_outcome(
        conn,
        &decision_id,
        success,
        applied_principles,
        &format!("{}: {}", bead_title, notes),
        context.as_deref(),
    )
}

/// Map bead categories to 100minds domains
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
        _ => "general",
    }
}

/// Get learning statistics
pub fn get_learning_stats(conn: &Connection) -> Result<LearningStats> {
    let total_outcomes: i64 = conn.query_row(
        "SELECT COUNT(*) FROM decisions WHERE outcome_success IS NOT NULL",
        [],
        |row| row.get(0),
    )?;

    let successful_outcomes: i64 = conn.query_row(
        "SELECT COUNT(*) FROM decisions WHERE outcome_success = 1",
        [],
        |row| row.get(0),
    )?;

    let total_adjustments: i64 = conn.query_row(
        "SELECT COUNT(*) FROM framework_adjustments",
        [],
        |row| row.get(0),
    )?;

    let principles_with_learning: i64 = conn.query_row(
        "SELECT COUNT(DISTINCT principle_id) FROM framework_adjustments",
        [],
        |row| row.get(0),
    )?;

    // Get top improved principles
    let mut stmt = conn.prepare(
        "SELECT p.name,
                p.learned_confidence - p.base_confidence as delta,
                COUNT(fa.id) as adjustment_count
         FROM principles p
         LEFT JOIN framework_adjustments fa ON p.id = fa.principle_id
         GROUP BY p.id
         HAVING adjustment_count > 0
         ORDER BY delta DESC
         LIMIT 5"
    )?;

    let top_improved: Vec<(String, f64, i64)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
        .collect::<Result<Vec<_>, _>>()?;

    // Get top declined principles
    let mut stmt = conn.prepare(
        "SELECT p.name,
                p.learned_confidence - p.base_confidence as delta,
                COUNT(fa.id) as adjustment_count
         FROM principles p
         LEFT JOIN framework_adjustments fa ON p.id = fa.principle_id
         GROUP BY p.id
         HAVING adjustment_count > 0
         ORDER BY delta ASC
         LIMIT 5"
    )?;

    let top_declined: Vec<(String, f64, i64)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(LearningStats {
        total_outcomes,
        successful_outcomes,
        success_rate: if total_outcomes > 0 {
            successful_outcomes as f64 / total_outcomes as f64
        } else {
            0.0
        },
        total_adjustments,
        principles_with_learning,
        top_improved,
        top_declined,
    })
}

/// Learning statistics summary
#[derive(Debug, Clone, Serialize)]
pub struct LearningStats {
    pub total_outcomes: i64,
    pub successful_outcomes: i64,
    pub success_rate: f64,
    pub total_adjustments: i64,
    pub principles_with_learning: i64,
    pub top_improved: Vec<(String, f64, i64)>,
    pub top_declined: Vec<(String, f64, i64)>,
}

/// Print learning statistics in a human-readable format
pub fn print_learning_stats(stats: &LearningStats) {
    println!("\n┌─────────────────────────────────────────────────────────────┐");
    println!("│ 🔄 LEARNING FLYWHEEL STATUS                                 │");
    println!("└─────────────────────────────────────────────────────────────┘\n");

    if stats.total_outcomes == 0 {
        println!("⚠️  FLYWHEEL NOT ACTIVATED");
        println!("   No outcomes recorded yet. Record outcomes to start learning:");
        println!("   cargo run --bin 100minds -- --outcome <decision-id> --success --principles \"id1,id2\"");
        return;
    }

    println!("OUTCOMES:");
    println!("   Total: {}", stats.total_outcomes);
    println!("   Successful: {} ({:.1}%)",
             stats.successful_outcomes,
             stats.success_rate * 100.0);
    println!();

    println!("LEARNING:");
    println!("   Adjustments made: {}", stats.total_adjustments);
    println!("   Principles learning: {}", stats.principles_with_learning);
    println!();

    if !stats.top_improved.is_empty() {
        println!("📈 TOP IMPROVED PRINCIPLES:");
        for (name, delta, count) in &stats.top_improved {
            let bar = if *delta > 0.0 { "+" } else { "" };
            println!("   {}{:.2} ({} adjustments) - {}", bar, delta, count, name);
        }
        println!();
    }

    if !stats.top_declined.is_empty() {
        println!("📉 PRINCIPLES NEEDING REVIEW:");
        for (name, delta, count) in &stats.top_declined {
            println!("   {:.2} ({} adjustments) - {}", delta, count, name);
        }
        println!();
    }
}

// ============================================================================
// SWARM INTEGRATION - Distributed learning synchronization
// ============================================================================

use crate::types::{SyncPosteriorsResponse, PrinciplePosterior, RecordOutcomeRequest};
use std::collections::HashMap;

/// Sync Thompson posteriors for distributed swarm learning
///
/// Returns all posteriors, optionally filtered by timestamp
pub fn sync_posteriors(
    conn: &Connection,
    since_ts: Option<i64>,
    domain: Option<&str>,
) -> Result<SyncPosteriorsResponse> {
    // Initialize schema if needed
    init_thompson_schema(conn)?;

    let mut posteriors: HashMap<String, PrinciplePosterior> = HashMap::new();
    let mut domains: HashMap<String, HashMap<String, PrinciplePosterior>> = HashMap::new();

    // Get global posteriors
    let global_query = if since_ts.is_some() {
        "SELECT principle_id, alpha, beta, pulls FROM thompson_arms
         WHERE strftime('%s', updated_at) > ?"
    } else {
        "SELECT principle_id, alpha, beta, pulls FROM thompson_arms"
    };

    let mut stmt = conn.prepare(global_query)?;
    let rows = if let Some(ts) = since_ts {
        stmt.query([ts.to_string()])?
    } else {
        stmt.query([])?
    };

    let mut rows = rows;
    while let Some(row) = rows.next()? {
        let principle_id: String = row.get(0)?;
        let alpha: f64 = row.get(1)?;
        let beta: f64 = row.get(2)?;
        let pulls: u32 = row.get::<_, i64>(3)? as u32;

        posteriors.insert(principle_id, PrinciplePosterior { alpha, beta, pulls });
    }

    // Get domain-specific posteriors
    let domain_query = if let Some(d) = domain {
        format!(
            "SELECT principle_id, domain, alpha, beta FROM thompson_domain_arms WHERE domain = '{}'",
            d.replace('\'', "''")
        )
    } else {
        "SELECT principle_id, domain, alpha, beta FROM thompson_domain_arms".to_string()
    };

    let mut stmt = conn.prepare(&domain_query)?;
    let mut rows = stmt.query([])?;

    while let Some(row) = rows.next()? {
        let principle_id: String = row.get(0)?;
        let domain_name: String = row.get(1)?;
        let alpha: f64 = row.get(2)?;
        let beta: f64 = row.get(3)?;

        let domain_map = domains.entry(domain_name).or_default();
        domain_map.insert(principle_id, PrinciplePosterior {
            alpha,
            beta,
            pulls: 0, // Domain arms don't track pulls separately
        });
    }

    // Get last updated timestamp
    let last_updated: i64 = conn.query_row(
        "SELECT COALESCE(MAX(strftime('%s', updated_at)), 0) FROM thompson_arms",
        [],
        |row| row.get(0),
    ).unwrap_or(0);

    Ok(SyncPosteriorsResponse {
        posteriors,
        domains,
        last_updated,
    })
}

/// Record outcomes in batch (for worker catch-up sync)
pub fn record_outcomes_batch(
    conn: &Connection,
    outcomes: &[RecordOutcomeRequest],
) -> Result<Vec<OutcomeResult>> {
    let mut results = Vec::new();

    for outcome in outcomes {
        let context = outcome.domain.as_ref().map(|d| {
            serde_json::json!({
                "domain": d,
                "confidence_score": outcome.confidence_score,
                "failure_stage": outcome.failure_stage,
            }).to_string()
        });

        let result = record_outcome(
            conn,
            &outcome.decision_id,
            outcome.success,
            &outcome.principle_ids,
            outcome.notes.as_deref().unwrap_or(""),
            context.as_deref(),
        )?;

        results.push(result);
    }

    Ok(results)
}

/// Enhanced record outcome with swarm fields
pub fn record_outcome_v2(
    conn: &Connection,
    request: &RecordOutcomeRequest,
) -> Result<OutcomeResult> {
    // Build context with swarm-specific fields
    let context = serde_json::json!({
        "domain": request.domain,
        "confidence_score": request.confidence_score,
        "failure_stage": request.failure_stage,
    });

    record_outcome(
        conn,
        &request.decision_id,
        request.success,
        &request.principle_ids,
        request.notes.as_deref().unwrap_or(""),
        Some(&context.to_string()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_category_to_domain() {
        assert_eq!(category_to_domain("[SWARM-FIX]"), "software-design");
        assert_eq!(category_to_domain("[CI-RETRY]"), "quality");
        assert_eq!(category_to_domain("[AUDIT]"), "entrepreneurship");
        assert_eq!(category_to_domain("[FEATURE]"), "systems-thinking");
    }

    #[test]
    fn test_sync_posteriors_empty() {
        let conn = Connection::open_in_memory().unwrap();
        init_thompson_schema(&conn).unwrap();

        let result = sync_posteriors(&conn, None, None).unwrap();
        assert!(result.posteriors.is_empty());
        assert!(result.domains.is_empty());
    }
}
