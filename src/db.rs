//! Database layer for 100minds wisdom storage
//!
//! Uses SQLite with FTS5 for full-text search on principles.
//! Designed for simplicity (Dijkstra) and antifragility (Taleb) -
//! single file, zero network dependencies, works offline.

use anyhow::{Context, Result};
use rusqlite::{Connection, params};
use std::path::Path;

/// Initialize the database with schema
pub fn init_db(path: &Path) -> Result<Connection> {
    let conn = Connection::open(path)
        .with_context(|| format!("Failed to open database at {:?}", path))?;

    conn.execute_batch(SCHEMA)?;

    Ok(conn)
}

const SCHEMA: &str = r#"
-- Thinkers: The 100 minds
CREATE TABLE IF NOT EXISTS thinkers (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    domain TEXT NOT NULL,
    background TEXT,
    profile_json TEXT,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP
);

-- Principles: Structured frameworks from each thinker
CREATE TABLE IF NOT EXISTS principles (
    id TEXT PRIMARY KEY,
    thinker_id TEXT NOT NULL REFERENCES thinkers(id),
    name TEXT NOT NULL,
    description TEXT NOT NULL,
    domain_tags TEXT,           -- JSON array of domains
    application_rule TEXT,
    anti_pattern TEXT,
    falsification TEXT,         -- How to know this principle is wrong
    base_confidence REAL DEFAULT 0.5,
    learned_confidence REAL DEFAULT 0.5,
    UNIQUE(thinker_id, name)
);

-- FTS5 index for fast principle search
CREATE VIRTUAL TABLE IF NOT EXISTS principles_fts USING fts5(
    name,
    description,
    application_rule,
    content=principles,
    content_rowid=rowid
);

-- Triggers to keep FTS in sync
CREATE TRIGGER IF NOT EXISTS principles_ai AFTER INSERT ON principles BEGIN
    INSERT INTO principles_fts(rowid, name, description, application_rule)
    VALUES (new.rowid, new.name, new.description, new.application_rule);
END;

CREATE TRIGGER IF NOT EXISTS principles_ad AFTER DELETE ON principles BEGIN
    INSERT INTO principles_fts(principles_fts, rowid, name, description, application_rule)
    VALUES ('delete', old.rowid, old.name, old.description, old.application_rule);
END;

CREATE TRIGGER IF NOT EXISTS principles_au AFTER UPDATE ON principles BEGIN
    INSERT INTO principles_fts(principles_fts, rowid, name, description, application_rule)
    VALUES ('delete', old.rowid, old.name, old.description, old.application_rule);
    INSERT INTO principles_fts(rowid, name, description, application_rule)
    VALUES (new.rowid, new.name, new.description, new.application_rule);
END;

