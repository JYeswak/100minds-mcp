//! Thompson Sampling for Principle Optimization
//!
//! Replaces fixed confidence adjustments (+0.05/-0.08) with
//! proper Bayesian inference using Beta distributions.
//!
//! Benefits:
//! - Natural exploration/exploitation trade-off
//! - Uncertainty quantification (wide CI = try more)
//! - Context-aware learning (per-domain statistics)

use anyhow::Result;
use rand::prelude::*;
use rand::seq::SliceRandom;
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use statrs::distribution::{Beta, ContinuousCDF};
use std::collections::HashMap;

/// Configuration for Feel-Good Thompson Sampling
#[derive(Debug, Clone)]
pub struct FGTSConfig {
    /// Optimism constant (c in bonus = c / sqrt(n))
    pub optimism_constant: f64,
    /// Decay factor for bonus (0.95 = 5% decay per pull)
    pub bonus_decay: f64,
    /// Epsilon for hybrid exploration (probability of random selection)
    pub epsilon: f64,
    /// Minimum pulls before disabling epsilon exploration
    pub epsilon_threshold: u32,
}

impl Default for FGTSConfig {
    fn default() -> Self {
        Self {
            optimism_constant: 2.0,  // Per NeurIPS 2025 paper
            bonus_decay: 0.95,       // Decay to prevent perpetual optimism
            epsilon: 0.1,            // 10% random exploration for cold-start
            epsilon_threshold: 10,   // After 10 pulls, disable epsilon
        }
    }
}

/// A single principle arm in the multi-armed bandit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrincipleArm {
    /// Principle ID
    pub id: String,

    /// Principle name (for display)
    pub name: String,

    /// Alpha parameter (successes + 1)
    pub alpha: f64,

    /// Beta parameter (failures + 1)
    pub beta: f64,

    /// Number of times this arm has been pulled (for FG-TS decay)
    #[serde(default)]
    pub pulls: u32,
}

impl PrincipleArm {
    /// Create a new arm with uninformative prior (α=1, β=1)
    pub fn new(id: String, name: String) -> Self {
        Self {
            id,
            name,
            alpha: 1.0,
            beta: 1.0,
            pulls: 0,
        }
    }

    /// Create from existing confidence (convert point estimate to Beta params)
    pub fn from_confidence(id: String, name: String, confidence: f64, sample_size: f64) -> Self {
        // Use confidence as mean of Beta, with specified equivalent sample size
        // Mean of Beta(α, β) = α / (α + β)
        // So α = confidence * sample_size, β = (1 - confidence) * sample_size
        let alpha = (confidence * sample_size).max(1.0);
        let beta = ((1.0 - confidence) * sample_size).max(1.0);

        Self { id, name, alpha, beta, pulls: sample_size as u32 }
    }

    /// Standard Thompson Sampling: Sample from the Beta distribution
    pub fn sample(&self, rng: &mut impl Rng) -> f64 {
        // Use inverse CDF sampling for Beta distribution
        let u: f64 = rng.gen();
        match Beta::new(self.alpha, self.beta) {
            Ok(dist) => dist.inverse_cdf(u),
            Err(_) => self.mean(), // Fallback to mean if params invalid
        }
    }

    /// Feel-Good Thompson Sampling: Add optimism bonus for undersampled arms
    /// This solves the cold-start problem for orphan principles
    /// Formula: bonus = (c / sqrt(α + β)) * decay^pulls
    /// Per NeurIPS 2025 (arXiv 2507.15290): 25% improvement over vanilla TS
    pub fn fg_sample(&self, rng: &mut impl Rng, config: &FGTSConfig) -> f64 {
        // Calculate optimism bonus (decays with pulls)
        let n = self.alpha + self.beta;
        let raw_bonus = config.optimism_constant / n.sqrt();
        let decayed_bonus = raw_bonus * config.bonus_decay.powi(self.pulls as i32);
        let bonus = decayed_bonus.min(0.5); // Cap bonus at 0.5

        // Sample from optimistic Beta distribution
        let optimistic_alpha = self.alpha + bonus;
        let u: f64 = rng.gen();
        match Beta::new(optimistic_alpha, self.beta) {
            Ok(dist) => dist.inverse_cdf(u),
            Err(_) => self.mean() + bonus, // Fallback with bonus
        }
    }

