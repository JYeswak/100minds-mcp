//! Scenario Benchmark Framework
//!
//! Test 100minds against scenarios with known ground truth.
//! Measures precision, recall, NDCG, and anti-principle detection.

use crate::counsel::CounselEngine;
use crate::provenance::Provenance;
use crate::types::*;
use super::EvalMetrics;
use anyhow::Result;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::time::Instant;

/// A single test scenario with ground truth
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioCase {
    /// Unique identifier
    pub id: String,

    /// Category (e.g., "architecture", "scaling", "performance")
    pub category: String,

    /// The decision question to ask
    pub question: String,

    /// Additional context for the question
    #[serde(default)]
    pub context: HashMap<String, String>,

    /// Ground truth: principles that SHOULD be cited
    pub expected_principles: Vec<String>,

    /// Ground truth: thinkers who SHOULD be cited
    pub expected_thinkers: Vec<String>,

    /// Anti-principles: principles that would be WRONG to cite
    #[serde(default)]
    pub anti_principles: Vec<String>,

    /// Difficulty level (1-5)
    #[serde(default = "default_difficulty")]
    pub difficulty: u8,
}

fn default_difficulty() -> u8 { 3 }

/// Results from running all scenarios
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioResults {
    pub total_scenarios: usize,
    pub aggregate: EvalMetrics,
    pub by_category: HashMap<String, EvalMetrics>,
    pub individual: Vec<IndividualResult>,
    pub worst_performers: Vec<IndividualResult>,
}

/// Result for a single scenario
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndividualResult {
    pub scenario_id: String,
    pub category: String,
    pub question: String,
    pub metrics: EvalMetrics,
    pub principles_cited: Vec<String>,
    pub thinkers_cited: Vec<String>,
    pub anti_principles_cited: Vec<String>,
    pub missing_expected: Vec<String>,
}

/// Load scenarios from a JSON file
pub fn load_scenarios(path: &Path) -> Result<Vec<ScenarioCase>> {
    let content = std::fs::read_to_string(path)?;
    let scenarios: Vec<ScenarioCase> = serde_json::from_str(&content)?;
    Ok(scenarios)
}

/// Load all scenarios from a directory
pub fn load_all_scenarios(dir: &Path) -> Result<Vec<ScenarioCase>> {
    let mut all_scenarios = Vec::new();

    if dir.is_dir() {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().map_or(false, |e| e == "json") {
                match load_scenarios(&path) {
                    Ok(scenarios) => all_scenarios.extend(scenarios),
                    Err(e) => eprintln!("Warning: Failed to load {:?}: {}", path, e),
                }
            }
        }
    }

    Ok(all_scenarios)
}

/// Run benchmark on all scenarios
pub fn run_benchmark(
    conn: &Connection,
    provenance: &Provenance,
    scenarios: &[ScenarioCase],
) -> Result<ScenarioResults> {
    let engine = CounselEngine::new(conn, provenance);
    let mut individual_results = Vec::new();
    let mut by_category: HashMap<String, Vec<EvalMetrics>> = HashMap::new();

    for scenario in scenarios {
        let result = run_single_scenario(conn, &engine, scenario)?;

        by_category
            .entry(scenario.category.clone())
            .or_default()
            .push(result.metrics.clone());

        individual_results.push(result);
    }

    // Aggregate metrics
    let all_metrics: Vec<_> = individual_results.iter().map(|r| &r.metrics).collect();
    let aggregate = aggregate_metrics(&all_metrics);

    // Aggregate by category
    let category_metrics: HashMap<String, EvalMetrics> = by_category
        .into_iter()
        .map(|(cat, metrics)| {
            let refs: Vec<_> = metrics.iter().collect();
            (cat, aggregate_metrics(&refs))
        })
        .collect();

    // Find worst performers (lowest P@3)
    let mut sorted = individual_results.clone();
    sorted.sort_by(|a, b| {
        let a_p3 = a.metrics.precision_at_k.get(&3).unwrap_or(&0.0);
        let b_p3 = b.metrics.precision_at_k.get(&3).unwrap_or(&0.0);
        a_p3.partial_cmp(b_p3).unwrap()
    });
    let worst_performers: Vec<_> = sorted.into_iter().take(10).collect();

    Ok(ScenarioResults {
        total_scenarios: scenarios.len(),
        aggregate,
        by_category: category_metrics,
        individual: individual_results,
        worst_performers,
    })
}

