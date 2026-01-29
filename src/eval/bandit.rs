//! Advanced Bandit Optimization for Principle Selection
//!
//! Implements:
//! - Feel-Good Thompson Sampling (FG-TS) for aggressive exploration
//! - Contextual arms with domain-specific learning
//! - UCB exploration bonus for undersampled principles
//! - Bayesian hyperparameter optimization

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Beta distribution posterior for Thompson Sampling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BetaPosterior {
    /// Success count + prior (α)
    pub alpha: f64,
    /// Failure count + prior (β)
    pub beta: f64,
    /// Total samples observed
    pub sample_count: u64,
    /// Last update timestamp
    pub last_updated: Option<String>,
}

impl Default for BetaPosterior {
    fn default() -> Self {
        Self {
            alpha: 1.0, // Uniform prior
            beta: 1.0,
            sample_count: 0,
            last_updated: None,
        }
    }
}

impl BetaPosterior {
    /// Create with custom priors
    pub fn with_priors(alpha: f64, beta: f64) -> Self {
        Self {
            alpha,
            beta,
            sample_count: 0,
            last_updated: None,
        }
    }

    /// Sample from the Beta distribution using Box-Muller transform approximation
    pub fn sample(&self, seed: u64) -> f64 {
        // Deterministic pseudo-random based on seed
        let u1 = pseudo_random(seed) as f64 / u64::MAX as f64;
        let u2 = pseudo_random(seed.wrapping_add(1)) as f64 / u64::MAX as f64;

        // Approximation of Beta sample using inverse CDF
        // For alpha, beta close to 1, use simpler approximation
        if self.alpha >= 1.0 && self.beta >= 1.0 {
            beta_sample_approx(self.alpha, self.beta, u1, u2)
        } else {
            // Fallback to mean for edge cases
            self.mean()
        }
    }

    /// Update posterior with observation
    pub fn update(&mut self, success: bool) {
        if success {
            self.alpha += 1.0;
        } else {
            self.beta += 1.0;
        }
        self.sample_count += 1;
        self.last_updated = Some(chrono::Utc::now().to_rfc3339());
    }

    /// Posterior mean (expected success rate)
    pub fn mean(&self) -> f64 {
        self.alpha / (self.alpha + self.beta)
    }

    /// Posterior variance
    pub fn variance(&self) -> f64 {
        let sum = self.alpha + self.beta;
        (self.alpha * self.beta) / (sum * sum * (sum + 1.0))
    }

    /// 95% credible interval width (measure of uncertainty)
    pub fn credible_interval_width(&self) -> f64 {
        // Approximation: 2 * 1.96 * sqrt(variance)
        3.92 * self.variance().sqrt()
    }
}

/// Simple pseudo-random number generator (xorshift)
fn pseudo_random(seed: u64) -> u64 {
    let mut x = seed.wrapping_add(1);
    x ^= x >> 12;
    x ^= x << 25;
    x ^= x >> 27;
    x.wrapping_mul(0x2545F4914F6CDD1D)
}

