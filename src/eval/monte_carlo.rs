//! Monte Carlo Simulation for Principle Selection Analysis
//!
//! Runs thousands of simulated queries to understand:
//! - Which principles are over/under-selected
//! - Selection variance and stability
//! - Tail risk (poor recommendations)

use crate::counsel::CounselEngine;
use crate::db::PrincipleMatch;
use crate::provenance::Provenance;
use crate::types::*;
use anyhow::Result;
use rand::prelude::*;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration for Monte Carlo simulation
#[derive(Debug, Clone)]
pub struct MonteCarloConfig {
    /// Number of simulations to run
    pub num_simulations: u32,

    /// Question templates to use
    pub question_templates: Vec<QuestionTemplate>,

    /// User behavior model for outcome simulation
    pub user_behavior: UserBehaviorModel,

    /// Random seed for reproducibility (None = random)
    pub seed: Option<u64>,
}

impl Default for MonteCarloConfig {
    fn default() -> Self {
        Self {
            num_simulations: 1000,
            question_templates: default_question_templates(),
            user_behavior: UserBehaviorModel::default(),
            seed: None,
        }
    }
}

/// Template for generating random questions
#[derive(Debug, Clone)]
pub struct QuestionTemplate {
    pub category: String,
    pub template: String,
    pub weight: f64, // Probability weight for this template
}

/// Model of how users respond to recommendations
#[derive(Debug, Clone)]
pub struct UserBehaviorModel {
    /// Base probability of accepting any recommendation
    pub base_acceptance: f64,

    /// How much relevance affects acceptance (0-1)
    pub relevance_weight: f64,

    /// How much confidence affects acceptance (0-1)
    pub confidence_weight: f64,
}

impl Default for UserBehaviorModel {
    fn default() -> Self {
        Self {
            base_acceptance: 0.5,
            relevance_weight: 0.3,
            confidence_weight: 0.2,
        }
    }
}

/// Results from Monte Carlo simulation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonteCarloResults {
    pub num_simulations: u32,

    /// Selection rate for each principle (principle_name -> rate)
    pub principle_selection_rates: HashMap<String, f64>,

    /// Selection rate for each thinker
    pub thinker_selection_rates: HashMap<String, f64>,

    /// 95% confidence interval for mean relevance
    pub confidence_interval_95: (f64, f64),

    /// Variance in selection (higher = less stable)
    pub selection_variance: f64,

    /// Tail risk: probability of <50% relevance
    pub tail_risk: f64,

    /// Simulated outcome distribution
    pub simulated_outcomes: OutcomeDistribution,
}

/// Distribution of simulated outcomes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutcomeDistribution {
    pub success_rate: f64,
    pub partial_success_rate: f64,
    pub failure_rate: f64,
}

