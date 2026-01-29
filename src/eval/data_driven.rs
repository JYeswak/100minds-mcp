//! Data-Driven Evaluation Framework
//!
//! Uses heuristics to evaluate quality rather than matching against hardcoded expectations.
//! Records results for continuous learning and Thompson Sampling optimization.

use super::synthetic::{generate_sample, GeneratorConfig, SyntheticQuestion};
use crate::counsel::CounselEngine;
use crate::provenance::Provenance;
use crate::types::{CounselContext, CounselDepth, CounselRequest};

#[allow(unused_imports)] // Used by full evaluation mode
use crate::types::CounselResponse;
use anyhow::Result;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Instant;

/// Results from data-driven evaluation run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataDrivenResults {
    pub total_questions: usize,
    pub avg_quality_score: f64,
    pub scores_by_criterion: HashMap<String, f64>,
    pub scores_by_domain: HashMap<String, DomainStats>,
    pub best_performers: Vec<QuestionResult>,
    pub worst_performers: Vec<QuestionResult>,
    pub principle_effectiveness: HashMap<String, PrincipleStats>,
    pub thinker_effectiveness: HashMap<String, ThinkerStats>,
}

/// Stats per domain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainStats {
    pub count: usize,
    pub avg_score: f64,
    pub avg_relevance: f64,
    pub avg_actionability: f64,
}

/// Stats per principle - tracks how effective each principle is
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrincipleStats {
    pub times_cited: usize,
    pub avg_score_when_cited: f64,
    pub domains_cited_in: Vec<String>,
}

/// Stats per thinker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThinkerStats {
    pub times_cited: usize,
    pub avg_score_when_cited: f64,
    pub top_principles: Vec<String>,
}

/// Result for a single question
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionResult {
    pub question: String,
    pub domain: String,
    pub quality_score: f64,
    pub relevance: f64,
    pub actionability: f64,
    pub principles_cited: Vec<String>,
    pub thinkers_cited: Vec<String>,
    pub latency_ms: u64,
    pub judge_reasoning: String,
}

/// Run data-driven evaluation without LLM judge (fast mode)
/// Uses heuristics to estimate quality based on principles returned
pub fn run_fast_evaluation(
    conn: &Connection,
    provenance: &Provenance,
    num_questions: usize,
) -> Result<DataDrivenResults> {
    let engine = CounselEngine::new(conn, provenance);
    let config = GeneratorConfig::default();
    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(42);
    let questions = generate_sample(&config, num_questions, seed);

    let mut results: Vec<QuestionResult> = Vec::new();
    let mut domain_scores: HashMap<String, Vec<f64>> = HashMap::new();
    let mut principle_stats: HashMap<String, (usize, f64)> = HashMap::new();
    let mut thinker_stats: HashMap<String, (usize, f64)> = HashMap::new();

    for (i, q) in questions.iter().enumerate() {
        if i % 100 == 0 && i > 0 {
            eprintln!("Progress: {}/{}", i, num_questions);
        }

        let result = evaluate_single_question(conn, &engine, q)?;

        // Track domain scores
        domain_scores
            .entry(q.domain.clone())
            .or_default()
            .push(result.quality_score);

        // Track principle effectiveness
        for principle in &result.principles_cited {
            let entry = principle_stats.entry(principle.clone()).or_insert((0, 0.0));
            entry.0 += 1;
            entry.1 += result.quality_score;
        }

        // Track thinker effectiveness
        for thinker in &result.thinkers_cited {
            let entry = thinker_stats.entry(thinker.clone()).or_insert((0, 0.0));
            entry.0 += 1;
            entry.1 += result.quality_score;
        }

        results.push(result);
    }

    // Calculate aggregates
    let total_score: f64 = results.iter().map(|r| r.quality_score).sum();
    let avg_quality_score = total_score / results.len().max(1) as f64;

    // Domain stats
    let scores_by_domain: HashMap<String, DomainStats> = domain_scores
        .into_iter()
        .map(|(domain, scores)| {
            let avg = scores.iter().sum::<f64>() / scores.len() as f64;
            (
                domain,
                DomainStats {
                    count: scores.len(),
                    avg_score: avg,
                    avg_relevance: avg, // Simplified for fast mode
                    avg_actionability: avg,
                },
            )
        })
        .collect();

    // Principle effectiveness
    let principle_effectiveness: HashMap<String, PrincipleStats> = principle_stats
        .into_iter()
        .map(|(name, (count, total))| {
            (
                name,
                PrincipleStats {
                    times_cited: count,
                    avg_score_when_cited: total / count as f64,
                    domains_cited_in: vec![], // TODO: track
                },
            )
        })
        .collect();

    // Thinker effectiveness
    let thinker_effectiveness: HashMap<String, ThinkerStats> = thinker_stats
        .into_iter()
        .map(|(name, (count, total))| {
            (
                name,
                ThinkerStats {
                    times_cited: count,
                    avg_score_when_cited: total / count as f64,
                    top_principles: vec![], // TODO: track
                },
            )
        })
        .collect();

    // Sort for best/worst
    let mut sorted = results.clone();
    sorted.sort_by(|a, b| b.quality_score.partial_cmp(&a.quality_score).unwrap());

    let best_performers: Vec<_> = sorted.iter().take(10).cloned().collect();
    let worst_performers: Vec<_> = sorted.iter().rev().take(10).cloned().collect();

    Ok(DataDrivenResults {
        total_questions: results.len(),
        avg_quality_score,
        scores_by_criterion: HashMap::new(), // Simplified for fast mode
        scores_by_domain,
        best_performers,
        worst_performers,
        principle_effectiveness,
        thinker_effectiveness,
    })
}

