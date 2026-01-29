//! Evaluation Framework for 100minds
//!
//! ML-style evaluation and optimization system:
//! - Scenario benchmarks with ground truth
//! - Monte Carlo simulation for principle selection analysis
//! - Thompson Sampling for principle optimization
//! - LLM-as-judge quality assessment
//! - Thinker/principle coverage analysis
//!
//! 2026 SOTA additions:
//! - Synthetic question generation (100k-1M scale)
//! - Feel-Good Thompson Sampling with UCB exploration
//! - Bayesian hyperparameter optimization
//! - Multi-criteria LLM-as-judge rubric

pub mod bandit;
pub mod coverage;
pub mod data_driven;
pub mod judge;
pub mod llm_judge;
pub mod monte_carlo;
pub mod scenarios;
pub mod synthetic;
pub mod thompson;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Unified evaluation metrics across all eval types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalMetrics {
    /// Precision at K (what fraction of top K results are relevant)
    pub precision_at_k: HashMap<usize, f64>,

    /// Recall (what fraction of relevant items were retrieved)
    pub recall: f64,

    /// Normalized Discounted Cumulative Gain (ranking quality)
    pub ndcg: f64,

    /// Rate of selecting anti-principles (wrong recommendations)
    pub anti_principle_rate: f64,

    /// Diversity of thinkers cited (unique / total)
    pub thinker_diversity: f64,

    /// Response latency in milliseconds
    pub latency_ms: u64,
}

impl Default for EvalMetrics {
    fn default() -> Self {
        Self {
            precision_at_k: HashMap::new(),
            recall: 0.0,
            ndcg: 0.0,
            anti_principle_rate: 0.0,
            thinker_diversity: 0.0,
            latency_ms: 0,
        }
    }
}

/// Overall evaluation report combining all analysis types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalReport {
    pub timestamp: String,
    pub scenario_results: Option<scenarios::ScenarioResults>,
    pub monte_carlo_results: Option<monte_carlo::MonteCarloResults>,
    pub coverage_analysis: Option<coverage::CoverageAnalysis>,
    pub judge_results: Option<llm_judge::JudgeResults>,
    pub summary: EvalSummary,
}

/// High-level summary for quick assessment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalSummary {
    pub overall_score: f64,
    pub strengths: Vec<String>,
    pub weaknesses: Vec<String>,
    pub recommendations: Vec<String>,
}

impl EvalReport {
    /// Generate summary from component results
    pub fn generate_summary(&mut self) {
        let mut strengths = Vec::new();
        let mut weaknesses = Vec::new();
        let mut recommendations = Vec::new();
        let mut scores = Vec::new();

        // Analyze scenario results
        if let Some(ref sr) = self.scenario_results {
            let avg_precision = sr
                .by_category
                .values()
                .filter_map(|m| m.precision_at_k.get(&3))
                .sum::<f64>()
                / sr.by_category.len().max(1) as f64;

            scores.push(avg_precision);

            if avg_precision >= 0.7 {
                strengths.push(format!("Strong P@3: {:.1}%", avg_precision * 100.0));
            } else {
                weaknesses.push(format!("Low P@3: {:.1}%", avg_precision * 100.0));
                recommendations
                    .push("Improve relevance scoring or add missing principles".to_string());
            }

            if sr.aggregate.anti_principle_rate > 0.05 {
                weaknesses.push(format!(
                    "High anti-principle rate: {:.1}%",
                    sr.aggregate.anti_principle_rate * 100.0
                ));
                recommendations.push("Review anti-principle detection and scoring".to_string());
            }
        }

        // Analyze coverage
        if let Some(ref ca) = self.coverage_analysis {
            if !ca.orphan_principles.is_empty() {
                weaknesses.push(format!(
                    "{} orphan principles never selected",
                    ca.orphan_principles.len()
                ));
                recommendations
                    .push("Remove or improve keywords for orphan principles".to_string());
            }

            if !ca.recommended_removals.is_empty() {
                recommendations.push(format!(
                    "Consider removing {} redundant thinkers",
                    ca.recommended_removals.len()
                ));
            }

            // Check thinker diversity
            let active_thinkers = ca
                .thinker_utilization
                .values()
                .filter(|&&u| u > 0.01)
                .count();
            let total_thinkers = ca.thinker_utilization.len();
            let utilization_rate = active_thinkers as f64 / total_thinkers.max(1) as f64;

            scores.push(utilization_rate);

            if utilization_rate < 0.5 {
                weaknesses.push(format!(
                    "Low thinker utilization: only {}/{} actively cited",
                    active_thinkers, total_thinkers
                ));
            }
        }

        // Analyze Monte Carlo results
        if let Some(ref mc) = self.monte_carlo_results {
            let variance = mc.selection_variance;
            if variance > 0.3 {
                weaknesses.push(format!("High selection variance: {:.2}", variance));
                recommendations
                    .push("Stabilize principle selection with better scoring".to_string());
            } else {
                strengths.push(format!("Stable selection: variance {:.2}", variance));
            }

            scores.push(1.0 - variance.min(1.0));
        }

        // Calculate overall score
        let overall = if scores.is_empty() {
            0.5
        } else {
            scores.iter().sum::<f64>() / scores.len() as f64
        };

        self.summary = EvalSummary {
            overall_score: overall,
            strengths,
            weaknesses,
            recommendations,
        };
    }
}

