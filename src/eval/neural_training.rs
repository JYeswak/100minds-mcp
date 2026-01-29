//! Neural Bandit Training Data Generation
//!
//! Generates synthetic decision/outcome pairs for training neural posterior networks.
//! The neural bandit replaces Beta distributions with learned representations that
//! capture complex context-dependent success patterns.
//!
//! Training data format:
//! - Question embedding (context)
//! - Principle embedding (arm)
//! - Domain one-hot encoding
//! - Difficulty level
//! - Success label (0/1)
//!
//! Architecture target: Neural posterior with ~10k training examples

use super::synthetic::{generate_sample, GeneratorConfig};
use crate::counsel::CounselEngine;
use crate::provenance::Provenance;
use crate::types::{CounselContext, CounselDepth, CounselRequest};
use anyhow::Result;
use rand::prelude::*;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// A single training example for the neural bandit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingExample {
    /// Unique ID for the example
    pub id: String,

    /// The question/decision text
    pub question: String,

    /// Domain of the question
    pub domain: String,

    /// Difficulty level (1-5)
    pub difficulty: u8,

    /// The principle that was selected
    pub principle_id: String,

    /// The principle name (for human readability)
    pub principle_name: String,

    /// The thinker who proposed the principle
    pub thinker_id: String,

    /// Position rank (0 = first recommended, higher = less relevant)
    pub position_rank: usize,

    /// Confidence score from the selection algorithm
    pub confidence: f64,

    /// Whether this principle led to a good outcome (0.0 or 1.0)
    pub success: f64,

    /// Reasoning for the success/failure (for debugging)
    pub reasoning: String,

    /// Additional context features
    pub context_features: ContextFeatures,
}

/// Contextual features for the neural network
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextFeatures {
    /// Stakeholder type (CTO, Engineer, PM, etc.)
    pub stakeholder: String,

    /// Company stage (startup, growth, enterprise)
    pub company_stage: String,

    /// Urgency level (exploration, crisis, etc.)
    pub urgency: String,

    /// Whether there's a domain match between question and principle
    pub domain_match: bool,

    /// Number of principles selected for this decision
    pub total_principles_selected: usize,

    /// Whether this was a FOR or AGAINST position
    pub is_for_position: bool,
}

/// Configuration for training data generation (V3 - causal heuristics)
#[derive(Debug, Clone)]
pub struct TrainingConfig {
    /// Number of questions to generate
    pub num_questions: usize,

    /// Random seed for reproducibility
    pub seed: u64,

    /// Success rate baseline (before adjustments)
    pub base_success_rate: f64,

    /// Bonus for domain match
    pub domain_match_bonus: f64,

    /// Bonus for low position rank (more relevant)
    pub relevance_bonus: f64,

    /// Penalty for high difficulty
    pub difficulty_penalty: f64,

    /// Noise factor for randomness
    pub noise_factor: f64,

    // V2 heuristic improvements
    /// Confidence score weight (how much counsel confidence affects success)
    pub confidence_weight: f64,

    /// Thinker expertise bonus (domain-matched thinker)
    pub thinker_expertise_bonus: f64,

    /// Disagreement penalty (for vs against mismatch)
    pub disagreement_penalty: f64,

    // V3 causal fixes (ICLR 2026 inspired)
    /// Penalty for wrong-domain expert (Beck on arch should fail)
    pub cross_domain_expert_penalty: f64,

    /// Exponential rank decay base (0.5 = halve each rank)
    pub rank_decay_base: f64,

    /// Causal adversarial flip rate (for domain mismatch probing)
    pub causal_adversarial_rate: f64,
}

impl Default for TrainingConfig {
    fn default() -> Self {
        Self {
            num_questions: 10_000,
            seed: 42,
            // V3: Calibrated to avoid ceiling saturation
            base_success_rate: 0.35,  // Lowered from 0.45 for spread
            domain_match_bonus: 0.15, // Reduced from 0.25
            relevance_bonus: 0.20,    // V3: For exponential decay
            difficulty_penalty: 0.08,
            noise_factor: 0.08,       // Reduced from 0.1 for cleaner signal
            // V2 improvements
            confidence_weight: 0.20,       // Reduced from 0.3
            thinker_expertise_bonus: 0.15, // Bonus for expert thinker
            disagreement_penalty: 0.10,    // Penalty for position mismatch
            // V4 causal fixes (ICLR 2026 + deep probe hardening)
            cross_domain_expert_penalty: 0.40, // STRONG: Beck on arch → <50%
            rank_decay_base: 0.5,              // Exponential: 11pp+ spread
            causal_adversarial_rate: 0.20,     // V4: Increased to 20% for robustness
        }
    }
}

