//! Thinker and Principle Coverage Analysis
//!
//! Answers key questions:
//! - Which thinkers are over/under-used?
//! - Which principles are redundant?
//! - What domains have coverage gaps?
//! - Who should we add or remove?

use anyhow::Result;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Complete coverage analysis results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageAnalysis {
    /// Utilization rate per thinker (decisions citing / total decisions)
    pub thinker_utilization: HashMap<String, f64>,

    /// Coverage per domain (principles with domain tag / total)
    pub domain_coverage: HashMap<String, f64>,

    /// Pairs of similar principles (name1, name2, similarity)
    pub principle_redundancy: Vec<(String, String, f64)>,

    /// Principles that were never selected in any decision
    pub orphan_principles: Vec<String>,

    /// Recommended thinkers to add
    pub recommended_additions: Vec<ThinkerSuggestion>,

    /// Recommended thinkers/principles to remove
    pub recommended_removals: Vec<String>,
}

/// Suggestion for a new thinker to add
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThinkerSuggestion {
    pub name: String,
    pub domain: String,
    pub reason: String,
}

/// Run complete coverage analysis
pub fn analyze_coverage(conn: &Connection) -> Result<CoverageAnalysis> {
    let thinker_utilization = analyze_thinker_utilization(conn)?;
    let domain_coverage = analyze_domain_coverage(conn)?;
    let principle_redundancy = find_redundant_principles(conn)?;
    let orphan_principles = find_orphan_principles(conn)?;

    // Generate recommendations based on analysis
    let (recommended_additions, recommended_removals) =
        generate_recommendations(&thinker_utilization, &domain_coverage, &orphan_principles);

    Ok(CoverageAnalysis {
        thinker_utilization,
        domain_coverage,
        principle_redundancy,
        orphan_principles,
        recommended_additions,
        recommended_removals,
    })
}

/// Calculate utilization rate for each thinker
fn analyze_thinker_utilization(conn: &Connection) -> Result<HashMap<String, f64>> {
    let mut utilization = HashMap::new();

    // Get all thinkers
    let mut thinker_stmt = conn.prepare(
        "SELECT id, name FROM thinkers"
    )?;
    let thinkers: Vec<(String, String)> = thinker_stmt.query_map([], |row| {
        Ok((row.get(0)?, row.get(1)?))
    })?.collect::<Result<Vec<_>, _>>()?;

    // Get total decisions
    let total_decisions: i64 = conn.query_row(
        "SELECT COUNT(*) FROM decisions",
        [],
        |row| row.get(0),
    )?;

    if total_decisions == 0 {
        // No decisions yet - all thinkers have 0 utilization
        for (_, name) in &thinkers {
            utilization.insert(name.clone(), 0.0);
        }
        return Ok(utilization);
    }

    // Count decisions citing each thinker (via counsel_json)
    for (thinker_id, thinker_name) in &thinkers {
        // Search for thinker_id in counsel_json
        let citing_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM decisions WHERE counsel_json LIKE ?1",
            [format!("%\"thinker_id\":\"{}\"%", thinker_id)],
            |row| row.get(0),
        )?;

        let rate = citing_count as f64 / total_decisions as f64;
        utilization.insert(thinker_name.clone(), rate);
    }

    Ok(utilization)
}

/// Analyze coverage by domain
fn analyze_domain_coverage(conn: &Connection) -> Result<HashMap<String, f64>> {
    let mut coverage = HashMap::new();

    // Get all unique domains from principles
    let mut domain_stmt = conn.prepare(
        "SELECT DISTINCT domain_tags FROM principles WHERE domain_tags IS NOT NULL"
    )?;

    let domain_tags: Vec<String> = domain_stmt.query_map([], |row| row.get(0))?
        .filter_map(|r| r.ok())
        .collect();

    // Parse domains from JSON arrays
    let mut all_domains: HashSet<String> = HashSet::new();
    for tags_json in &domain_tags {
        if let Ok(tags) = serde_json::from_str::<Vec<String>>(tags_json) {
            for tag in tags {
                all_domains.insert(tag);
            }
        }
    }

    // Count principles per domain
    let total_principles: i64 = conn.query_row(
        "SELECT COUNT(*) FROM principles",
        [],
        |row| row.get(0),
    )?;

    for domain in &all_domains {
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM principles WHERE domain_tags LIKE ?1",
            [format!("%\"{}\"%" , domain)],
            |row| row.get(0),
        )?;

        let rate = count as f64 / total_principles.max(1) as f64;
        coverage.insert(domain.clone(), rate);
    }

    Ok(coverage)
}