    /// Update based on outcome (also increments pull count)
    pub fn update(&mut self, success: bool) {
        self.pulls += 1;
        if success {
            self.alpha += 1.0;
        } else {
            self.beta += 1.0;
        }
    }

    /// Check if this arm is "cold" (needs exploration)
    pub fn is_cold(&self, threshold: u32) -> bool {
        self.pulls < threshold
    }

    /// Update with partial success (fractional)
    pub fn update_partial(&mut self, success_fraction: f64) {
        self.alpha += success_fraction;
        self.beta += 1.0 - success_fraction;
    }

    /// Mean of the distribution (point estimate)
    pub fn mean(&self) -> f64 {
        self.alpha / (self.alpha + self.beta)
    }

    /// Variance of the distribution (uncertainty)
    pub fn variance(&self) -> f64 {
        let n = self.alpha + self.beta;
        (self.alpha * self.beta) / (n.powi(2) * (n + 1.0))
    }

    /// 95% credible interval
    pub fn credible_interval_95(&self) -> (f64, f64) {
        match Beta::new(self.alpha, self.beta) {
            Ok(dist) => (dist.inverse_cdf(0.025), dist.inverse_cdf(0.975)),
            Err(_) => (0.0, 1.0),
        }
    }

    /// Total observations (equivalent sample size)
    pub fn total_observations(&self) -> f64 {
        self.alpha + self.beta - 2.0 // Subtract prior
    }
}

/// Context-aware arm that tracks per-domain statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextualArm {
    /// Principle ID
    pub principle_id: String,

    /// Principle name
    pub name: String,

    /// Global arm (all contexts combined)
    pub global: PrincipleArm,

    /// Per-domain arms
    pub domain_arms: HashMap<String, PrincipleArm>,
}

impl ContextualArm {
    pub fn new(id: String, name: String) -> Self {
        let global = PrincipleArm::new(id.clone(), name.clone());
        Self {
            principle_id: id,
            name,
            global,
            domain_arms: HashMap::new(),
        }
    }

    /// Standard Thompson Sampling considering context (domain)
    pub fn sample(&self, domain: Option<&str>, rng: &mut impl Rng) -> f64 {
        match domain {
            Some(d) if self.domain_arms.contains_key(d) => {
                // Combine domain-specific and global knowledge
                let domain_arm = &self.domain_arms[d];
                let domain_sample = domain_arm.sample(rng);
                let global_sample = self.global.sample(rng);

                // Weight by sample sizes
                let domain_weight = domain_arm.total_observations();
                let global_weight = self.global.total_observations();
                let total_weight = domain_weight + global_weight;

                if total_weight > 0.0 {
                    (domain_sample * domain_weight + global_sample * global_weight) / total_weight
                } else {
                    (domain_sample + global_sample) / 2.0
                }
            }
            _ => self.global.sample(rng),
        }
    }

    /// Feel-Good Thompson Sampling with context
    /// Adds optimism bonus for undersampled arms to solve cold-start
    pub fn fg_sample(&self, domain: Option<&str>, rng: &mut impl Rng, config: &FGTSConfig) -> f64 {
        match domain {
            Some(d) if self.domain_arms.contains_key(d) => {
                // Combine domain-specific and global knowledge with FG-TS
                let domain_arm = &self.domain_arms[d];
                let domain_sample = domain_arm.fg_sample(rng, config);
                let global_sample = self.global.fg_sample(rng, config);

                // Weight by sample sizes
                let domain_weight = domain_arm.total_observations();
                let global_weight = self.global.total_observations();
                let total_weight = domain_weight + global_weight;

                if total_weight > 0.0 {
                    (domain_sample * domain_weight + global_sample * global_weight) / total_weight
                } else {
                    (domain_sample + global_sample) / 2.0
                }
            }
            _ => self.global.fg_sample(rng, config),
        }
    }