/// Training data batch result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingBatch {
    pub examples: Vec<TrainingExample>,
    pub metadata: BatchMetadata,
}

/// Metadata about the generated batch
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchMetadata {
    pub total_examples: usize,
    pub positive_examples: usize,
    pub negative_examples: usize,
    pub unique_questions: usize,
    pub unique_principles: usize,
    pub unique_thinkers: usize,
    pub domains: HashMap<String, usize>,
    pub avg_examples_per_question: f64,
    pub generation_seed: u64,
}

/// Ground truth heuristics for simulating outcomes
/// Maps question patterns to expected principle patterns
#[derive(Debug, Clone)]
struct GroundTruthHeuristics {
    /// Patterns that indicate scaling questions
    scaling_patterns: Vec<&'static str>,
    /// Patterns that indicate architecture questions
    architecture_patterns: Vec<&'static str>,
    /// Patterns that indicate testing questions
    testing_patterns: Vec<&'static str>,
    /// Patterns that indicate management questions
    management_patterns: Vec<&'static str>,

    /// Principles that are good for scaling
    scaling_principles: HashSet<&'static str>,
    /// Principles that are good for architecture
    architecture_principles: HashSet<&'static str>,
    /// Principles that are good for testing
    testing_principles: HashSet<&'static str>,
    /// Principles that are good for management
    management_principles: HashSet<&'static str>,
}

impl Default for GroundTruthHeuristics {
    fn default() -> Self {
        Self {
            scaling_patterns: vec![
                "scale", "traffic", "load", "horizontal", "vertical",
                "performance", "bottleneck", "servers", "capacity",
            ],
            architecture_patterns: vec![
                "microservice", "monolith", "rewrite", "refactor", "decompose",
                "migrate", "architecture", "service", "api",
            ],
            testing_patterns: vec![
                "test", "tdd", "coverage", "unit", "integration",
                "flaky", "quality", "bug", "regression",
            ],
            management_patterns: vec![
                "team", "engineer", "hire", "process", "agile",
                "sprint", "morale", "communication", "deadline",
            ],

            scaling_principles: [
                "brooks law", "horizontal scaling", "vertical scaling",
                "premature optimization", "measure first", "profile before optimize",
            ].into_iter().collect(),

            architecture_principles: [
                "strangler fig", "yagni", "kiss", "single responsibility",
                "separation of concerns", "incremental change", "small steps",
            ].into_iter().collect(),

            testing_principles: [
                "test first", "red green refactor", "fast feedback",
                "test pyramid", "mutation testing", "coverage is not quality",
            ].into_iter().collect(),

            management_principles: [
                "brooks law", "two pizza team", "conway's law",
                "mythical man month", "communication overhead", "team autonomy",
            ].into_iter().collect(),
        }
    }
}

/// Get the primary domain(s) a thinker is expert in
fn get_thinker_expert_domains(thinker: &str) -> Vec<&'static str> {
    let thinker_lower = thinker.to_lowercase();
    let mut domains = Vec::new();

    // Architecture experts
    if thinker_lower.contains("fowler")
        || thinker_lower.contains("newman")
        || thinker_lower.contains("martin")
        || thinker_lower.contains("evans")
    {
        domains.push("architecture");
    }

    // Scaling experts
    if thinker_lower.contains("vogels")
        || thinker_lower.contains("hamilton")
    {
        domains.push("scaling");
    }

    // Testing experts
    if thinker_lower.contains("beck")
        || thinker_lower.contains("humble")
        || thinker_lower.contains("feathers")
    {
        domains.push("testing");
    }

    // Management experts
    if thinker_lower.contains("brooks")
        || thinker_lower.contains("demarco")
        || thinker_lower.contains("lister")
        || thinker_lower.contains("fournier")
    {
        domains.push("management");
    }

    // Security experts
    if thinker_lower.contains("schneier")
        || thinker_lower.contains("mcgraw")
        || thinker_lower.contains("mitnick")
    {
        domains.push("security");
    }

    // Performance experts
    if thinker_lower.contains("gregg")
        || thinker_lower.contains("knuth")
        || thinker_lower.contains("carmack")
    {
        domains.push("performance");
    }

    // Database experts
    if thinker_lower.contains("stonebraker")
        || thinker_lower.contains("lamport")
    {
        domains.push("database");
    }

    // DevOps experts
    if thinker_lower.contains("humble")
        || thinker_lower.contains("kim")
    {
        domains.push("devops");
    }

    domains
}