/// Approximate Beta sample using the Johnk algorithm idea
fn beta_sample_approx(alpha: f64, beta: f64, u1: f64, u2: f64) -> f64 {
    // For large alpha, beta: use normal approximation
    if alpha > 10.0 && beta > 10.0 {
        let mean = alpha / (alpha + beta);
        let var = (alpha * beta) / ((alpha + beta).powi(2) * (alpha + beta + 1.0));
        // Box-Muller for normal
        let z = (-2.0 * u1.max(1e-10).ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
        return (mean + z * var.sqrt()).clamp(0.0, 1.0);
    }

    // For smaller parameters: Johnk's algorithm approximation
    let x = u1.powf(1.0 / alpha);
    let y = u2.powf(1.0 / beta);
    let sum = x + y;

    if sum <= 1.0 {
        x / sum
    } else {
        // Retry logic replaced with fallback
        alpha / (alpha + beta) // Fallback to mean
    }
}

/// Feel-Good Thompson Sampling with exploration bonus
pub struct FeelGoodThompsonSampler {
    /// Per-principle posteriors
    pub arms: HashMap<String, BetaPosterior>,
    /// Per-(principle, domain) posteriors for contextual learning
    pub contextual_arms: HashMap<(String, String), BetaPosterior>,
    /// Exploration coefficient (higher = more exploration)
    pub exploration_c: f64,
    /// Total samples across all arms (for UCB calculation)
    pub total_samples: u64,
}

impl Default for FeelGoodThompsonSampler {
    fn default() -> Self {
        Self {
            arms: HashMap::new(),
            contextual_arms: HashMap::new(),
            exploration_c: 2.0,
            total_samples: 0,
        }
    }
}

impl FeelGoodThompsonSampler {
    /// Sample with Feel-Good TS: adds exploration bonus for undersampled arms
    pub fn sample(&self, principle_id: &str, domain: Option<&str>, seed: u64) -> f64 {
        // Get base Thompson sample
        let base_sample = if let Some(d) = domain {
            // Use contextual arm if available
            self.contextual_arms
                .get(&(principle_id.to_string(), d.to_string()))
                .map(|arm| arm.sample(seed))
                .unwrap_or_else(|| {
                    // Fall back to global arm
                    self.arms
                        .get(principle_id)
                        .map(|arm| arm.sample(seed))
                        .unwrap_or(0.5)
                })
        } else {
            self.arms
                .get(principle_id)
                .map(|arm| arm.sample(seed))
                .unwrap_or(0.5)
        };

        // Feel-Good exploration bonus
        let sample_count = self.get_sample_count(principle_id, domain);
        let fg_bonus = self.feel_good_bonus(sample_count);

        (base_sample + fg_bonus).min(1.0)
    }

    /// Feel-Good exploration bonus
    fn feel_good_bonus(&self, sample_count: u64) -> f64 {
        if sample_count == 0 || self.total_samples == 0 {
            return self.exploration_c * 0.5; // High bonus for unseen arms
        }

        // UCB-style exploration bonus
        let bonus =
            self.exploration_c * ((self.total_samples as f64).ln() / (sample_count as f64)).sqrt();

        // Cap the bonus
        bonus.min(0.3)
    }

    /// Get sample count for an arm
    fn get_sample_count(&self, principle_id: &str, domain: Option<&str>) -> u64 {
        if let Some(d) = domain {
            self.contextual_arms
                .get(&(principle_id.to_string(), d.to_string()))
                .map(|arm| arm.sample_count)
                .unwrap_or(0)
        } else {
            self.arms
                .get(principle_id)
                .map(|arm| arm.sample_count)
                .unwrap_or(0)
        }
    }

    /// Update arm with observation
    pub fn update(&mut self, principle_id: &str, domain: Option<&str>, success: bool) {
        // Update global arm
        self.arms
            .entry(principle_id.to_string())
            .or_default()
            .update(success);

        // Update contextual arm if domain provided
        if let Some(d) = domain {
            self.contextual_arms
                .entry((principle_id.to_string(), d.to_string()))
                .or_default()
                .update(success);
        }

        self.total_samples += 1;
    }

    /// Get top-k arms by Thompson sample
    pub fn top_k(&self, k: usize, domain: Option<&str>, seed: u64) -> Vec<(String, f64)> {
        let mut samples: Vec<(String, f64)> = self
            .arms
            .keys()
            .map(|id| {
                (
                    id.clone(),
                    self.sample(id, domain, seed.wrapping_add(hash_str(id))),
                )
            })
            .collect();

        samples.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        samples.truncate(k);
        samples
    }

    /// Identify arms that need more exploration
    pub fn underexplored_arms(&self, threshold: u64) -> Vec<String> {
        self.arms
            .iter()
            .filter(|(_, arm)| arm.sample_count < threshold)
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Identify consistently poor-performing arms
    pub fn poor_performers(&self, threshold: f64) -> Vec<String> {
        self.arms
            .iter()
            .filter(|(_, arm)| arm.sample_count >= 20 && arm.mean() < threshold)
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Get confidence-ranked arms
    pub fn confidence_ranking(&self) -> Vec<(String, f64, f64)> {
        let mut ranked: Vec<(String, f64, f64)> = self
            .arms
            .iter()
            .map(|(id, arm)| (id.clone(), arm.mean(), arm.credible_interval_width()))
            .collect();

        // Sort by mean - uncertainty (pessimistic bound)
        ranked.sort_by(|a, b| {
            let a_pessimistic = a.1 - a.2 / 2.0;
            let b_pessimistic = b.1 - b.2 / 2.0;
            b_pessimistic
                .partial_cmp(&a_pessimistic)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        ranked
    }
}

fn hash_str(s: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    hasher.finish()
}

/// Hyperparameter configuration for scoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoringHyperparameters {
    // FTS weights
    pub fts_weight: f64,
    pub semantic_weight: f64,
    pub rrf_k: f64,

    // Domain boost weights
    pub domain_boost_architecture: f64,
    pub domain_boost_testing: f64,
    pub domain_boost_scaling: f64,
    pub domain_boost_management: f64,
    pub domain_boost_performance: f64,

    // Thompson Sampling params
    pub ts_prior_alpha: f64,
    pub ts_prior_beta: f64,
    pub exploration_c: f64,

    // Temporal decay
    pub decay_lambda: f64,
}

impl Default for ScoringHyperparameters {
    fn default() -> Self {
        Self {
            fts_weight: 1.0,
            semantic_weight: 0.7,
            rrf_k: 60.0,

            domain_boost_architecture: 20.0,
            domain_boost_testing: 25.0,
            domain_boost_scaling: 15.0,
            domain_boost_management: 10.0,
            domain_boost_performance: 15.0,

            ts_prior_alpha: 1.0,
            ts_prior_beta: 1.0,
            exploration_c: 2.0,

            decay_lambda: 0.95,
        }
    }
}

/// Define the hyperparameter search space
#[derive(Debug, Clone)]
pub struct HyperparameterSpace {
    pub fts_weight: (f64, f64),
    pub semantic_weight: (f64, f64),
    pub rrf_k: (f64, f64),
    pub domain_boost_range: (f64, f64),
    pub ts_prior_range: (f64, f64),
    pub exploration_c_range: (f64, f64),
    pub decay_lambda_range: (f64, f64),
}

impl Default for HyperparameterSpace {
    fn default() -> Self {
        Self {
            fts_weight: (0.5, 2.0),
            semantic_weight: (0.0, 1.0),
            rrf_k: (20.0, 100.0),
            domain_boost_range: (5.0, 50.0),
            ts_prior_range: (0.1, 10.0),
            exploration_c_range: (0.5, 5.0),
            decay_lambda_range: (0.8, 0.99),
        }
    }
}

impl HyperparameterSpace {
    /// Sample a random configuration from the space
    pub fn sample(&self, seed: u64) -> ScoringHyperparameters {
        let mut params = ScoringHyperparameters::default();

        params.fts_weight = uniform_sample(self.fts_weight.0, self.fts_weight.1, seed);
        params.semantic_weight = uniform_sample(
            self.semantic_weight.0,
            self.semantic_weight.1,
            seed.wrapping_add(1),
        );
        params.rrf_k = uniform_sample(self.rrf_k.0, self.rrf_k.1, seed.wrapping_add(2));

        params.domain_boost_architecture = uniform_sample(
            self.domain_boost_range.0,
            self.domain_boost_range.1,
            seed.wrapping_add(3),
        );
        params.domain_boost_testing = uniform_sample(
            self.domain_boost_range.0,
            self.domain_boost_range.1,
            seed.wrapping_add(4),
        );
        params.domain_boost_scaling = uniform_sample(
            self.domain_boost_range.0,
            self.domain_boost_range.1,
            seed.wrapping_add(5),
        );
        params.domain_boost_management = uniform_sample(
            self.domain_boost_range.0,
            self.domain_boost_range.1,
            seed.wrapping_add(6),
        );
        params.domain_boost_performance = uniform_sample(
            self.domain_boost_range.0,
            self.domain_boost_range.1,
            seed.wrapping_add(7),
        );

        params.ts_prior_alpha = uniform_sample(
            self.ts_prior_range.0,
            self.ts_prior_range.1,
            seed.wrapping_add(8),
        );
        params.ts_prior_beta = uniform_sample(
            self.ts_prior_range.0,
            self.ts_prior_range.1,
            seed.wrapping_add(9),
        );
        params.exploration_c = uniform_sample(
            self.exploration_c_range.0,
            self.exploration_c_range.1,
            seed.wrapping_add(10),
        );

        params.decay_lambda = uniform_sample(
            self.decay_lambda_range.0,
            self.decay_lambda_range.1,
            seed.wrapping_add(11),
        );

        params
    }
}

fn uniform_sample(min: f64, max: f64, seed: u64) -> f64 {
    let u = pseudo_random(seed) as f64 / u64::MAX as f64;
    min + u * (max - min)
}

/// Simple Bayesian optimization state (Gaussian Process approximation)
#[derive(Debug, Clone)]
pub struct BayesianOptimizer {
    /// Observed points: (params, score)
    pub observations: Vec<(ScoringHyperparameters, f64)>,
    /// Best observed so far
    pub best_params: Option<ScoringHyperparameters>,
    pub best_score: f64,
    /// Search space
    pub space: HyperparameterSpace,
}

impl BayesianOptimizer {
    pub fn new(space: HyperparameterSpace) -> Self {
        Self {
            observations: Vec::new(),
            best_params: None,
            best_score: 0.0,
            space,
        }
    }

    /// Suggest next point to evaluate (Thompson Sampling over GP posterior)
    pub fn suggest_next(&self, seed: u64) -> ScoringHyperparameters {
        if self.observations.len() < 5 {
            // Random exploration in early phase
            return self.space.sample(seed);
        }

        // Sample multiple candidates and pick best expected improvement
        let mut best_candidate = self.space.sample(seed);
        let mut best_ei = 0.0;

        for i in 0..20 {
            let candidate = self.space.sample(seed.wrapping_add(i));
            let ei = self.expected_improvement(&candidate);
            if ei > best_ei {
                best_ei = ei;
                best_candidate = candidate;
            }
        }

        best_candidate
    }

    /// Approximate Expected Improvement
    fn expected_improvement(&self, _params: &ScoringHyperparameters) -> f64 {
        // Simplified: just return random for now
        // Full implementation would use GP posterior
        // This is a placeholder for the full Bayesian optimization
        pseudo_random(hash_str(&format!("{:?}", _params))) as f64 / u64::MAX as f64
    }

    /// Record observation
    pub fn observe(&mut self, params: ScoringHyperparameters, score: f64) {
        if score > self.best_score {
            self.best_score = score;
            self.best_params = Some(params.clone());
        }
        self.observations.push((params, score));
    }

    /// Get optimization report
    pub fn report(&self) -> OptimizationReport {
        let scores: Vec<f64> = self.observations.iter().map(|(_, s)| *s).collect();
        let mean_score = scores.iter().sum::<f64>() / scores.len().max(1) as f64;

        // Improvement over iterations
        let improvements: Vec<f64> = scores.windows(2).map(|w| w[1] - w[0]).collect();

        OptimizationReport {
            iterations: self.observations.len(),
            best_score: self.best_score,
            mean_score,
            score_variance: scores.iter().map(|s| (s - mean_score).powi(2)).sum::<f64>()
                / scores.len().max(1) as f64,
            improvement_trend: improvements.iter().sum::<f64>() / improvements.len().max(1) as f64,
            best_params: self.best_params.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationReport {
    pub iterations: usize,
    pub best_score: f64,
    pub mean_score: f64,
    pub score_variance: f64,
    pub improvement_trend: f64,
    pub best_params: Option<ScoringHyperparameters>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_beta_posterior() {
        let mut arm = BetaPosterior::default();

        // Initial state
        assert_eq!(arm.mean(), 0.5);

        // After successes
        arm.update(true);
        arm.update(true);
        arm.update(false);

        assert!((arm.mean() - 0.6).abs() < 0.1); // ~3/5
    }

    #[test]
    fn test_feel_good_sampler() {
        let mut sampler = FeelGoodThompsonSampler::default();

        // Add some arms
        for i in 0..10 {
            let id = format!("principle-{}", i);
            sampler.arms.insert(id.clone(), BetaPosterior::default());
        }

        // Sample should work
        let sample = sampler.sample("principle-0", None, 42);
        assert!(sample >= 0.0 && sample <= 1.0);

        // Update and check
        sampler.update("principle-0", Some("architecture"), true);
        assert_eq!(sampler.total_samples, 1);
    }

    #[test]
    fn test_hyperparameter_sampling() {
        let space = HyperparameterSpace::default();
        let params = space.sample(123);

        assert!(params.fts_weight >= space.fts_weight.0);
        assert!(params.fts_weight <= space.fts_weight.1);
        assert!(params.semantic_weight >= 0.0 && params.semantic_weight <= 1.0);
    }

    #[test]
    fn test_bayesian_optimizer() {
        let space = HyperparameterSpace::default();
        let mut optimizer = BayesianOptimizer::new(space);

        // Run a few iterations
        for i in 0..10 {
            let params = optimizer.suggest_next(i as u64);
            let score = 0.5 + (i as f64 * 0.01); // Fake improving score
            optimizer.observe(params, score);
        }

        let report = optimizer.report();
        assert_eq!(report.iterations, 10);
        assert!(report.best_score > 0.5);
    }
}