    /// Check if this arm is cold (needs exploration)
    pub fn is_cold(&self, threshold: u32) -> bool {
        self.global.is_cold(threshold)
    }

    /// Total pulls across all domains
    pub fn total_pulls(&self) -> u32 {
        self.global.pulls
    }

    /// Update based on outcome
    pub fn update(&mut self, domain: Option<&str>, success: bool) {
        // Always update global
        self.global.update(success);

        // Update domain-specific if provided
        if let Some(d) = domain {
            let domain_arm = self.domain_arms
                .entry(d.to_string())
                .or_insert_with(|| PrincipleArm::new(
                    self.principle_id.clone(),
                    self.name.clone(),
                ));
            domain_arm.update(success);
        }
    }
}

/// Thompson Sampling selector for principles
/// Enhanced with Feel-Good TS (FG-TS) for cold-start and hybrid epsilon-greedy
pub struct ThompsonSelector {
    arms: HashMap<String, ContextualArm>,
    config: FGTSConfig,
}

impl ThompsonSelector {
    /// Create new selector from database with default FG-TS config
    pub fn from_db(conn: &Connection) -> Result<Self> {
        Self::from_db_with_config(conn, FGTSConfig::default())
    }

    /// Create new selector from database with custom FG-TS config
    pub fn from_db_with_config(conn: &Connection, config: FGTSConfig) -> Result<Self> {
        let mut arms = HashMap::new();

        // Load principles
        let mut stmt = conn.prepare(
            "SELECT id, name, learned_confidence FROM principles"
        )?;

        let principles = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, f64>(2)?,
            ))
        })?;

        for result in principles {
            let (id, name, confidence) = result?;

            // Convert existing confidence to Beta parameters
            // Assume 10 implicit observations for initial conversion
            let global = PrincipleArm::from_confidence(
                id.clone(),
                name.clone(),
                confidence,
                10.0,
            );

            let arm = ContextualArm {
                principle_id: id.clone(),
                name,
                global,
                domain_arms: HashMap::new(),
            };

            arms.insert(id, arm);
        }

        // Load historical adjustments to refine estimates
        let mut adj_stmt = conn.prepare(
            "SELECT principle_id, adjustment, context_pattern FROM framework_adjustments"
        )?;

        let adjustments = adj_stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, f64>(1)?,
                row.get::<_, Option<String>>(2)?,
            ))
        })?;

        for result in adjustments {
            let (principle_id, adjustment, context) = result?;
            if let Some(arm) = arms.get_mut(&principle_id) {
                let success = adjustment > 0.0;
                let domain = context.as_ref().and_then(|c| extract_domain(c));
                arm.update(domain.as_deref(), success);
            }
        }

        Ok(Self { arms, config })
    }

    /// Select top K principles using Feel-Good Thompson Sampling
    /// Combines FG-TS with hybrid epsilon-greedy for cold-start
    /// Per NeurIPS 2025: 25% improvement over vanilla TS, regret <0.15
    pub fn select_top_k(
        &self,
        candidates: &[String],
        k: usize,
        domain: Option<&str>,
    ) -> Vec<(String, f64)> {
        let mut rng = rand::thread_rng();

        // Separate cold and warm arms
        let (cold_candidates, _warm_candidates): (Vec<_>, Vec<_>) = candidates.iter()
            .partition(|id| {
                self.arms.get(*id)
                    .map(|arm| arm.is_cold(self.config.epsilon_threshold))
                    .unwrap_or(true)
            });

        let mut selected: Vec<(String, f64)> = Vec::with_capacity(k);

        // Hybrid epsilon-greedy: With probability epsilon, pick from cold arms
        let epsilon_slots = if !cold_candidates.is_empty() {
            let max_epsilon_slots = (k as f64 * self.config.epsilon).ceil() as usize;
            max_epsilon_slots.min(cold_candidates.len()).min(k)
        } else {
            0
        };

        // Fill epsilon slots with random cold arms (exploration)
        if epsilon_slots > 0 {
            let mut cold_shuffled = cold_candidates.clone();
            cold_shuffled.shuffle(&mut rng);
            for id in cold_shuffled.into_iter().take(epsilon_slots) {
                if let Some(arm) = self.arms.get(id) {
                    let sample = arm.fg_sample(domain, &mut rng, &self.config);
                    selected.push((id.clone(), sample));
                }
            }
        }

        // Fill remaining slots with FG-TS on all candidates
        let remaining_k = k.saturating_sub(selected.len());
        if remaining_k > 0 {
            let already_selected: std::collections::HashSet<_> = selected.iter()
                .map(|(id, _)| id.clone())
                .collect();

            let mut samples: Vec<(String, f64)> = candidates.iter()
                .filter(|id| !already_selected.contains(*id))
                .filter_map(|id| {
                    self.arms.get(id).map(|arm| {
                        (id.clone(), arm.fg_sample(domain, &mut rng, &self.config))
                    })
                })
                .collect();

            // Sort by FG-TS sample (descending)
            samples.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

            // Take remaining slots
            selected.extend(samples.into_iter().take(remaining_k));
        }

        // Final sort by sample value
        selected.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        selected
    }

    /// Select using standard Thompson Sampling (no FG-TS, for comparison)
    pub fn select_top_k_vanilla(
        &self,
        candidates: &[String],
        k: usize,
        domain: Option<&str>,
    ) -> Vec<(String, f64)> {
        let mut rng = rand::thread_rng();

        let mut samples: Vec<(String, f64)> = candidates.iter()
            .filter_map(|id| {
                self.arms.get(id).map(|arm| {
                    (id.clone(), arm.sample(domain, &mut rng))
                })
            })
            .collect();

        samples.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        samples.into_iter().take(k).collect()
    }

    /// Get count of cold (orphan) principles
    pub fn count_cold_arms(&self) -> usize {
        self.arms.values()
            .filter(|arm| arm.is_cold(self.config.epsilon_threshold))
            .count()
    }

    /// Get diversity metric (Gini coefficient) for principle selection
    /// Lower is better: 0 = perfect equality, 1 = one arm dominates
    pub fn gini_coefficient(&self) -> f64 {
        let pulls: Vec<f64> = self.arms.values()
            .map(|arm| arm.total_pulls() as f64)
            .collect();

        if pulls.is_empty() || pulls.iter().all(|&p| p == 0.0) {
            return 0.0;
        }

        let n = pulls.len() as f64;
        let mean = pulls.iter().sum::<f64>() / n;
        if mean == 0.0 {
            return 0.0;
        }

        let mut sorted_pulls = pulls.clone();
        sorted_pulls.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let sum_weighted: f64 = sorted_pulls.iter()
            .enumerate()
            .map(|(i, &p)| (i + 1) as f64 * p)
            .sum();

        (2.0 * sum_weighted) / (n * pulls.iter().sum::<f64>()) - (n + 1.0) / n
    }

    /// Update principle based on outcome
    pub fn record_outcome(
        &mut self,
        principle_id: &str,
        success: bool,
        domain: Option<&str>,
    ) {
        if let Some(arm) = self.arms.get_mut(principle_id) {
            arm.update(domain, success);
        }
    }

    /// Get statistics for a principle
    pub fn get_stats(&self, principle_id: &str) -> Option<PrincipleStats> {
        self.arms.get(principle_id).map(|arm| {
            let ci = arm.global.credible_interval_95();
            PrincipleStats {
                principle_id: principle_id.to_string(),
                name: arm.name.clone(),
                mean: arm.global.mean(),
                variance: arm.global.variance(),
                ci_lower: ci.0,
                ci_upper: ci.1,
                total_observations: arm.global.total_observations(),
                domain_stats: arm.domain_arms.iter()
                    .map(|(d, a)| (d.clone(), a.mean()))
                    .collect(),
            }
        })
    }

    /// Get all principles sorted by mean (for analysis)
    pub fn get_all_stats(&self) -> Vec<PrincipleStats> {
        let mut stats: Vec<_> = self.arms.values()
            .map(|arm| {
                let ci = arm.global.credible_interval_95();
                PrincipleStats {
                    principle_id: arm.principle_id.clone(),
                    name: arm.name.clone(),
                    mean: arm.global.mean(),
                    variance: arm.global.variance(),
                    ci_lower: ci.0,
                    ci_upper: ci.1,
                    total_observations: arm.global.total_observations(),
                    domain_stats: arm.domain_arms.iter()
                        .map(|(d, a)| (d.clone(), a.mean()))
                        .collect(),
                }
            })
            .collect();

        stats.sort_by(|a, b| b.mean.partial_cmp(&a.mean).unwrap());
        stats
    }

    /// Persist updated parameters back to database
    pub fn persist_to_db(&self, conn: &Connection) -> Result<()> {
        let mut stmt = conn.prepare(
            "UPDATE principles SET learned_confidence = ?2 WHERE id = ?1"
        )?;

        for (id, arm) in &self.arms {
            stmt.execute(params![id, arm.global.mean()])?;
        }

        Ok(())
    }
}