/// Build a lookup table of principle ID -> name
fn build_principle_name_lookup(conn: &Connection) -> HashMap<String, String> {
    let mut lookup = HashMap::new();

    if let Ok(mut stmt) = conn.prepare("SELECT id, name FROM principles") {
        if let Ok(rows) = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        }) {
            for row in rows.flatten() {
                lookup.insert(row.0.to_lowercase(), row.1.to_lowercase());
            }
        }
    }

    lookup
}

/// Run a single scenario and compute metrics
fn run_single_scenario(
    conn: &Connection,
    engine: &CounselEngine,
    scenario: &ScenarioCase,
) -> Result<IndividualResult> {
    let start = Instant::now();

    // Build principle ID -> name lookup
    let id_to_name = build_principle_name_lookup(conn);

    // Build counsel request
    let request = CounselRequest {
        question: scenario.question.clone(),
        context: CounselContext {
            domain: scenario.context.get("domain").cloned(),
            constraints: scenario.context.get("constraints")
                .map(|c| c.split(',').map(|s| s.trim().to_string()).collect())
                .unwrap_or_default(),
            prefer_thinkers: vec![],
            depth: CounselDepth::Standard,
        },
    };

    // Get counsel
    let response = engine.counsel(&request)?;
    let latency = start.elapsed().as_millis() as u64;

    // Extract cited principles and thinkers
    // Convert principle IDs to names for comparison against scenarios
    // IMPORTANT: Preserve order for P@K calculation (positions are ranked by relevance)
    let mut principles_ordered: Vec<String> = Vec::new();
    let mut principles_seen: HashSet<String> = HashSet::new();
    let mut thinkers_cited: HashSet<String> = HashSet::new();

    for position in &response.positions {
        for principle_id in &position.principles_cited {
            // Look up the principle name from ID
            let id_lower = principle_id.to_lowercase();
            let name = if let Some(n) = id_to_name.get(&id_lower) {
                n.clone()
            } else {
                // Fallback: use the ID itself (it may be descriptive enough)
                id_lower
            };

            // Only add if not seen (preserve first occurrence order)
            if !principles_seen.contains(&name) {
                principles_seen.insert(name.clone());
                principles_ordered.push(name);
            }
        }
        thinkers_cited.insert(position.thinker.to_lowercase());
    }

    let principles_vec = principles_ordered;  // Already ordered by position relevance
    let thinkers_vec: Vec<String> = thinkers_cited.iter().cloned().collect();

    // Check for anti-principles
    let expected_lower: HashSet<String> = scenario.expected_principles
        .iter()
        .map(|p| p.to_lowercase())
        .collect();

    let anti_lower: HashSet<String> = scenario.anti_principles
        .iter()
        .map(|p| p.to_lowercase())
        .collect();

    let anti_cited: Vec<String> = principles_seen
        .intersection(&anti_lower)
        .cloned()
        .collect();

    let missing: Vec<String> = expected_lower
        .difference(&principles_seen)
        .cloned()
        .collect();

    // Compute metrics
    let metrics = compute_metrics(
        &principles_vec,
        &scenario.expected_principles,
        &scenario.anti_principles,
        &thinkers_vec,
        latency,
    );

    Ok(IndividualResult {
        scenario_id: scenario.id.clone(),
        category: scenario.category.clone(),
        question: scenario.question.clone(),
        metrics,
        principles_cited: principles_vec,
        thinkers_cited: thinkers_vec,
        anti_principles_cited: anti_cited,
        missing_expected: missing,
    })
}