/// Find principles that are semantically similar (potentially redundant)
fn find_redundant_principles(conn: &Connection) -> Result<Vec<(String, String, f64)>> {
    let mut redundant = Vec::new();

    // Get all principles
    let mut stmt = conn.prepare(
        "SELECT id, name, description FROM principles"
    )?;

    let principles: Vec<(String, String, String)> = stmt.query_map([], |row| {
        Ok((row.get(0)?, row.get(1)?, row.get(2)?))
    })?.collect::<Result<Vec<_>, _>>()?;

    // Compare each pair (O(n²) but principles count is small ~345)
    for i in 0..principles.len() {
        for j in (i + 1)..principles.len() {
            let (_, name_a, desc_a) = &principles[i];
            let (_, name_b, desc_b) = &principles[j];

            let similarity = compute_text_similarity(
                &format!("{} {}", name_a, desc_a),
                &format!("{} {}", name_b, desc_b),
            );

            // High similarity threshold (0.7+) suggests redundancy
            if similarity > 0.7 {
                redundant.push((name_a.clone(), name_b.clone(), similarity));
            }
        }
    }

    // Sort by similarity (highest first)
    redundant.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap());

    Ok(redundant)
}

/// Find principles that have never been selected
fn find_orphan_principles(conn: &Connection) -> Result<Vec<String>> {
    let mut orphans = Vec::new();

    // Get all principle names
    let mut stmt = conn.prepare("SELECT name FROM principles")?;
    let all_names: Vec<String> = stmt.query_map([], |row| row.get(0))?
        .filter_map(|r| r.ok())
        .collect();

    // Get all principles cited in decisions
    let mut cited_stmt = conn.prepare("SELECT counsel_json FROM decisions")?;
    let counsel_jsons: Vec<String> = cited_stmt.query_map([], |row| row.get(0))?
        .filter_map(|r| r.ok())
        .collect();

    // Extract cited principles from JSON
    let mut cited_principles: HashSet<String> = HashSet::new();
    for json in &counsel_jsons {
        // Parse and extract principles_cited arrays
        if let Ok(counsel) = serde_json::from_str::<serde_json::Value>(json) {
            if let Some(positions) = counsel.get("positions").and_then(|p| p.as_array()) {
                for pos in positions {
                    if let Some(cited) = pos.get("principles_cited").and_then(|c| c.as_array()) {
                        for p in cited {
                            if let Some(name) = p.as_str() {
                                cited_principles.insert(name.to_lowercase());
                            }
                        }
                    }
                }
            }
        }
    }

    // Find uncited principles
    for name in &all_names {
        if !cited_principles.contains(&name.to_lowercase()) {
            orphans.push(name.clone());
        }
    }

    Ok(orphans)
}

/// Generate recommendations based on analysis
fn generate_recommendations(
    utilization: &HashMap<String, f64>,
    domain_coverage: &HashMap<String, f64>,
    orphans: &[String],
) -> (Vec<ThinkerSuggestion>, Vec<String>) {
    let mut additions = Vec::new();
    let mut removals = Vec::new();

    // Find under-covered domains (< 5% of principles)
    for (domain, coverage) in domain_coverage {
        if *coverage < 0.05 {
            additions.push(ThinkerSuggestion {
                name: format!("{} Expert", capitalize_domain(domain)),
                domain: domain.clone(),
                reason: format!(
                    "Domain '{}' has only {:.1}% principle coverage",
                    domain,
                    coverage * 100.0
                ),
            });
        }
    }

    // Find thinkers with 0% utilization who have many principles
    for (thinker, util) in utilization {
        if *util < 0.01 {
            removals.push(format!(
                "{} (cited in <1% of decisions)",
                thinker
            ));
        }
    }

    // Check for systematic gaps in important domains
    let important_domains = [
        "software-architecture",
        "entrepreneurship",
        "management-theory",
        "ai-ml",
        "security",
    ];

    for domain in important_domains {
        if !domain_coverage.contains_key(domain) {
            additions.push(ThinkerSuggestion {
                name: format!("{} Thought Leader", capitalize_domain(domain)),
                domain: domain.to_string(),
                reason: format!(
                    "Important domain '{}' has no dedicated principles",
                    domain
                ),
            });
        }
    }

    // If many orphan principles (>20%), suggest review
    if orphans.len() > 20 {
        additions.push(ThinkerSuggestion {
            name: "Keyword Optimization".to_string(),
            domain: "meta".to_string(),
            reason: format!(
                "{} principles never selected - review keywords and descriptions",
                orphans.len()
            ),
        });
    }

    (additions, removals)
}