/// Run Monte Carlo simulation
pub fn run_simulation(
    conn: &Connection,
    provenance: &Provenance,
    config: &MonteCarloConfig,
) -> Result<MonteCarloResults> {
    let mut rng = match config.seed {
        Some(seed) => StdRng::seed_from_u64(seed),
        None => StdRng::from_entropy(),
    };

    let engine = CounselEngine::new(conn, provenance);

    // Track selections across all simulations
    let mut principle_counts: HashMap<String, u32> = HashMap::new();
    let mut thinker_counts: HashMap<String, u32> = HashMap::new();
    let mut relevance_scores: Vec<f64> = Vec::new();
    let mut outcome_success = 0u32;
    let mut outcome_partial = 0u32;
    let mut outcome_fail = 0u32;

    // Get all principles for baseline
    let all_principles = get_all_principles(conn)?;
    for p in &all_principles {
        principle_counts.insert(p.name.clone(), 0);
    }

    // Run simulations
    for _ in 0..config.num_simulations {
        // Generate random question
        let question = generate_random_question(&config.question_templates, &mut rng);

        // Get counsel
        let request = CounselRequest {
            question: question.clone(),
            context: CounselContext::default(),
            decision_id: None,  // Auto-generate UUID (Monte Carlo sim)
        };

        // Run counsel (but don't store decisions to avoid polluting DB)
        let response = match engine.counsel(&request) {
            Ok(r) => r,
            Err(_) => continue, // Skip failed queries
        };

        // Track selections
        let mut sim_relevance = 0.0;
        let mut total_confidence = 0.0;

        for position in &response.positions {
            for principle in &position.principles_cited {
                *principle_counts.entry(principle.clone()).or_insert(0) += 1;
            }
            *thinker_counts.entry(position.thinker.clone()).or_insert(0) += 1;

            // Estimate relevance based on confidence (proxy for ground truth)
            sim_relevance += position.confidence;
            total_confidence += 1.0;
        }

        let avg_relevance = if total_confidence > 0.0 {
            sim_relevance / total_confidence
        } else {
            0.0
        };
        relevance_scores.push(avg_relevance);

        // Simulate user outcome based on behavior model
        let acceptance_prob = config.user_behavior.base_acceptance
            + config.user_behavior.relevance_weight * avg_relevance
            + config.user_behavior.confidence_weight * (sim_relevance / 4.0); // Normalize

        let outcome_roll: f64 = rng.gen();
        if outcome_roll < acceptance_prob * 0.6 {
            outcome_success += 1;
        } else if outcome_roll < acceptance_prob {
            outcome_partial += 1;
        } else {
            outcome_fail += 1;
        }
    }

    // Compute statistics
    let n = config.num_simulations as f64;

    let principle_rates: HashMap<String, f64> = principle_counts
        .into_iter()
        .map(|(k, v)| (k, v as f64 / n))
        .collect();

    let thinker_rates: HashMap<String, f64> = thinker_counts
        .into_iter()
        .map(|(k, v)| (k, v as f64 / n))
        .collect();

    // Relevance statistics
    let mean_relevance = relevance_scores.iter().sum::<f64>() / relevance_scores.len() as f64;
    let variance = relevance_scores
        .iter()
        .map(|r| (r - mean_relevance).powi(2))
        .sum::<f64>()
        / relevance_scores.len() as f64;
    let std_dev = variance.sqrt();

    // 95% CI: mean ± 1.96 * std_error
    let std_error = std_dev / (relevance_scores.len() as f64).sqrt();
    let ci_95 = (
        mean_relevance - 1.96 * std_error,
        mean_relevance + 1.96 * std_error,
    );

    // Tail risk
    let below_50 = relevance_scores.iter().filter(|&&r| r < 0.5).count();
    let tail_risk = below_50 as f64 / relevance_scores.len() as f64;

    // Selection variance (how much does the selection vary?)
    let selection_variance = compute_selection_variance(&principle_rates);

    // Outcome distribution
    let total_outcomes = (outcome_success + outcome_partial + outcome_fail) as f64;
    let outcomes = OutcomeDistribution {
        success_rate: outcome_success as f64 / total_outcomes,
        partial_success_rate: outcome_partial as f64 / total_outcomes,
        failure_rate: outcome_fail as f64 / total_outcomes,
    };

    Ok(MonteCarloResults {
        num_simulations: config.num_simulations,
        principle_selection_rates: principle_rates,
        thinker_selection_rates: thinker_rates,
        confidence_interval_95: ci_95,
        selection_variance,
        tail_risk,
        simulated_outcomes: outcomes,
    })
}

/// Generate a random question from templates
fn generate_random_question(templates: &[QuestionTemplate], rng: &mut StdRng) -> String {
    // Weighted random selection
    let total_weight: f64 = templates.iter().map(|t| t.weight).sum();
    let mut roll: f64 = rng.gen::<f64>() * total_weight;

    for template in templates {
        roll -= template.weight;
        if roll <= 0.0 {
            return apply_template_variations(&template.template, rng);
        }
    }

    // Fallback
    "Should we add this feature?".to_string()
}

/// Apply random variations to a template
fn apply_template_variations(template: &str, rng: &mut StdRng) -> String {
    let techs = [
        "Redis",
        "Kafka",
        "PostgreSQL",
        "MongoDB",
        "Elasticsearch",
        "GraphQL",
    ];
    let team_sizes = ["3", "5", "10", "20", "50"];
    let deadlines = ["2 weeks", "1 month", "3 months", "6 months"];

    let mut result = template.to_string();

    if result.contains("{tech}") {
        result = result.replace("{tech}", techs.choose(rng).unwrap());
    }
    if result.contains("{size}") {
        result = result.replace("{size}", team_sizes.choose(rng).unwrap());
    }
    if result.contains("{deadline}") {
        result = result.replace("{deadline}", deadlines.choose(rng).unwrap());
    }

    result
}