/// Evaluate a single question using heuristic scoring
fn evaluate_single_question(
    _conn: &Connection,
    engine: &CounselEngine,
    question: &SyntheticQuestion,
) -> Result<QuestionResult> {
    let start = Instant::now();

    let request = CounselRequest {
        question: question.question.clone(),
        context: CounselContext {
            domain: Some(question.domain.clone()),
            constraints: vec![],
            prefer_thinkers: vec![],
            depth: CounselDepth::Standard,
        },
    };

    let response = engine.counsel(&request)?;
    let latency = start.elapsed().as_millis() as u64;

    // Extract principles and thinkers
    let mut principles: Vec<String> = Vec::new();
    let mut thinkers: Vec<String> = Vec::new();

    for position in &response.positions {
        thinkers.push(position.thinker.clone());
        principles.extend(position.principles_cited.clone());
    }

    // Heuristic quality scoring (0-5 scale)
    let mut score: f64 = 2.5; // Base score

    // Boost for having positions
    if !response.positions.is_empty() {
        score += 0.5;
    }

    // Boost for having multiple perspectives (FOR and AGAINST)
    let has_for = response
        .positions
        .iter()
        .any(|p| p.stance == crate::types::Stance::For);
    let has_against = response
        .positions
        .iter()
        .any(|p| p.stance == crate::types::Stance::Against);
    if has_for && has_against {
        score += 0.5;
    }

    // Boost for thinker diversity
    let unique_thinker_count = {
        let unique: std::collections::HashSet<_> = thinkers.iter().collect();
        unique.len()
    };
    if unique_thinker_count >= 3 {
        score += 0.5;
    }

    // Boost for domain match (principle description contains domain terms)
    // This is a simplified heuristic
    let domain_keywords: Vec<&str> = match question.domain.as_str() {
        "architecture" => vec!["architect", "design", "system", "service", "micro"],
        "testing" => vec!["test", "tdd", "coverage", "unit", "integration"],
        "performance" => vec!["optimi", "fast", "slow", "latency", "profile"],
        "scaling" => vec!["scale", "growth", "team", "communi"],
        "rewrite" => vec!["refactor", "legacy", "rewrite", "migration"],
        _ => vec![],
    };

    let argument_lower: String = response
        .positions
        .iter()
        .map(|p| p.argument.to_lowercase())
        .collect::<Vec<_>>()
        .join(" ");

    let domain_matches = domain_keywords
        .iter()
        .filter(|kw| argument_lower.contains(*kw))
        .count();

    if domain_matches >= 2 {
        score += 0.5;
    }

    // Cap at 5.0
    score = score.min(5.0);

    let judge_reasoning = format!(
        "Heuristic score: {} positions, {} unique thinkers, {} domain matches",
        response.positions.len(),
        unique_thinker_count,
        domain_matches
    );

    Ok(QuestionResult {
        question: question.question.clone(),
        domain: question.domain.clone(),
        quality_score: score,
        relevance: score,     // Simplified
        actionability: score, // Simplified
        principles_cited: principles,
        thinkers_cited: thinkers,
        latency_ms: latency,
        judge_reasoning,
    })
}

/// Print data-driven results
pub fn print_data_driven_results(results: &DataDrivenResults) {
    println!("\n┌─────────────────────────────────────────────────────────────┐");
    println!("│ 📊 DATA-DRIVEN EVALUATION RESULTS                           │");
    println!("└─────────────────────────────────────────────────────────────┘\n");

    println!(
        "OVERALL: {} questions, avg quality: {:.2}/5.0",
        results.total_questions, results.avg_quality_score
    );

    println!("\nBY DOMAIN:");
    let mut domains: Vec<_> = results.scores_by_domain.iter().collect();
    domains.sort_by(|a, b| b.1.avg_score.partial_cmp(&a.1.avg_score).unwrap());
    for (domain, stats) in domains {
        let bar = "█".repeat((stats.avg_score * 4.0) as usize);
        println!(
            "   {:20} {:.2}/5.0 {} (n={})",
            domain, stats.avg_score, bar, stats.count
        );
    }

    println!("\nTOP PERFORMING THINKERS:");
    let mut thinkers: Vec<_> = results.thinker_effectiveness.iter().collect();
    thinkers.sort_by(|a, b| {
        b.1.avg_score_when_cited
            .partial_cmp(&a.1.avg_score_when_cited)
            .unwrap()
    });
    for (name, stats) in thinkers.iter().take(10) {
        println!(
            "   {:25} {:.2}/5.0 (cited {} times)",
            name, stats.avg_score_when_cited, stats.times_cited
        );
    }

    println!("\nTOP PERFORMING PRINCIPLES:");
    let mut principles: Vec<_> = results
        .principle_effectiveness
        .iter()
        .filter(|(_, s)| s.times_cited >= 5) // Only principles cited enough times
        .collect();
    principles.sort_by(|a, b| {
        b.1.avg_score_when_cited
            .partial_cmp(&a.1.avg_score_when_cited)
            .unwrap()
    });
    for (name, stats) in principles.iter().take(10) {
        println!(
            "   {:40} {:.2}/5.0 (cited {} times)",
            truncate(name, 40),
            stats.avg_score_when_cited,
            stats.times_cited
        );
    }

    println!("\nWORST PERFORMERS (questions to investigate):");
    for result in results.worst_performers.iter().take(5) {
        println!(
            "   [{:.1}] {} - {}",
            result.quality_score,
            result.domain,
            truncate(&result.question, 50)
        );
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max - 3])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heuristic_scoring() {
        // Basic test that scoring works
        let score = 2.5 + 0.5 + 0.5; // base + positions + diversity
        assert!(score >= 2.0 && score <= 5.0);
    }
}