/// Compute text similarity using Jaccard similarity on word n-grams
fn compute_text_similarity(a: &str, b: &str) -> f64 {
    let a_ngrams = text_to_ngrams(a, 2);
    let b_ngrams = text_to_ngrams(b, 2);

    if a_ngrams.is_empty() || b_ngrams.is_empty() {
        return 0.0;
    }

    let intersection = a_ngrams.intersection(&b_ngrams).count();
    let union = a_ngrams.union(&b_ngrams).count();

    if union == 0 {
        0.0
    } else {
        intersection as f64 / union as f64
    }
}

/// Convert text to set of word n-grams
fn text_to_ngrams(text: &str, n: usize) -> HashSet<String> {
    let lower = text.to_lowercase();
    let words: Vec<&str> = lower
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() > 2)
        .collect();

    let mut ngrams = HashSet::new();
    for window in words.windows(n) {
        ngrams.insert(window.join(" "));
    }

    // Also add individual words
    for word in &words {
        ngrams.insert((*word).to_string());
    }

    ngrams
}

/// Capitalize domain name for display
fn capitalize_domain(domain: &str) -> String {
    domain
        .split('-')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(c) => c.to_uppercase().chain(chars).collect(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Print coverage analysis results
pub fn print_coverage_analysis(analysis: &CoverageAnalysis) {
    println!("\n┌─────────────────────────────────────────────────────────────┐");
    println!("│ 📊 100MINDS COVERAGE ANALYSIS                               │");
    println!("└─────────────────────────────────────────────────────────────┘\n");

    // Thinker utilization
    let active = analysis.thinker_utilization.values().filter(|&&u| u > 0.01).count();
    let total = analysis.thinker_utilization.len();
    println!("THINKER UTILIZATION: {}/{} active (>1%)", active, total);

    let mut sorted_util: Vec<_> = analysis.thinker_utilization.iter().collect();
    sorted_util.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap());

    println!("\nTop 10:");
    for (name, util) in sorted_util.iter().take(10) {
        let bar = "█".repeat((**util * 20.0) as usize);
        println!("   {:30} {:5.1}% {}", name, *util * 100.0, bar);
    }

    // Domain coverage
    println!("\nDOMAIN COVERAGE:");
    for (domain, coverage) in &analysis.domain_coverage {
        println!("   {:25} {:5.1}%", domain, coverage * 100.0);
    }

    // Orphans
    if !analysis.orphan_principles.is_empty() {
        println!("\nORPHAN PRINCIPLES ({}): never selected", analysis.orphan_principles.len());
        for p in analysis.orphan_principles.iter().take(5) {
            println!("   • {}", p);
        }
        if analysis.orphan_principles.len() > 5 {
            println!("   ... and {} more", analysis.orphan_principles.len() - 5);
        }
    }

    // Redundancy
    if !analysis.principle_redundancy.is_empty() {
        println!("\nPOTENTIAL REDUNDANCIES:");
        for (a, b, sim) in analysis.principle_redundancy.iter().take(5) {
            println!("   {:.0}% similar: \"{}\" ↔ \"{}\"", sim * 100.0, a, b);
        }
    }

    // Recommendations
    if !analysis.recommended_additions.is_empty() {
        println!("\nRECOMMENDED ADDITIONS:");
        for suggestion in &analysis.recommended_additions {
            println!("   ➕ {} ({})", suggestion.name, suggestion.domain);
            println!("      {}", suggestion.reason);
        }
    }

    if !analysis.recommended_removals.is_empty() {
        println!("\nRECOMMENDED REMOVALS:");
        for name in &analysis.recommended_removals {
            println!("   ➖ {}", name);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_similarity() {
        let a = "Adding people to a late project makes it later";
        let b = "Adding engineers to a late project makes it later";

        let sim = compute_text_similarity(a, b);
        // Jaccard on bigrams: ~0.57 for these sentences (8/14 shared n-grams)
        assert!(sim > 0.5, "Expected similar sentences to have sim > 0.5, got {}", sim);

        let c = "Microservices require careful consideration";
        let sim2 = compute_text_similarity(a, c);
        assert!(sim2 < 0.3, "Expected dissimilar sentences to have sim < 0.3, got {}", sim2);
    }

    #[test]
    fn test_capitalize_domain() {
        assert_eq!(capitalize_domain("software-architecture"), "Software Architecture");
        assert_eq!(capitalize_domain("ai-ml"), "Ai Ml");
        assert_eq!(capitalize_domain("security"), "Security");
    }
}
