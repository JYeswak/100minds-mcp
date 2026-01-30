//! Neural Posterior for Principle Selection
//!
//! ONNX-based neural network that replaces Beta distributions for principle ranking.
//! Trained on 40k synthetic decision/outcome pairs.
//!
//! Architecture:
//! - MLP encoder with self-attention on context features
//! - Principle and thinker embeddings (64-dim)
//! - Outputs: success probability + epistemic uncertainty
//!
//! Integration with Thompson Sampling:
//! - Use success_prob as mean estimate
//! - Use uncertainty to boost exploration (UCB-style)

use anyhow::{anyhow, Context, Result};
use ndarray::Array2;
use ort::{
    inputs,
    session::{builder::GraphOptimizationLevel, Session},
    value::Tensor,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Context dimension from training (must match ONNX model)
const CONTEXT_DIM: usize = 33;

/// Vocabulary mappings loaded from JSON
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeuralVocab {
    pub domain: HashMap<String, usize>,
    pub stakeholder: HashMap<String, usize>,
    pub stage: HashMap<String, usize>,
    pub urgency: HashMap<String, usize>,
    pub principle: HashMap<String, usize>,
    pub thinker: HashMap<String, usize>,
}

impl NeuralVocab {
    /// Load vocabulary from JSON file
    pub fn from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path).context("Failed to read vocabulary file")?;
        let vocab: NeuralVocab =
            serde_json::from_str(&content).context("Failed to parse vocabulary JSON")?;
        Ok(vocab)
    }

    /// Get principle index (0 if unknown)
    pub fn principle_idx(&self, principle_id: &str) -> usize {
        *self.principle.get(principle_id).unwrap_or(&0)
    }

    /// Get thinker index (0 if unknown)
    pub fn thinker_idx(&self, thinker_id: &str) -> usize {
        *self.thinker.get(thinker_id).unwrap_or(&0)
    }
}

/// Context features for a single scoring request
#[derive(Debug, Clone)]
pub struct ScoringContext {
    pub domain: String,
    pub stakeholder: String,
    pub company_stage: String,
    pub urgency: String,
    pub difficulty: u8,
    pub position_rank: usize,
    pub confidence: f64,
    pub domain_match: bool,
    pub total_principles_selected: usize,
    pub is_for_position: bool,
}

impl Default for ScoringContext {
    fn default() -> Self {
        Self {
            domain: "architecture".to_string(),
            stakeholder: "Tech Lead".to_string(),
            company_stage: "growth".to_string(),
            urgency: "normal".to_string(),
            difficulty: 3,
            position_rank: 0,
            confidence: 0.5,
            domain_match: false,
            total_principles_selected: 3,
            is_for_position: true,
        }
    }
}

/// Neural posterior result for a single principle
#[derive(Debug, Clone)]
pub struct PosteriorResult {
    pub principle_id: String,
    pub success_prob: f32,
    pub uncertainty: f32,
    /// UCB-style score: success_prob + exploration_weight * uncertainty
    pub ucb_score: f32,
}

/// Neural posterior model for principle selection
pub struct NeuralPosterior {
    session: Session,
    vocab: NeuralVocab,
    /// Exploration weight for UCB scoring (higher = more exploration)
    exploration_weight: f32,
}

impl NeuralPosterior {
    /// Load neural posterior from model directory
    pub fn new(model_dir: &Path) -> Result<Self> {
        let model_path = model_dir.join("neural_bandit.onnx");
        let vocab_path = model_dir.join("neural_bandit_vocab.json");

        if !model_path.exists() {
            return Err(anyhow!(
                "Neural bandit model not found at {}. Run training first.",
                model_path.display()
            ));
        }

        // Load ONNX model
        let session = Session::builder()?
            .with_optimization_level(GraphOptimizationLevel::Level3)?
            .with_intra_threads(2)?
            .commit_from_file(&model_path)
            .context("Failed to load neural bandit ONNX model")?;

        // Load vocabulary
        let vocab = NeuralVocab::from_file(&vocab_path)?;

        Ok(Self {
            session,
            vocab,
            exploration_weight: 1.0, // UCB exploration factor
        })
    }

    /// Set exploration weight (0 = pure exploitation, higher = more exploration)
    pub fn set_exploration_weight(&mut self, weight: f32) {
        self.exploration_weight = weight;
    }