/// Statistics for a single principle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrincipleStats {
    pub principle_id: String,
    pub name: String,
    pub mean: f64,
    pub variance: f64,
    pub ci_lower: f64,
    pub ci_upper: f64,
    pub total_observations: f64,
    pub domain_stats: HashMap<String, f64>,
}

/// Extract domain from context pattern JSON
fn extract_domain(context_json: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(context_json)
        .ok()
        .and_then(|v| v.get("domain").and_then(|d| d.as_str().map(|s| s.to_string())))
}

/// Initialize Thompson Sampling schema in database
pub fn init_thompson_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(r#"
        -- Thompson Sampling parameters for principles
        CREATE TABLE IF NOT EXISTS thompson_arms (
            principle_id TEXT PRIMARY KEY REFERENCES principles(id),
            alpha REAL NOT NULL DEFAULT 1.0,
            beta REAL NOT NULL DEFAULT 1.0,
            updated_at TEXT DEFAULT CURRENT_TIMESTAMP
        );

        -- Per-domain Thompson parameters
        CREATE TABLE IF NOT EXISTS thompson_domain_arms (
            principle_id TEXT NOT NULL REFERENCES principles(id),
            domain TEXT NOT NULL,
            alpha REAL NOT NULL DEFAULT 1.0,
            beta REAL NOT NULL DEFAULT 1.0,
            updated_at TEXT DEFAULT CURRENT_TIMESTAMP,
            PRIMARY KEY (principle_id, domain)
        );
    "#)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_principle_arm_update() {
        let mut arm = PrincipleArm::new("test".to_string(), "Test".to_string());

        assert_eq!(arm.alpha, 1.0);
        assert_eq!(arm.beta, 1.0);
        assert_eq!(arm.mean(), 0.5); // Uninformative prior

        // Success should increase mean
        arm.update(true);
        assert!(arm.mean() > 0.5);

        // Failure should decrease mean
        arm.update(false);
        arm.update(false);
        assert!(arm.mean() < 0.5);
    }

    #[test]
    fn test_credible_interval() {
        let arm = PrincipleArm::new("test".to_string(), "Test".to_string());
        let (lower, upper) = arm.credible_interval_95();

        // With uninformative prior, CI should be wide
        assert!(lower < 0.1);
        assert!(upper > 0.9);

        // With more data, CI should narrow
        let informed = PrincipleArm::from_confidence(
            "test2".to_string(),
            "Test2".to_string(),
            0.7,
            100.0,
        );
        let (lower2, upper2) = informed.credible_interval_95();
        assert!(upper2 - lower2 < upper - lower);
    }

    #[test]
    fn test_thompson_sampling() {
        let arm = PrincipleArm::from_confidence(
            "test".to_string(),
            "Test".to_string(),
            0.8,
            20.0,
        );

        let mut rng = rand::thread_rng();
        let samples: Vec<f64> = (0..1000).map(|_| arm.sample(&mut rng)).collect();

        // Samples should be around the mean
        let avg: f64 = samples.iter().sum::<f64>() / samples.len() as f64;
        assert!((avg - arm.mean()).abs() < 0.1);
    }

    #[test]
    fn test_fg_sample_optimism_bonus() {
        // Cold arm (few pulls) should get higher samples due to optimism bonus
        let cold_arm = PrincipleArm::new("cold".to_string(), "Cold Arm".to_string());

        // Warm arm with same alpha/beta but many pulls
        let mut warm_arm = PrincipleArm::new("warm".to_string(), "Warm Arm".to_string());
        warm_arm.pulls = 50;

        let config = FGTSConfig::default();
        let mut rng = rand::thread_rng();

        // Sample both many times
        let cold_samples: Vec<f64> = (0..1000).map(|_| cold_arm.fg_sample(&mut rng, &config)).collect();
        let warm_samples: Vec<f64> = (0..1000).map(|_| warm_arm.fg_sample(&mut rng, &config)).collect();

        let cold_avg: f64 = cold_samples.iter().sum::<f64>() / cold_samples.len() as f64;
        let warm_avg: f64 = warm_samples.iter().sum::<f64>() / warm_samples.len() as f64;

        // Cold arm should have higher average due to optimism bonus
        assert!(cold_avg > warm_avg, "Cold arm ({:.3}) should have higher avg than warm ({:.3})", cold_avg, warm_avg);
    }

    #[test]
    fn test_fg_sample_bonus_decays() {
        let config = FGTSConfig::default();
        let mut rng = rand::thread_rng();

        // Create arms with increasing pulls
        let arms: Vec<PrincipleArm> = (0..5).map(|i| {
            let mut arm = PrincipleArm::new(format!("arm-{}", i), format!("Arm {}", i));
            arm.pulls = i * 10;
            arm
        }).collect();

        let avgs: Vec<f64> = arms.iter().map(|arm| {
            let samples: Vec<f64> = (0..1000).map(|_| arm.fg_sample(&mut rng, &config)).collect();
            samples.iter().sum::<f64>() / samples.len() as f64
        }).collect();

        // Each subsequent arm should have lower average (decaying bonus)
        for i in 1..avgs.len() {
            assert!(avgs[i] <= avgs[i-1] + 0.05, // Allow small tolerance
                "Arm {} (avg {:.3}) should be <= arm {} (avg {:.3})",
                i, avgs[i], i-1, avgs[i-1]);
        }
    }

    #[test]
    fn test_cold_arm_detection() {
        let mut arm = PrincipleArm::new("test".to_string(), "Test".to_string());

        assert!(arm.is_cold(10), "New arm should be cold");

        // Simulate 10 pulls
        for _ in 0..10 {
            arm.update(true);
        }

        assert!(!arm.is_cold(10), "Arm with 10 pulls should not be cold");
    }

    #[test]
    fn test_gini_coefficient() {
        // Create a selector with artificial arms
        let mut arms = HashMap::new();

        // Equal distribution: 10 arms, 10 pulls each
        for i in 0..10 {
            let mut arm = ContextualArm::new(format!("arm-{}", i), format!("Arm {}", i));
            arm.global.pulls = 10;
            arms.insert(format!("arm-{}", i), arm);
        }

        let selector = ThompsonSelector { arms, config: FGTSConfig::default() };
        let gini = selector.gini_coefficient();

        // Perfect equality should have Gini close to 0
        assert!(gini < 0.2, "Equal distribution should have low Gini: {:.3}", gini);

        // Now create unequal distribution
        let mut arms2 = HashMap::new();
        for i in 0..10 {
            let mut arm = ContextualArm::new(format!("arm-{}", i), format!("Arm {}", i));
            // One arm dominates
            arm.global.pulls = if i == 0 { 100 } else { 1 };
            arms2.insert(format!("arm-{}", i), arm);
        }

        let selector2 = ThompsonSelector { arms: arms2, config: FGTSConfig::default() };
        let gini2 = selector2.gini_coefficient();

        // Unequal distribution should have higher Gini
        assert!(gini2 > gini, "Unequal distribution ({:.3}) should have higher Gini than equal ({:.3})", gini2, gini);
    }
}