/// Check if a thinker is an expert in a given domain
fn is_thinker_domain_expert(thinker: &str, domain: &str) -> bool {
    get_thinker_expert_domains(thinker).contains(&domain)
}

/// Check if thinker is expert in a DIFFERENT domain (cross-domain mismatch)
/// Returns true if thinker is known expert but NOT in this domain
fn is_cross_domain_expert(thinker: &str, question_domain: &str) -> bool {
    let expert_domains = get_thinker_expert_domains(thinker);
    // Only penalize if they ARE an expert, but in wrong domain
    !expert_domains.is_empty() && !expert_domains.contains(&question_domain)
}

impl GroundTruthHeuristics {
    /// Determine if a principle is appropriate for a question
    fn is_good_match(&self, question: &str, principle_name: &str) -> bool {
        let q_lower = question.to_lowercase();
        let p_lower = principle_name.to_lowercase();

        // Check scaling
        let is_scaling_question = self.scaling_patterns.iter().any(|p| q_lower.contains(p));
        let is_scaling_principle = self.scaling_principles.iter().any(|p| p_lower.contains(p));

        // Check architecture
        let is_arch_question = self.architecture_patterns.iter().any(|p| q_lower.contains(p));
        let is_arch_principle = self.architecture_principles.iter().any(|p| p_lower.contains(p));

        // Check testing
        let is_testing_question = self.testing_patterns.iter().any(|p| q_lower.contains(p));
        let is_testing_principle = self.testing_principles.iter().any(|p| p_lower.contains(p));

        // Check management
        let is_mgmt_question = self.management_patterns.iter().any(|p| q_lower.contains(p));
        let is_mgmt_principle = self.management_principles.iter().any(|p| p_lower.contains(p));

        // Good match if categories align
        (is_scaling_question && is_scaling_principle)
            || (is_arch_question && is_arch_principle)
            || (is_testing_question && is_testing_principle)
            || (is_mgmt_question && is_mgmt_principle)
    }
}