-- Decisions: The provenance chain
CREATE TABLE IF NOT EXISTS decisions (
    id TEXT PRIMARY KEY,
    question TEXT NOT NULL,
    context_json TEXT,
    counsel_json TEXT NOT NULL,

    -- Provenance chain
    previous_hash TEXT,
    content_hash TEXT NOT NULL,
    signature TEXT NOT NULL,
    agent_pubkey TEXT NOT NULL,

    -- Outcome tracking
    outcome_success INTEGER,    -- NULL = unknown, 0 = failed, 1 = succeeded
    outcome_notes TEXT,
    outcome_recorded_at TEXT,

    created_at TEXT DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_decisions_hash ON decisions(content_hash);
CREATE INDEX IF NOT EXISTS idx_decisions_created ON decisions(created_at);

-- Framework adjustments: What we've learned from outcomes
CREATE TABLE IF NOT EXISTS framework_adjustments (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    principle_id TEXT NOT NULL REFERENCES principles(id),
    context_pattern TEXT,       -- JSON pattern that matched
    adjustment REAL NOT NULL,   -- Confidence delta (+/-)
    decision_id TEXT REFERENCES decisions(id),
    created_at TEXT DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_adjustments_principle ON framework_adjustments(principle_id);

-- Synthesis cache: Pre-computed multi-thinker combinations
CREATE TABLE IF NOT EXISTS synthesis_cache (
    id TEXT PRIMARY KEY,
    thinker_ids TEXT NOT NULL,  -- JSON array of thinker IDs
    question_hash TEXT NOT NULL,
    synthesis_json TEXT NOT NULL,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_synthesis_thinkers ON synthesis_cache(thinker_ids);

-- Archived principles: Culled poor performers (preserved for analysis)
CREATE TABLE IF NOT EXISTS archived_principles (
    id TEXT PRIMARY KEY,
    thinker_id TEXT NOT NULL,
    name TEXT NOT NULL,
    description TEXT NOT NULL,
    domain_tags TEXT,
    application_rule TEXT,
    anti_pattern TEXT,
    falsification TEXT,
    base_confidence REAL,
    learned_confidence REAL,
    archived_at TEXT DEFAULT CURRENT_TIMESTAMP,
    cull_reason TEXT DEFAULT 'low_confidence'
);

-- Success column for framework_adjustments (if not exists)
-- ALTER TABLE framework_adjustments ADD COLUMN success INTEGER;

-- Contextual Thompson Sampling: Track principle success per domain
-- This enables context-aware learning (e.g., YAGNI works for features but not security)
CREATE TABLE IF NOT EXISTS contextual_arms (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    principle_id TEXT NOT NULL REFERENCES principles(id),
    domain TEXT NOT NULL,           -- e.g., "architecture", "testing", "security"
    alpha REAL DEFAULT 1.0,         -- Success count + 1 (Beta prior)
    beta REAL DEFAULT 1.0,          -- Failure count + 1 (Beta prior)
    sample_count INTEGER DEFAULT 0,
    last_updated TEXT DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(principle_id, domain)
);

CREATE INDEX IF NOT EXISTS idx_contextual_arms_principle ON contextual_arms(principle_id);
CREATE INDEX IF NOT EXISTS idx_contextual_arms_domain ON contextual_arms(domain);

-- Query reformulations: Cache successful query expansions
CREATE TABLE IF NOT EXISTS query_expansions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    original_query TEXT NOT NULL,
    expanded_query TEXT NOT NULL,
    success_rate REAL DEFAULT 0.5,
    sample_count INTEGER DEFAULT 0,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_query_expansions_original ON query_expansions(original_query);

-- Hard negatives: Track principle-question pairs that FAILED
-- Used for contrastive learning and negative mining
CREATE TABLE IF NOT EXISTS hard_negatives (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    question_hash TEXT NOT NULL,
    principle_id TEXT NOT NULL REFERENCES principles(id),
    failure_count INTEGER DEFAULT 1,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(question_hash, principle_id)
);

CREATE INDEX IF NOT EXISTS idx_hard_negatives_question ON hard_negatives(question_hash);
"#;

/// Get the latest decision hash for chain linking
pub fn get_latest_decision_hash(conn: &Connection) -> Result<Option<String>> {
    let mut stmt = conn.prepare(
        "SELECT content_hash FROM decisions ORDER BY created_at DESC LIMIT 1"
    )?;

    let hash: Option<String> = stmt.query_row([], |row| row.get(0)).ok();
    Ok(hash)
}

/// Search principles by query using FTS5, with LIKE fallback
pub fn search_principles(conn: &Connection, query: &str, limit: usize) -> Result<Vec<PrincipleMatch>> {
    // Extract keywords from query (alphanumeric words only)
    // Take more keywords to support expanded queries
    let keywords: Vec<&str> = query
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() > 2)
        .take(15)  // Increased to support semantic expansion
        .collect();

    if keywords.is_empty() {
        return Ok(Vec::new());
    }

    // Try FTS5 first with simple keyword OR
    let fts_query = keywords.join(" OR ");
    let fts_result = conn.prepare(
        r#"
        SELECT p.id, p.thinker_id, p.name, p.description, p.learned_confidence,
               bm25(principles_fts) as score
        FROM principles_fts
        JOIN principles p ON principles_fts.rowid = p.rowid
        WHERE principles_fts MATCH ?1
        ORDER BY score
        LIMIT ?2
        "#
    );

    if let Ok(mut stmt) = fts_result {
        if let Ok(matches) = stmt.query_map(params![fts_query, limit as i64], |row| {
            Ok(PrincipleMatch {
                id: row.get(0)?,
                thinker_id: row.get(1)?,
                name: row.get(2)?,
                description: row.get(3)?,
                confidence: row.get(4)?,
                relevance_score: row.get(5)?,
            })
        }) {
            let results: Vec<_> = matches.filter_map(|r| r.ok()).collect();
            if !results.is_empty() {
                return Ok(results);
            }
        }
    }

    // Fallback: LIKE search on principles table
    let like_pattern = format!("%{}%", keywords.join("%"));
    let mut stmt = conn.prepare(
        r#"
        SELECT id, thinker_id, name, description, learned_confidence, 0.5 as score
        FROM principles
        WHERE name LIKE ?1 OR description LIKE ?1
        ORDER BY learned_confidence DESC
        LIMIT ?2
        "#
    )?;

    let matches = stmt.query_map(params![like_pattern, limit as i64], |row| {
        Ok(PrincipleMatch {
            id: row.get(0)?,
            thinker_id: row.get(1)?,
            name: row.get(2)?,
            description: row.get(3)?,
            confidence: row.get(4)?,
            relevance_score: row.get(5)?,
        })
    })?
    .collect::<Result<Vec<_>, _>>()?;

    Ok(matches)
}

#[derive(Debug, Clone)]
pub struct PrincipleMatch {
    pub id: String,
    pub thinker_id: String,
    pub name: String,
    pub description: String,
    pub confidence: f64,
    pub relevance_score: f64,
}

/// Get principles by domain
pub fn get_principles_by_domain(conn: &Connection, domain: &str) -> Result<Vec<PrincipleMatch>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT p.id, p.thinker_id, p.name, p.description, p.learned_confidence, 0.0
        FROM principles p
        WHERE p.domain_tags LIKE ?1
        ORDER BY p.learned_confidence DESC
        "#
    )?;

    let pattern = format!("%\"{}\"%" , domain);
    let matches = stmt.query_map([pattern], |row| {
        Ok(PrincipleMatch {
            id: row.get(0)?,
            thinker_id: row.get(1)?,
            name: row.get(2)?,
            description: row.get(3)?,
            confidence: row.get(4)?,
            relevance_score: row.get(5)?,
        })
    })?
    .collect::<Result<Vec<_>, _>>()?;

    Ok(matches)
}