/// Compute evaluation metrics for a single result
fn compute_metrics(
    cited: &[String],
    expected: &[String],
    anti: &[String],
    thinkers: &[String],
    latency_ms: u64,
) -> EvalMetrics {
    let cited_lower: HashSet<String> = cited.iter().map(|s| s.to_lowercase()).collect();
    let expected_lower: HashSet<String> = expected.iter().map(|s| s.to_lowercase()).collect();
    let anti_lower: HashSet<String> = anti.iter().map(|s| s.to_lowercase()).collect();

    // Precision at K
    let mut precision_at_k = HashMap::new();
    for k in [1, 3, 5] {
        let top_k: HashSet<_> = cited.iter().take(k).map(|s| s.to_lowercase()).collect();
        let relevant = top_k.intersection(&expected_lower).count();
        let precision = if top_k.is_empty() { 0.0 } else {
            relevant as f64 / top_k.len() as f64
        };
        precision_at_k.insert(k, precision);
    }

    // Recall
    let retrieved_relevant = cited_lower.intersection(&expected_lower).count();
    let recall = if expected_lower.is_empty() { 1.0 } else {
        retrieved_relevant as f64 / expected_lower.len() as f64
    };

    // NDCG (Normalized Discounted Cumulative Gain)
    let ndcg = compute_ndcg(&cited_lower, &expected_lower);

    // Anti-principle rate
    let anti_cited = cited_lower.intersection(&anti_lower).count();
    let anti_rate = if cited.is_empty() { 0.0 } else {
        anti_cited as f64 / cited.len() as f64
    };

    // Thinker diversity
    let unique_thinkers: HashSet<_> = thinkers.iter().collect();
    let diversity = if thinkers.is_empty() { 0.0 } else {
        unique_thinkers.len() as f64 / thinkers.len() as f64
    };

    EvalMetrics {
        precision_at_k,
        recall,
        ndcg,
        anti_principle_rate: anti_rate,
        thinker_diversity: diversity,
        latency_ms,
    }
}

/// Compute NDCG (Normalized Discounted Cumulative Gain)
fn compute_ndcg(retrieved: &HashSet<String>, relevant: &HashSet<String>) -> f64 {
    if relevant.is_empty() {
        return 1.0;
    }

    // DCG: sum of (rel_i / log2(i + 1)) for each position
    let mut dcg = 0.0;
    for (i, item) in retrieved.iter().enumerate() {
        if relevant.contains(item) {
            dcg += 1.0 / (i as f64 + 2.0).log2();
        }
    }

    // Ideal DCG: all relevant items at top
    let mut idcg = 0.0;
    for i in 0..relevant.len() {
        idcg += 1.0 / (i as f64 + 2.0).log2();
    }

    if idcg == 0.0 { 0.0 } else { dcg / idcg }
}

/// Aggregate metrics across multiple results
fn aggregate_metrics(metrics: &[&EvalMetrics]) -> EvalMetrics {
    if metrics.is_empty() {
        return EvalMetrics::default();
    }

    let n = metrics.len() as f64;

    // Average precision at K
    let mut precision_at_k = HashMap::new();
    for k in [1, 3, 5] {
        let sum: f64 = metrics.iter()
            .filter_map(|m| m.precision_at_k.get(&k))
            .sum();
        precision_at_k.insert(k, sum / n);
    }

    // Average other metrics
    let recall = metrics.iter().map(|m| m.recall).sum::<f64>() / n;
    let ndcg = metrics.iter().map(|m| m.ndcg).sum::<f64>() / n;
    let anti_rate = metrics.iter().map(|m| m.anti_principle_rate).sum::<f64>() / n;
    let diversity = metrics.iter().map(|m| m.thinker_diversity).sum::<f64>() / n;
    let latency = metrics.iter().map(|m| m.latency_ms).sum::<u64>() / metrics.len() as u64;

    EvalMetrics {
        precision_at_k,
        recall,
        ndcg,
        anti_principle_rate: anti_rate,
        thinker_diversity: diversity,
        latency_ms: latency,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_ndcg() {
        let retrieved: HashSet<_> = ["a", "b", "c"].iter().map(|s| s.to_string()).collect();
        let relevant: HashSet<_> = ["a", "c"].iter().map(|s| s.to_string()).collect();

        let ndcg = compute_ndcg(&retrieved, &relevant);
        assert!(ndcg > 0.0 && ndcg <= 1.0);
    }

    #[test]
    fn test_precision_at_k() {
        let cited = vec!["YAGNI".to_string(), "Brooks Law".to_string(), "KISS".to_string()];
        let expected = vec!["YAGNI".to_string(), "KISS".to_string()];

        let metrics = compute_metrics(&cited, &expected, &[], &["Brooks".to_string()], 100);

        // P@1: YAGNI is relevant -> 1.0
        assert_eq!(metrics.precision_at_k.get(&1), Some(&1.0));

        // P@3: 2 of 3 are relevant -> 0.67
        let p3 = metrics.precision_at_k.get(&3).unwrap();
        assert!(*p3 > 0.6 && *p3 < 0.7);
    }
}