    /// Build context feature vector from ScoringContext
    fn build_context_vector(&self, ctx: &ScoringContext) -> Vec<f32> {
        let mut features = Vec::with_capacity(CONTEXT_DIM);

        // Domain one-hot (10 dims)
        let domain_size = self.vocab.domain.len();
        let domain_idx = *self.vocab.domain.get(&ctx.domain).unwrap_or(&0);
        for i in 0..domain_size {
            features.push(if i == domain_idx { 1.0 } else { 0.0 });
        }

        // Stakeholder one-hot
        let stakeholder_size = self.vocab.stakeholder.len();
        let stakeholder_idx = *self.vocab.stakeholder.get(&ctx.stakeholder).unwrap_or(&0);
        for i in 0..stakeholder_size {
            features.push(if i == stakeholder_idx { 1.0 } else { 0.0 });
        }

        // Company stage one-hot
        let stage_size = self.vocab.stage.len();
        let stage_idx = *self.vocab.stage.get(&ctx.company_stage).unwrap_or(&0);
        for i in 0..stage_size {
            features.push(if i == stage_idx { 1.0 } else { 0.0 });
        }

        // Urgency one-hot
        let urgency_size = self.vocab.urgency.len();
        let urgency_idx = *self.vocab.urgency.get(&ctx.urgency).unwrap_or(&0);
        for i in 0..urgency_size {
            features.push(if i == urgency_idx { 1.0 } else { 0.0 });
        }

        // Scalar features (normalized)
        features.push(ctx.difficulty as f32 / 5.0);
        features.push(ctx.position_rank as f32 / 10.0);
        features.push(ctx.confidence as f32);
        features.push(if ctx.domain_match { 1.0 } else { 0.0 });
        features.push(ctx.total_principles_selected as f32 / 10.0);
        features.push(if ctx.is_for_position { 1.0 } else { 0.0 });

        // Pad to CONTEXT_DIM if needed
        while features.len() < CONTEXT_DIM {
            features.push(0.0);
        }

        features.truncate(CONTEXT_DIM);
        features
    }

    /// Score a single principle
    pub fn score(
        &mut self,
        ctx: &ScoringContext,
        principle_id: &str,
        thinker_id: &str,
    ) -> Result<PosteriorResult> {
        let context_vec = self.build_context_vector(ctx);

        // Prepare inputs
        let context_array = Array2::from_shape_vec((1, CONTEXT_DIM), context_vec)?;
        let context_tensor = Tensor::from_array(context_array)?;

        let principle_idx = self.vocab.principle_idx(principle_id) as i64;
        let thinker_idx = self.vocab.thinker_idx(thinker_id) as i64;
        let arm_array = Array2::from_shape_vec((1, 2), vec![principle_idx, thinker_idx])?;
        let arm_tensor = Tensor::from_array(arm_array)?;

        // Run inference
        let outputs = self.session.run(inputs![
            "context" => context_tensor,
            "arm_indices" => arm_tensor,
        ])?;

        // Extract outputs
        let success_output = outputs
            .get("success_prob")
            .ok_or_else(|| anyhow!("No success_prob output"))?;
        let (_shape, data) = success_output.try_extract_tensor::<f32>()?;
        let success_prob: f32 = *data.first().unwrap_or(&0.5);

        let uncertainty_output = outputs
            .get("uncertainty")
            .ok_or_else(|| anyhow!("No uncertainty output"))?;
        let (_shape, data) = uncertainty_output.try_extract_tensor::<f32>()?;
        let uncertainty: f32 = *data.first().unwrap_or(&0.5);

        let ucb_score = success_prob + self.exploration_weight * uncertainty;

        Ok(PosteriorResult {
            principle_id: principle_id.to_string(),
            success_prob,
            uncertainty,
            ucb_score,
        })
    }

    /// Score multiple principles and return sorted by UCB score (highest first)
    pub fn score_batch(
        &mut self,
        ctx: &ScoringContext,
        principles: &[(String, String)], // (principle_id, thinker_id)
    ) -> Result<Vec<PosteriorResult>> {
        let mut results = Vec::with_capacity(principles.len());

        for (principle_id, thinker_id) in principles {
            let result = self.score(ctx, principle_id, thinker_id)?;
            results.push(result);
        }

        // Sort by UCB score (highest first)
        results.sort_by(|a, b| b.ucb_score.partial_cmp(&a.ucb_score).unwrap());

        Ok(results)
    }

    /// Get vocabulary reference
    pub fn vocab(&self) -> &NeuralVocab {
        &self.vocab
    }

    /// Check if a principle is known to the model
    pub fn knows_principle(&self, principle_id: &str) -> bool {
        self.vocab.principle.contains_key(principle_id)
    }

    /// Check if a thinker is known to the model
    pub fn knows_thinker(&self, thinker_id: &str) -> bool {
        self.vocab.thinker.contains_key(thinker_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_context_vector_length() {
        let vocab = NeuralVocab {
            domain: [("architecture".to_string(), 0)].into_iter().collect(),
            stakeholder: [("Tech Lead".to_string(), 0)].into_iter().collect(),
            stage: [("growth".to_string(), 0)].into_iter().collect(),
            urgency: [("normal".to_string(), 0)].into_iter().collect(),
            principle: HashMap::new(),
            thinker: HashMap::new(),
        };

        // Create minimal neural posterior for testing
        // (would need actual model for full test)
    }

    #[test]
    fn test_scoring_context_default() {
        let ctx = ScoringContext::default();
        assert_eq!(ctx.difficulty, 3);
        assert_eq!(ctx.domain, "architecture");
    }
}