/// Record a decision
pub fn insert_decision(
    conn: &Connection,
    id: &str,
    question: &str,
    context_json: Option<&str>,
    counsel_json: &str,
    previous_hash: Option<&str>,
    content_hash: &str,
    signature: &str,
    agent_pubkey: &str,
) -> Result<()> {
    conn.execute(
        r#"
        INSERT INTO decisions (id, question, context_json, counsel_json,
                               previous_hash, content_hash, signature, agent_pubkey)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
        "#,
        params![id, question, context_json, counsel_json,
                previous_hash, content_hash, signature, agent_pubkey],
    )?;
    Ok(())
}

/// Record an outcome for a decision
pub fn record_outcome(
    conn: &Connection,
    decision_id: &str,
    success: bool,
    notes: Option<&str>,
) -> Result<()> {
    conn.execute(
        r#"
        UPDATE decisions
        SET outcome_success = ?2, outcome_notes = ?3, outcome_recorded_at = CURRENT_TIMESTAMP
        WHERE id = ?1
        "#,
        params![decision_id, success as i32, notes],
    )?;
    Ok(())
}

/// Apply a confidence adjustment to a principle
pub fn apply_adjustment(
    conn: &Connection,
    principle_id: &str,
    context_pattern: Option<&str>,
    adjustment: f64,
    decision_id: &str,
) -> Result<()> {
    // Record the adjustment
    conn.execute(
        r#"
        INSERT INTO framework_adjustments (principle_id, context_pattern, adjustment, decision_id)
        VALUES (?1, ?2, ?3, ?4)
        "#,
        params![principle_id, context_pattern, adjustment, decision_id],
    )?;

    // Update the learned confidence (bounded 0.0 - 1.0)
    conn.execute(
        r#"
        UPDATE principles
        SET learned_confidence = MIN(1.0, MAX(0.0, learned_confidence + ?2))
        WHERE id = ?1
        "#,
        params![principle_id, adjustment],
    )?;

    Ok(())
}

/// Update contextual Thompson Sampling arm for a principle in a domain
/// This enables domain-specific learning (e.g., YAGNI great for features, bad for security)
pub fn update_contextual_arm(
    conn: &Connection,
    principle_id: &str,
    domain: &str,
    success: bool,
) -> Result<()> {
    // Upsert the contextual arm
    conn.execute(
        r#"
        INSERT INTO contextual_arms (principle_id, domain, alpha, beta, sample_count)
        VALUES (?1, ?2, ?3, ?4, 1)
        ON CONFLICT(principle_id, domain) DO UPDATE SET
            alpha = alpha + ?3 - 1,
            beta = beta + ?4 - 1,
            sample_count = sample_count + 1,
            last_updated = CURRENT_TIMESTAMP
        "#,
        params![
            principle_id,
            domain,
            if success { 2.0 } else { 1.0 },  // Add 1 to alpha on success
            if success { 1.0 } else { 2.0 },  // Add 1 to beta on failure
        ],
    )?;

    Ok(())
}