/// Generate training data for neural bandits
pub fn generate_training_data(
    conn: &Connection,
    provenance: &Provenance,
    config: &TrainingConfig,
) -> Result<TrainingBatch> {
    let engine = CounselEngine::new(conn, provenance);
    let gen_config = GeneratorConfig::default();
    let questions = generate_sample(&gen_config, config.num_questions, config.seed);

    let heuristics = GroundTruthHeuristics::default();
    let mut examples = Vec::new();
    let mut rng = StdRng::seed_from_u64(config.seed);

    let mut unique_principles: HashSet<String> = HashSet::new();
    let mut unique_thinkers: HashSet<String> = HashSet::new();
    let mut domains: HashMap<String, usize> = HashMap::new();

    for (q_idx, question) in questions.iter().enumerate() {
        if q_idx % 1000 == 0 && q_idx > 0 {
            eprintln!("Progress: {}/{}", q_idx, config.num_questions);
        }

        // Get counsel for this question
        let request = CounselRequest {
            question: question.question.clone(),
            context: CounselContext {
                domain: Some(question.domain.clone()),
                constraints: vec![],
                prefer_thinkers: vec![],
                depth: CounselDepth::Standard,
            },
            decision_id: None,  // Auto-generate UUID (training data)
        };

        let response = match engine.counsel(&request) {
            Ok(r) => r,
            Err(_) => continue, // Skip questions that fail
        };

        // Count domain
        *domains.entry(question.domain.clone()).or_insert(0) += 1;

        // Generate training examples from each position
        let total_positions = response.positions.len();

        for (rank, position) in response.positions.iter().enumerate() {
            unique_thinkers.insert(position.thinker.clone());

            for principle_id in &position.principles_cited {
                unique_principles.insert(principle_id.clone());

                // V2 improved heuristics for success prediction
                let is_good_match = heuristics.is_good_match(&question.question, principle_id);

                // Calculate success probability with improved signal
                let mut success_prob = config.base_success_rate;

                // 1. Pattern matching bonus
                if is_good_match {
                    success_prob += config.domain_match_bonus;
                }

                // 2. Counsel confidence bonus (leverage existing Thompson Sampling)
                // confidence is 0.0-1.0, scale to bonus
                let confidence_bonus = position.confidence * config.confidence_weight;
                success_prob += confidence_bonus;

                // 3. Position rank bonus - V3: EXPONENTIAL decay for 5pp+ spread
                // rank 0: +0.25, rank 1: +0.125, rank 2: +0.0625, rank 3: +0.03125
                let rank_bonus = config.relevance_bonus * config.rank_decay_base.powi(rank as i32);
                success_prob += rank_bonus;

                // 4. Domain alignment bonus
                let domain_aligned = match question.domain.as_str() {
                    "architecture" | "database" | "scaling" => {
                        position.stance == crate::types::Stance::For
                    }
                    "testing" | "security" => {
                        // These domains often benefit from cautionary advice
                        position.stance == crate::types::Stance::Against
                    }
                    _ => true, // Neutral for other domains
                };
                if domain_aligned {
                    success_prob += config.domain_match_bonus * 0.5;
                } else {
                    success_prob -= config.disagreement_penalty;
                }

                // 5. Thinker expertise bonus (some thinkers specialize in domains)
                let thinker_is_expert = is_thinker_domain_expert(&position.thinker, &question.domain);
                if thinker_is_expert {
                    success_prob += config.thinker_expertise_bonus;
                }

                // 6. V3: Cross-domain expert PENALTY (Beck on arch should FAIL)
                // This is the causal fix - experts in WRONG domain hurt outcomes
                let is_wrong_domain = is_cross_domain_expert(&position.thinker, &question.domain);
                if is_wrong_domain {
                    success_prob -= config.cross_domain_expert_penalty;
                }

                // 8. Difficulty penalty (harder questions have lower success)
                success_prob -= (question.difficulty as f64 - 2.5) * config.difficulty_penalty;

                // Add controlled noise
                let noise: f64 = rng.gen_range(-config.noise_factor..config.noise_factor);
                success_prob += noise;

                // Clamp to valid range
                success_prob = success_prob.clamp(0.05, 0.95); // Avoid extreme labels

                // Sample outcome
                let mut success = if rng.gen::<f64>() < success_prob {
                    1.0
                } else {
                    0.0
                };

                // V3: Causal adversarial flip for domain mismatches
                // This teaches the model that wrong-domain experts CAUSALLY lead to failure
                let causal_flipped = if is_wrong_domain && rng.gen::<f64>() < config.causal_adversarial_rate {
                    success = 0.0; // Force failure for wrong-domain experts
                    true
                } else {
                    false
                };

                let reasoning = format!(
                    "v3:pattern={},conf={:.2},rank={},rank_bonus={:.3},domain_aligned={},expert={},wrong_domain={},diff={},prob={:.2},causal_flip={}",
                    is_good_match, position.confidence, rank, rank_bonus, domain_aligned, thinker_is_expert,
                    is_wrong_domain, question.difficulty, success_prob, causal_flipped
                );

                let example = TrainingExample {
                    id: format!("ex-{}-{}-{}", q_idx, rank, principle_id),
                    question: question.question.clone(),
                    domain: question.domain.clone(),
                    difficulty: question.difficulty,
                    principle_id: principle_id.clone(),
                    principle_name: principle_id.clone(), // TODO: lookup actual name
                    thinker_id: position.thinker.clone(),
                    position_rank: rank,
                    confidence: position.confidence,
                    success,
                    reasoning,
                    context_features: ContextFeatures {
                        stakeholder: question.stakeholder.clone(),
                        company_stage: question.company_stage.clone(),
                        urgency: question.urgency.clone(),
                        domain_match: domain_aligned,
                        total_principles_selected: total_positions,
                        is_for_position: position.stance == crate::types::Stance::For,
                    },
                };

                examples.push(example);
            }
        }
    }

    let positive = examples.iter().filter(|e| e.success > 0.5).count();
    let negative = examples.len() - positive;

    let metadata = BatchMetadata {
        total_examples: examples.len(),
        positive_examples: positive,
        negative_examples: negative,
        unique_questions: questions.len(),
        unique_principles: unique_principles.len(),
        unique_thinkers: unique_thinkers.len(),
        domains,
        avg_examples_per_question: examples.len() as f64 / questions.len().max(1) as f64,
        generation_seed: config.seed,
    };

    Ok(TrainingBatch { examples, metadata })
}

