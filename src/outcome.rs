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
use chrono::Utc;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

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
        params![decision_id, success as i32, notes, Utc::now().to_rfc3339(),],
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
        let current: f64 = conn
            .query_row(
                "SELECT learned_confidence FROM principles WHERE id = ?1",
                [principle_id],
                |row| row.get(0),
            )
            .unwrap_or(0.5);

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
        let name: String = conn
            .query_row(
                "SELECT name FROM principles WHERE id = ?1",
                [principle_id],
                |row| row.get(0),
            )
            .unwrap_or_else(|_| principle_id.to_string());

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
        })
        .to_string()
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

    let total_adjustments: i64 =
        conn.query_row("SELECT COUNT(*) FROM framework_adjustments", [], |row| {
            row.get(0)
        })?;

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
         LIMIT 5",
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
         LIMIT 5",
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
    println!(
        "   Successful: {} ({:.1}%)",
        stats.successful_outcomes,
        stats.success_rate * 100.0
    );
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

use crate::types::{PrinciplePosterior, RecordOutcomeRequest, SyncPosteriorsResponse};
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
        domain_map.insert(
            principle_id,
            PrinciplePosterior {
                alpha,
                beta,
                pulls: 0, // Domain arms don't track pulls separately
            },
        );
    }

    // Get last updated timestamp
    let last_updated: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(strftime('%s', updated_at)), 0) FROM thompson_arms",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

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
            })
            .to_string()
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
    use crate::db;
    use tempfile::tempdir;

    fn setup_test_db() -> (Connection, tempfile::TempDir) {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let conn = db::init_db(&db_path).unwrap();
        // Initialize Thompson schema
        init_thompson_schema(&conn).unwrap();
        (conn, dir)
    }

    fn insert_test_thinker(conn: &Connection, id: &str, name: &str, domain: &str) {
        conn.execute(
            "INSERT OR IGNORE INTO thinkers (id, name, domain) VALUES (?1, ?2, ?3)",
            [id, name, domain],
        )
        .unwrap();
    }

    fn insert_test_principle(conn: &Connection, id: &str, thinker_id: &str, name: &str) {
        conn.execute(
            "INSERT OR IGNORE INTO principles (id, thinker_id, name, description, learned_confidence)
             VALUES (?1, ?2, ?3, ?4, 0.5)",
            [id, thinker_id, name, "Test description"],
        ).unwrap();
    }

    #[test]
    fn test_category_to_domain() {
        assert_eq!(category_to_domain("[SWARM-FIX]"), "software-design");
        assert_eq!(category_to_domain("[CI-RETRY]"), "quality");
        assert_eq!(category_to_domain("[AUDIT]"), "entrepreneurship");
        assert_eq!(category_to_domain("[FEATURE]"), "systems-thinking");
    }

    #[test]
    fn test_category_to_domain_more_categories() {
        assert_eq!(category_to_domain("[SCALE]"), "software-architecture");
        assert_eq!(category_to_domain("[PERF]"), "performance");
        assert_eq!(category_to_domain("[REFACTOR]"), "software-design");
        assert_eq!(category_to_domain("[HEALING]"), "resilience");
        assert_eq!(category_to_domain("[UNKNOWN]"), "general");
    }

    #[test]
    fn test_sync_posteriors_empty() {
        let conn = Connection::open_in_memory().unwrap();
        init_thompson_schema(&conn).unwrap();

        let result = sync_posteriors(&conn, None, None).unwrap();
        assert!(result.posteriors.is_empty());
        assert!(result.domains.is_empty());
    }

    #[test]
    fn test_record_outcome_success() {
        let (conn, _dir) = setup_test_db();
        insert_test_thinker(&conn, "t1", "Thinker", "domain");
        insert_test_principle(&conn, "p1", "t1", "Test Principle");

        // Record success
        let result = record_outcome(
            &conn,
            "test-decision-1",
            true,
            &["p1".to_string()],
            "Success!",
            None,
        )
        .unwrap();

        assert_eq!(result.decision_id, "test-decision-1");
        assert_eq!(result.principles_adjusted.len(), 1);
        assert_eq!(result.principles_adjusted[0].principle_id, "p1");
        // Success adds 0.05
        assert!((result.principles_adjusted[0].delta - 0.05).abs() < 0.001);
        assert!(result.principles_adjusted[0].new_confidence > 0.5);
    }

    #[test]
    fn test_record_outcome_failure_asymmetric() {
        let (conn, _dir) = setup_test_db();
        insert_test_thinker(&conn, "t2", "Thinker", "domain");
        insert_test_principle(&conn, "p2", "t2", "Test Principle");

        // Record failure
        let result = record_outcome(
            &conn,
            "test-decision-2",
            false,
            &["p2".to_string()],
            "Failed!",
            None,
        )
        .unwrap();

        assert_eq!(result.principles_adjusted.len(), 1);
        // Failure subtracts 0.10 (asymmetric - failures hurt more)
        assert!((result.principles_adjusted[0].delta - (-0.10)).abs() < 0.001);
        assert!(result.principles_adjusted[0].new_confidence < 0.5);
    }

    #[test]
    fn test_record_outcome_updates_thompson_params() {
        let (conn, _dir) = setup_test_db();
        insert_test_thinker(&conn, "t3", "Thinker", "domain");
        insert_test_principle(&conn, "p3", "t3", "Test Principle");

        // Record success
        record_outcome(
            &conn,
            "test-decision-3",
            true,
            &["p3".to_string()],
            "Success!",
            None,
        )
        .unwrap();

        // Check Thompson params updated
        let (alpha, beta): (f64, f64) = conn
            .query_row(
                "SELECT alpha, beta FROM thompson_arms WHERE principle_id = ?1",
                ["p3"],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();

        // Fresh insert uses delta values directly: alpha=1.0, beta=0.0 for success
        assert!((alpha - 1.0).abs() < 0.001);
        assert!((beta - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_record_bead_outcome() {
        let (conn, _dir) = setup_test_db();
        insert_test_thinker(&conn, "t4", "Thinker", "domain");
        insert_test_principle(&conn, "p4", "t4", "Test Principle");

        let result = record_bead_outcome(
            &conn,
            "bd-12345",
            "Fix the bug",
            true,
            &["p4".to_string()],
            "Worked!",
            Some("[SWARM-FIX]"),
        )
        .unwrap();

        assert_eq!(result.decision_id, "bead-bd-12345");
        assert_eq!(result.principles_adjusted.len(), 1);
    }

    #[test]
    fn test_get_learning_stats_empty() {
        let (conn, _dir) = setup_test_db();

        let stats = get_learning_stats(&conn).unwrap();
        assert_eq!(stats.total_outcomes, 0);
        assert_eq!(stats.successful_outcomes, 0);
        assert_eq!(stats.success_rate, 0.0);
    }

    #[test]
    fn test_get_learning_stats_with_data() {
        let (conn, _dir) = setup_test_db();
        insert_test_thinker(&conn, "t5", "Thinker", "domain");
        insert_test_principle(&conn, "p5", "t5", "Test Principle");

        // Record 2 successes and 1 failure
        record_outcome(&conn, "d1", true, &["p5".to_string()], "ok", None).unwrap();
        record_outcome(&conn, "d2", true, &["p5".to_string()], "ok", None).unwrap();
        record_outcome(&conn, "d3", false, &["p5".to_string()], "fail", None).unwrap();

        let stats = get_learning_stats(&conn).unwrap();
        assert_eq!(stats.total_outcomes, 3);
        assert_eq!(stats.successful_outcomes, 2);
        assert!((stats.success_rate - 0.666).abs() < 0.01);
        assert_eq!(stats.principles_with_learning, 1);
    }

    #[test]
    fn test_sync_posteriors_with_data() {
        let (conn, _dir) = setup_test_db();
        insert_test_thinker(&conn, "t6", "Thinker", "domain");
        insert_test_principle(&conn, "p6", "t6", "Test Principle");

        // Record outcome to populate Thompson params
        record_outcome(&conn, "d4", true, &["p6".to_string()], "ok", None).unwrap();

        let result = sync_posteriors(&conn, None, None).unwrap();
        assert!(
            !result.posteriors.is_empty(),
            "Should have posteriors after recording outcome"
        );
        assert!(
            result.posteriors.contains_key("p6"),
            "Should contain principle p6"
        );
        // last_updated may be 0 if timestamp parsing fails - just check posteriors work
    }

    #[test]
    fn test_record_outcomes_batch() {
        let (conn, _dir) = setup_test_db();
        insert_test_thinker(&conn, "t7", "Thinker", "domain");
        insert_test_principle(&conn, "p7", "t7", "Test Principle");

        let outcomes = vec![
            RecordOutcomeRequest {
                decision_id: "batch-1".to_string(),
                success: true,
                principle_ids: vec!["p7".to_string()],
                notes: Some("batch ok".to_string()),
                domain: Some("testing".to_string()),
                confidence_score: None,
                failure_stage: None,
            },
            RecordOutcomeRequest {
                decision_id: "batch-2".to_string(),
                success: false,
                principle_ids: vec!["p7".to_string()],
                notes: Some("batch fail".to_string()),
                domain: Some("testing".to_string()),
                confidence_score: None,
                failure_stage: None,
            },
        ];

        let results = record_outcomes_batch(&conn, &outcomes).unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].decision_id, "batch-1");
        assert_eq!(results[1].decision_id, "batch-2");
    }

    #[test]
    fn test_record_outcome_confidence_bounds() {
        let (conn, _dir) = setup_test_db();
        insert_test_thinker(&conn, "t8", "Thinker", "domain");

        // Insert principle with very low confidence
        conn.execute(
            "INSERT INTO principles (id, thinker_id, name, description, learned_confidence)
             VALUES ('p8', 't8', 'P', 'D', 0.15)",
            [],
        )
        .unwrap();

        // Record failure - should be bounded at 0.1
        let result = record_outcome(
            &conn,
            "test-bounds",
            false,
            &["p8".to_string()],
            "Failed!",
            None,
        )
        .unwrap();

        // New confidence should be bounded at 0.1 (not go below)
        assert!(result.principles_adjusted[0].new_confidence >= 0.1);
    }

    #[test]
    fn test_record_outcome_with_domain_context() {
        let (conn, _dir) = setup_test_db();
        insert_test_thinker(&conn, "t9", "Thinker", "domain");
        insert_test_principle(&conn, "p9", "t9", "Test Principle");

        let context = serde_json::json!({
            "domain": "software-design"
        })
        .to_string();

        record_outcome(
            &conn,
            "test-domain",
            true,
            &["p9".to_string()],
            "Success!",
            Some(&context),
        )
        .unwrap();

        // Check domain-specific Thompson params were created
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM thompson_domain_arms WHERE principle_id = ?1 AND domain = ?2",
                params!["p9", "software-design"],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(count, 1, "Domain-specific Thompson arm should exist");
    }
}