/// Get contextual confidence for a principle in a domain (Thompson Sampling mean)
pub fn get_contextual_confidence(
    conn: &Connection,
    principle_id: &str,
    domain: &str,
) -> Result<Option<f64>> {
    let result: Option<(f64, f64)> = conn.query_row(
        "SELECT alpha, beta FROM contextual_arms WHERE principle_id = ?1 AND domain = ?2",
        params![principle_id, domain],
        |row| Ok((row.get(0)?, row.get(1)?)),
    ).ok();

    Ok(result.map(|(alpha, beta)| alpha / (alpha + beta)))
}

/// Sample from contextual Thompson Sampling distribution
/// Returns a sampled confidence value for exploration/exploitation
pub fn sample_contextual_arm(
    conn: &Connection,
    principle_id: &str,
    domain: &str,
) -> Result<f64> {
    let (alpha, beta): (f64, f64) = conn.query_row(
        "SELECT alpha, beta FROM contextual_arms WHERE principle_id = ?1 AND domain = ?2",
        params![principle_id, domain],
        |row| Ok((row.get(0)?, row.get(1)?)),
    ).unwrap_or((1.0, 1.0));  // Default prior: Beta(1,1) = uniform

    // Simple approximation of Beta sampling using the mean + variance
    // For production, use a proper Beta distribution sampler
    let mean = alpha / (alpha + beta);
    let variance = (alpha * beta) / ((alpha + beta).powi(2) * (alpha + beta + 1.0));
    let std_dev = variance.sqrt();

    // Use mean + random noise for exploration (simplified Thompson Sampling)
    // In production, sample from actual Beta distribution
    use std::time::{SystemTime, UNIX_EPOCH};
    let seed = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().subsec_nanos();
    let random_factor = ((seed % 1000) as f64 / 1000.0 - 0.5) * 2.0;  // [-1, 1]

    Ok((mean + random_factor * std_dev).max(0.0).min(1.0))
}

/// Record a hard negative (principle that failed for a question)
pub fn record_hard_negative(
    conn: &Connection,
    question_hash: &str,
    principle_id: &str,
) -> Result<()> {
    conn.execute(
        r#"
        INSERT INTO hard_negatives (question_hash, principle_id, failure_count)
        VALUES (?1, ?2, 1)
        ON CONFLICT(question_hash, principle_id) DO UPDATE SET
            failure_count = failure_count + 1
        "#,
        params![question_hash, principle_id],
    )?;

    Ok(())
}

/// Check if a principle is a known hard negative for a question type
pub fn is_hard_negative(
    conn: &Connection,
    question_hash: &str,
    principle_id: &str,
) -> Result<bool> {
    let count: i64 = conn.query_row(
        "SELECT failure_count FROM hard_negatives WHERE question_hash = ?1 AND principle_id = ?2",
        params![question_hash, principle_id],
        |row| row.get(0),
    ).unwrap_or(0);

    Ok(count >= 3)  // Consider it a hard negative if failed 3+ times
}

/// Get domain-specific confidence boost using contextual arms
/// Falls back to global learned_confidence if no domain data
pub fn get_domain_boosted_confidence(
    conn: &Connection,
    principle_id: &str,
    domains: &[&str],
) -> Result<f64> {
    // Get base confidence
    let base: f64 = conn.query_row(
        "SELECT learned_confidence FROM principles WHERE id = ?1",
        [principle_id],
        |row| row.get(0),
    )?;

    // Check contextual confidence for each domain
    let mut domain_scores: Vec<f64> = Vec::new();
    for domain in domains {
        if let Some(ctx_conf) = get_contextual_confidence(conn, principle_id, domain)? {
            domain_scores.push(ctx_conf);
        }
    }

    if domain_scores.is_empty() {
        Ok(base)
    } else {
        // Weight: 60% base, 40% domain average
        let domain_avg = domain_scores.iter().sum::<f64>() / domain_scores.len() as f64;
        Ok(base * 0.6 + domain_avg * 0.4)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_init_db() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.db");
        let conn = init_db(&path).unwrap();

        // Verify tables exist
        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table'")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        assert!(tables.contains(&"thinkers".to_string()));
        assert!(tables.contains(&"principles".to_string()));
        assert!(tables.contains(&"decisions".to_string()));
    }
}