/// Export training data to JSONL format (one example per line)
pub fn export_to_jsonl(batch: &TrainingBatch, path: &std::path::Path) -> Result<()> {
    use std::io::Write;

    let file = std::fs::File::create(path)?;
    let mut writer = std::io::BufWriter::new(file);

    for example in &batch.examples {
        let line = serde_json::to_string(example)?;
        writeln!(writer, "{}", line)?;
    }

    Ok(())
}

/// Export training data to CSV format for ML frameworks
pub fn export_to_csv(batch: &TrainingBatch, path: &std::path::Path) -> Result<()> {
    use std::io::Write;

    let file = std::fs::File::create(path)?;
    let mut writer = std::io::BufWriter::new(file);

    // Header
    writeln!(
        writer,
        "id,domain,difficulty,position_rank,confidence,stakeholder,company_stage,urgency,domain_match,total_selected,is_for,success"
    )?;

    for ex in &batch.examples {
        writeln!(
            writer,
            "{},{},{},{},{:.4},{},{},{},{},{},{},{}",
            ex.id,
            ex.domain,
            ex.difficulty,
            ex.position_rank,
            ex.confidence,
            ex.context_features.stakeholder,
            ex.context_features.company_stage,
            ex.context_features.urgency,
            if ex.context_features.domain_match { 1 } else { 0 },
            ex.context_features.total_principles_selected,
            if ex.context_features.is_for_position { 1 } else { 0 },
            ex.success as i32,
        )?;
    }

    Ok(())
}

/// Print training batch summary
pub fn print_batch_summary(batch: &TrainingBatch) {
    println!("\n┌─────────────────────────────────────────────────────────────┐");
    println!("│ 🧠 NEURAL BANDIT TRAINING DATA                              │");
    println!("└─────────────────────────────────────────────────────────────┘\n");

    let meta = &batch.metadata;

    println!("OVERVIEW:");
    println!("   Total examples:     {:>8}", meta.total_examples);
    println!("   Positive (success): {:>8} ({:.1}%)",
             meta.positive_examples,
             100.0 * meta.positive_examples as f64 / meta.total_examples.max(1) as f64);
    println!("   Negative (failure): {:>8} ({:.1}%)",
             meta.negative_examples,
             100.0 * meta.negative_examples as f64 / meta.total_examples.max(1) as f64);

    println!("\nDIVERSITY:");
    println!("   Unique questions:   {:>8}", meta.unique_questions);
    println!("   Unique principles:  {:>8}", meta.unique_principles);
    println!("   Unique thinkers:    {:>8}", meta.unique_thinkers);
    println!("   Avg examples/q:     {:>8.1}", meta.avg_examples_per_question);

    println!("\nDOMAINS:");
    let mut domains: Vec<_> = meta.domains.iter().collect();
    domains.sort_by(|a, b| b.1.cmp(a.1));
    for (domain, count) in domains.iter().take(10) {
        let pct = 100.0 * (**count as f64) / meta.unique_questions.max(1) as f64;
        let bar = "█".repeat((pct / 5.0) as usize);
        println!("   {:20} {:>5} ({:>5.1}%) {}", domain, count, pct, bar);
    }

    println!("\nSEED: {}", meta.generation_seed);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ground_truth_heuristics() {
        let h = GroundTruthHeuristics::default();

        // Scaling question should match scaling principles
        assert!(h.is_good_match(
            "We need to handle 100x traffic",
            "Brooks Law"
        ));

        // Architecture question should match architecture principles
        assert!(h.is_good_match(
            "Should we rewrite our monolith?",
            "Strangler Fig Pattern"
        ));
    }

    #[test]
    fn test_training_config_default() {
        let config = TrainingConfig::default();
        assert_eq!(config.num_questions, 10_000);
        assert!(config.base_success_rate > 0.0 && config.base_success_rate < 1.0);
    }
}