/// Print evaluation results in a human-readable format
pub fn print_eval_report(report: &EvalReport) {
    println!("\n┌─────────────────────────────────────────────────────────────┐");
    println!("│ 🧪 100MINDS EVALUATION REPORT                               │");
    println!(
        "│    {}                                          │",
        &report.timestamp[..10]
    );
    println!("└─────────────────────────────────────────────────────────────┘\n");

    // Overall score
    let score_bar = "█".repeat((report.summary.overall_score * 10.0) as usize);
    let empty_bar = "░".repeat(10 - (report.summary.overall_score * 10.0) as usize);
    println!(
        "OVERALL SCORE: [{}{}] {:.0}%\n",
        score_bar,
        empty_bar,
        report.summary.overall_score * 100.0
    );

    // Strengths
    if !report.summary.strengths.is_empty() {
        println!("✅ STRENGTHS:");
        for s in &report.summary.strengths {
            println!("   • {}", s);
        }
        println!();
    }

    // Weaknesses
    if !report.summary.weaknesses.is_empty() {
        println!("⚠️  WEAKNESSES:");
        for w in &report.summary.weaknesses {
            println!("   • {}", w);
        }
        println!();
    }

    // Recommendations
    if !report.summary.recommendations.is_empty() {
        println!("💡 RECOMMENDATIONS:");
        for r in &report.summary.recommendations {
            println!("   • {}", r);
        }
        println!();
    }

    // Detailed sections
    if let Some(ref sr) = report.scenario_results {
        print_scenario_results(sr);
    }

    if let Some(ref mc) = report.monte_carlo_results {
        print_monte_carlo_results(mc);
    }

    if let Some(ref ca) = report.coverage_analysis {
        print_coverage_results(ca);
    }
}

fn print_scenario_results(results: &scenarios::ScenarioResults) {
    println!("─────────────────────────────────────────────────────────────");
    println!("📊 SCENARIO BENCHMARK RESULTS");
    println!("─────────────────────────────────────────────────────────────");
    println!("Scenarios run: {}", results.total_scenarios);
    println!();

    // Aggregate metrics
    println!("AGGREGATE METRICS:");
    for k in [1, 3, 5] {
        if let Some(&p) = results.aggregate.precision_at_k.get(&k) {
            println!("   P@{}: {:.1}%", k, p * 100.0);
        }
    }
    println!("   Recall: {:.1}%", results.aggregate.recall * 100.0);
    println!("   NDCG: {:.3}", results.aggregate.ndcg);
    println!(
        "   Anti-principle rate: {:.1}%",
        results.aggregate.anti_principle_rate * 100.0
    );
    println!(
        "   Thinker diversity: {:.1}%",
        results.aggregate.thinker_diversity * 100.0
    );
    println!("   Avg latency: {}ms", results.aggregate.latency_ms);
    println!();

    // By category
    println!("BY CATEGORY:");
    for (cat, metrics) in &results.by_category {
        let p3 = metrics.precision_at_k.get(&3).unwrap_or(&0.0);
        println!(
            "   {:20} P@3: {:.0}%  Recall: {:.0}%",
            cat,
            p3 * 100.0,
            metrics.recall * 100.0
        );
    }
    println!();
}