/// Compute variance in selection rates
fn compute_selection_variance(rates: &HashMap<String, f64>) -> f64 {
    if rates.is_empty() {
        return 0.0;
    }

    let mean: f64 = rates.values().sum::<f64>() / rates.len() as f64;
    let variance: f64 =
        rates.values().map(|r| (r - mean).powi(2)).sum::<f64>() / rates.len() as f64;

    variance.sqrt() // Return std dev for interpretability
}

/// Get all principles from database
fn get_all_principles(conn: &Connection) -> Result<Vec<PrincipleMatch>> {
    let mut stmt = conn
        .prepare("SELECT id, thinker_id, name, description, learned_confidence FROM principles")?;

    let principles = stmt
        .query_map([], |row| {
            Ok(PrincipleMatch {
                id: row.get(0)?,
                thinker_id: row.get(1)?,
                name: row.get(2)?,
                description: row.get(3)?,
                confidence: row.get(4)?,
                relevance_score: 0.0,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(principles)
}

/// Default question templates covering common decision types
fn default_question_templates() -> Vec<QuestionTemplate> {
    vec![
        // Architecture decisions
        QuestionTemplate {
            category: "architecture".to_string(),
            template: "Should we move to microservices?".to_string(),
            weight: 2.0,
        },
        QuestionTemplate {
            category: "architecture".to_string(),
            template: "Should we add {tech} to our stack?".to_string(),
            weight: 2.0,
        },
        QuestionTemplate {
            category: "architecture".to_string(),
            template: "Should we build a custom {tech} solution?".to_string(),
            weight: 1.5,
        },
        // Scaling decisions
        QuestionTemplate {
            category: "scaling".to_string(),
            template: "We need to handle 10x more traffic. How should we approach this?"
                .to_string(),
            weight: 1.5,
        },
        QuestionTemplate {
            category: "scaling".to_string(),
            template: "Should we add more engineers to meet the {deadline} deadline?".to_string(),
            weight: 2.0,
        },
        // Rewrite decisions
        QuestionTemplate {
            category: "rewrite".to_string(),
            template: "Should we rewrite the legacy system from scratch?".to_string(),
            weight: 2.0,
        },
        QuestionTemplate {
            category: "rewrite".to_string(),
            template: "The codebase is getting hard to maintain. Should we refactor?".to_string(),
            weight: 1.5,
        },
        // Performance decisions
        QuestionTemplate {
            category: "performance".to_string(),
            template: "Should we add a caching layer with {tech}?".to_string(),
            weight: 1.5,
        },
        QuestionTemplate {
            category: "performance".to_string(),
            template: "Our API is slow. Should we optimize now or later?".to_string(),
            weight: 1.5,
        },
        // Team decisions
        QuestionTemplate {
            category: "team".to_string(),
            template: "Should we split the team of {size} into smaller squads?".to_string(),
            weight: 1.5,
        },
        QuestionTemplate {
            category: "team".to_string(),
            template: "The project is behind schedule. Should we add contractors?".to_string(),
            weight: 1.5,
        },
        // Feature decisions
        QuestionTemplate {
            category: "feature".to_string(),
            template: "Should we add this feature customers are requesting?".to_string(),
            weight: 2.0,
        },
        QuestionTemplate {
            category: "feature".to_string(),
            template: "Should we build authentication in-house or use Auth0?".to_string(),
            weight: 1.5,
        },
        // Process decisions
        QuestionTemplate {
            category: "process".to_string(),
            template: "Should we adopt TDD for this project?".to_string(),
            weight: 1.0,
        },
        QuestionTemplate {
            category: "process".to_string(),
            template: "Should we switch from Scrum to Kanban?".to_string(),
            weight: 1.0,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_selection_variance() {
        let mut rates = HashMap::new();
        rates.insert("a".to_string(), 0.5);
        rates.insert("b".to_string(), 0.5);
        rates.insert("c".to_string(), 0.5);

        // All equal -> low variance
        let var = compute_selection_variance(&rates);
        assert!(var < 0.01);

        // Unequal -> higher variance
        rates.insert("d".to_string(), 0.0);
        rates.insert("e".to_string(), 1.0);
        let var2 = compute_selection_variance(&rates);
        assert!(var2 > var);
    }
}