fn print_monte_carlo_results(results: &monte_carlo::MonteCarloResults) {
    println!("─────────────────────────────────────────────────────────────");
    println!("🎲 MONTE CARLO SIMULATION RESULTS");
    println!("─────────────────────────────────────────────────────────────");
    println!("Simulations: {}", results.num_simulations);
    println!("Selection variance: {:.3}", results.selection_variance);
    println!(
        "95% CI: [{:.2}, {:.2}]",
        results.confidence_interval_95.0, results.confidence_interval_95.1
    );
    println!(
        "Tail risk (<50% relevance): {:.1}%",
        results.tail_risk * 100.0
    );
    println!();

    // Top over-selected
    println!("TOP 5 OVER-SELECTED PRINCIPLES:");
    let mut sorted: Vec<_> = results.principle_selection_rates.iter().collect();
    sorted.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap());
    for (name, rate) in sorted.iter().take(5) {
        println!("   {:.1}% - {}", *rate * 100.0, name);
    }
    println!();

    // Under-selected
    println!("TOP 5 UNDER-SELECTED (non-zero):");
    sorted.sort_by(|a, b| a.1.partial_cmp(b.1).unwrap());
    for (name, rate) in sorted.iter().filter(|(_, r)| **r > 0.0).take(5) {
        println!("   {:.1}% - {}", *rate * 100.0, name);
    }
    println!();
}

fn print_coverage_results(analysis: &coverage::CoverageAnalysis) {
    println!("─────────────────────────────────────────────────────────────");
    println!("📈 COVERAGE ANALYSIS");
    println!("─────────────────────────────────────────────────────────────");

    // Thinker utilization
    let active = analysis
        .thinker_utilization
        .values()
        .filter(|&&u| u > 0.01)
        .count();
    let total = analysis.thinker_utilization.len();
    println!(
        "Thinker utilization: {}/{} actively cited (>1%)",
        active, total
    );
    println!();

    // Top thinkers
    println!("TOP 10 THINKERS BY UTILIZATION:");
    let mut sorted: Vec<_> = analysis.thinker_utilization.iter().collect();
    sorted.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap());
    for (name, rate) in sorted.iter().take(10) {
        let bar = "█".repeat((**rate * 20.0) as usize);
        println!("   {:.<20} {:5.1}% {}", name, *rate * 100.0, bar);
    }
    println!();

    // Domain coverage
    println!("DOMAIN COVERAGE:");
    for (domain, coverage) in &analysis.domain_coverage {
        println!("   {:25} {:.0}%", domain, coverage * 100.0);
    }
    println!();

    // Orphans
    if !analysis.orphan_principles.is_empty() {
        println!("⚠️  ORPHAN PRINCIPLES (never selected):");
        for p in analysis.orphan_principles.iter().take(10) {
            println!("   • {}", p);
        }
        if analysis.orphan_principles.len() > 10 {
            println!("   ... and {} more", analysis.orphan_principles.len() - 10);
        }
        println!();
    }

    // Redundancy
    if !analysis.principle_redundancy.is_empty() {
        println!("🔄 REDUNDANT PRINCIPLE PAIRS (similarity > 0.8):");
        for (p1, p2, sim) in analysis.principle_redundancy.iter().take(5) {
            println!("   {:.0}% similar: {} ↔ {}", sim * 100.0, p1, p2);
        }
        println!();
    }

    // Recommendations
    if !analysis.recommended_removals.is_empty() {
        println!("🗑️  RECOMMENDED REMOVALS:");
        for name in &analysis.recommended_removals {
            println!("   • {}", name);
        }
        println!();
    }

    if !analysis.recommended_additions.is_empty() {
        println!("➕ RECOMMENDED ADDITIONS:");
        for suggestion in &analysis.recommended_additions {
            println!("   • {} (domain: {})", suggestion.name, suggestion.domain);
            println!("     Reason: {}", suggestion.reason);
        }
        println!();
    }
}
