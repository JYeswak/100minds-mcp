//! Core types for the 100minds Adversarial Wisdom Council
//!
//! These types embody the philosophical foundations:
//! - Adversarial by default (Taleb's via negativa)
//! - Falsifiable positions (Popper)
//! - Clear and simple (Dijkstra/Feynman)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A thinker in our wisdom council
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Thinker {
    pub id: String,
    pub name: String,
    pub domain: String,
    pub background: Option<String>,
    pub principles: Vec<Principle>,
}

/// A principle or framework from a thinker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Principle {
    pub id: String,
    pub thinker_id: String,
    pub name: String,
    pub description: String,
    pub domain_tags: Vec<String>,
    pub application_rule: Option<String>,
    pub anti_pattern: Option<String>,
    pub falsification: Option<String>,
    pub confidence: f64,
}

/// The stance a counsel position takes
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Stance {
    /// Argues in favor of the proposed decision
    For,
    /// Argues against the proposed decision
    Against,
    /// Synthesizes multiple viewpoints
    Synthesize,
    /// Devil's advocate - challenges assumptions
    Challenge,
}

/// A position in the adversarial counsel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CounselPosition {
    pub thinker: String,
    pub thinker_id: String,
    pub stance: Stance,
    pub argument: String,
    pub principles_cited: Vec<String>,
    pub confidence: f64,
    /// What would prove this position wrong (Popper)
    pub falsifiable_if: Option<String>,
}

/// Full counsel response with adversarial debate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CounselResponse {
    pub decision_id: String,
    pub question: String,
    pub positions: Vec<CounselPosition>,
    pub challenge: CounselPosition,
    pub summary: String,
    pub provenance: ProvenanceInfo,
    pub created_at: DateTime<Utc>,
    // === SWARM INTEGRATION FIELDS (v2) ===
    /// Principle IDs for outcome tracking
    #[serde(default)]
    pub principle_ids: Vec<String>,
    /// Urgency adjustment suggestion: "escalate" | "defer" | null
    #[serde(skip_serializing_if = "Option::is_none")]
    pub urgency_adjustment: Option<String>,
    /// Causal reasoning for why these principles were selected
    #[serde(default)]
    pub causal_hints: Vec<String>,
}

/// Provenance information for audit trail
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvenanceInfo {
    pub content_hash: String,
    pub previous_hash: Option<String>,
    pub signature: String,
    pub agent_pubkey: String,
}

/// Request for counsel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CounselRequest {
    pub question: String,
    #[serde(default)]
    pub context: CounselContext,
}

/// Context for counsel request
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CounselContext {
    /// Domain of the decision (e.g., "architecture", "hiring")
    pub domain: Option<String>,
    /// Constraints to consider
    pub constraints: Vec<String>,
    /// Preferred thinkers to consult
    pub prefer_thinkers: Vec<String>,
    /// Depth of analysis
    #[serde(default)]
    pub depth: CounselDepth,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CounselDepth {
    /// Quick counsel - 3 positions
    Quick,
    /// Standard counsel - 4-5 positions
    #[default]
    Standard,
    /// Deep counsel - 6+ positions with extensive analysis
    Deep,
}

/// A recorded decision with full provenance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Decision {
    pub id: String,
    pub question: String,
    pub context: Option<CounselContext>,
    pub counsel: CounselResponse,
    pub provenance: ProvenanceInfo,
    pub outcome: Option<Outcome>,
    pub created_at: DateTime<Utc>,
}

/// Outcome of a decision
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Outcome {
    pub success: bool,
    pub notes: Option<String>,
    pub recorded_at: DateTime<Utc>,
}

/// Request to record an outcome
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordOutcomeRequest {
    pub decision_id: String,
    pub success: bool,
    pub notes: Option<String>,
    // === SWARM INTEGRATION FIELDS (v2) ===
    /// Principle IDs that were used in this decision
    #[serde(default)]
    pub principle_ids: Vec<String>,
    /// Domain for contextual learning
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    /// Worker's self-reported confidence (0.0-1.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence_score: Option<f64>,
    /// Failure stage if not success: "lint" | "types" | "build" | "test"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure_stage: Option<String>,
}

/// Batch outcome recording for catch-up sync
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordOutcomesBatchRequest {
    pub outcomes: Vec<RecordOutcomeRequest>,
}

/// Response from counterfactual simulation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CounterfactualResponse {
    pub question: String,
    pub excluded_principles: Vec<String>,
    pub excluded_count: usize,
    pub alternative_positions: Vec<CounselPosition>,
    pub original_principle_ids: Vec<String>,
    pub new_principle_ids: Vec<String>,
    /// Jaccard distance: 1 - (intersection / union) of principle sets
    pub diversity_delta: f64,
}

/// Response from sync_posteriors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncPosteriorsResponse {
    /// Per-principle Thompson posteriors
    pub posteriors: std::collections::HashMap<String, PrinciplePosterior>,
    /// Per-domain per-principle posteriors
    pub domains: std::collections::HashMap<String, std::collections::HashMap<String, PrinciplePosterior>>,
    /// Unix timestamp of last update
    pub last_updated: i64,
}

/// Thompson Sampling posterior for a principle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrinciplePosterior {
    pub alpha: f64,
    pub beta: f64,
    pub pulls: u32,
}

/// Challenge request for additional devil's advocate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChallengeRequest {
    pub decision_id: String,
    pub focus: Option<String>,
}

/// Challenge response with additional adversarial positions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChallengeResponse {
    pub decision_id: String,
    pub challenges: Vec<CounselPosition>,
}

/// Audit request for provenance chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditRequest {
    pub decision_id: String,
    #[serde(default)]
    pub include_chain: bool,
    #[serde(default)]
    pub verify_signatures: bool,
}

/// Audit response with provenance chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditResponse {
    pub decision: Decision,
    pub chain: Vec<ProvenanceInfo>,
    pub chain_valid: bool,
    pub verification_errors: Vec<String>,
}

/// Synthesis request for multi-thinker combination
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynthesizeRequest {
    pub thinkers: Vec<String>,
    pub question: String,
}

/// Synthesis response combining multiple thinkers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynthesizeResponse {
    pub thinkers: Vec<String>,
    pub question: String,
    pub synthesis: String,
    pub contributing_principles: Vec<Principle>,
    pub combined_confidence: f64,
}

impl CounselResponse {
    /// Create a new counsel response
    pub fn new(
        question: String,
        positions: Vec<CounselPosition>,
        challenge: CounselPosition,
        provenance: ProvenanceInfo,
    ) -> Self {
        let summary = Self::generate_summary(&positions, &challenge);

        // Extract principle IDs from positions
        let principle_ids: Vec<String> = positions
            .iter()
            .flat_map(|p| p.principles_cited.clone())
            .collect();

        // Generate causal hints explaining why principles were selected
        let causal_hints: Vec<String> = positions
            .iter()
            .filter_map(|p| {
                if !p.principles_cited.is_empty() {
                    Some(format!(
                        "{} cites {} for {} stance",
                        p.thinker,
                        p.principles_cited.join(", "),
                        p.stance.name()
                    ))
                } else {
                    None
                }
            })
            .collect();

        Self {
            decision_id: Uuid::new_v4().to_string(),
            question,
            positions,
            challenge,
            summary,
            provenance,
            created_at: Utc::now(),
            principle_ids,
            urgency_adjustment: None,
            causal_hints,
        }
    }

    fn generate_summary(positions: &[CounselPosition], challenge: &CounselPosition) -> String {
        let for_count = positions.iter().filter(|p| p.stance == Stance::For).count();
        let against_count = positions.iter().filter(|p| p.stance == Stance::Against).count();

        let highest_confidence = positions
            .iter()
            .max_by(|a, b| a.confidence.partial_cmp(&b.confidence).unwrap())
            .map(|p| format!("{} ({:.0}%)", p.thinker, p.confidence * 100.0))
            .unwrap_or_default();

        format!(
            "{} position(s) FOR, {} AGAINST. Highest confidence: {}. Challenge: {}",
            for_count, against_count, highest_confidence, challenge.argument
        )
    }
}

impl Stance {
    pub fn emoji(&self) -> &'static str {
        match self {
            Stance::For => "🟢",
            Stance::Against => "🔴",
            Stance::Synthesize => "🟡",
            Stance::Challenge => "⚡",
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Stance::For => "FOR",
            Stance::Against => "AGAINST",
            Stance::Synthesize => "SYNTHESIS",
            Stance::Challenge => "CHALLENGE",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_provenance() -> ProvenanceInfo {
        ProvenanceInfo {
            content_hash: "abc123".to_string(),
            previous_hash: None,
            signature: "sig".to_string(),
            agent_pubkey: "pubkey".to_string(),
        }
    }

    fn mock_position(stance: Stance, thinker: &str, principles: Vec<&str>) -> CounselPosition {
        CounselPosition {
            thinker: thinker.to_string(),
            thinker_id: format!("{}-id", thinker.to_lowercase()),
            stance,
            argument: format!("{} argues this position", thinker),
            principles_cited: principles.iter().map(|s| s.to_string()).collect(),
            confidence: 0.8,
            falsifiable_if: Some("If conditions change".to_string()),
        }
    }

    #[test]
    fn test_stance_emoji() {
        assert_eq!(Stance::For.emoji(), "🟢");
        assert_eq!(Stance::Against.emoji(), "🔴");
        assert_eq!(Stance::Synthesize.emoji(), "🟡");
        assert_eq!(Stance::Challenge.emoji(), "⚡");
    }

    #[test]
    fn test_stance_name() {
        assert_eq!(Stance::For.name(), "FOR");
        assert_eq!(Stance::Against.name(), "AGAINST");
        assert_eq!(Stance::Synthesize.name(), "SYNTHESIS");
        assert_eq!(Stance::Challenge.name(), "CHALLENGE");
    }

    #[test]
    fn test_stance_serialization() {
        assert_eq!(serde_json::to_string(&Stance::For).unwrap(), "\"for\"");
        assert_eq!(serde_json::to_string(&Stance::Against).unwrap(), "\"against\"");
        assert_eq!(serde_json::to_string(&Stance::Synthesize).unwrap(), "\"synthesize\"");
        assert_eq!(serde_json::to_string(&Stance::Challenge).unwrap(), "\"challenge\"");
    }

    #[test]
    fn test_stance_deserialization() {
        let for_stance: Stance = serde_json::from_str("\"for\"").unwrap();
        assert_eq!(for_stance, Stance::For);

        let against: Stance = serde_json::from_str("\"against\"").unwrap();
        assert_eq!(against, Stance::Against);
    }

    #[test]
    fn test_counsel_response_new() {
        let positions = vec![
            mock_position(Stance::For, "Kent Beck", vec!["tdd", "yagni"]),
            mock_position(Stance::Against, "Fred Brooks", vec!["brooks-law"]),
        ];
        let challenge = mock_position(Stance::Challenge, "Nassim Taleb", vec!["antifragility"]);

        let response = CounselResponse::new(
            "Should we add more tests?".to_string(),
            positions,
            challenge,
            mock_provenance(),
        );

        assert!(!response.decision_id.is_empty());
        assert_eq!(response.question, "Should we add more tests?");
        assert_eq!(response.positions.len(), 2);
        assert_eq!(response.challenge.thinker, "Nassim Taleb");
        assert!(!response.summary.is_empty());
        assert!(response.summary.contains("FOR"));
        assert!(response.summary.contains("AGAINST"));
    }

    #[test]
    fn test_counsel_response_extracts_principle_ids() {
        let positions = vec![
            mock_position(Stance::For, "Kent Beck", vec!["tdd", "yagni"]),
            mock_position(Stance::Against, "Martin Fowler", vec!["refactoring"]),
        ];
        let challenge = mock_position(Stance::Challenge, "Taleb", vec![]);

        let response = CounselResponse::new(
            "Test question".to_string(),
            positions,
            challenge,
            mock_provenance(),
        );

        assert!(response.principle_ids.contains(&"tdd".to_string()));
        assert!(response.principle_ids.contains(&"yagni".to_string()));
        assert!(response.principle_ids.contains(&"refactoring".to_string()));
    }

    #[test]
    fn test_counsel_response_generates_causal_hints() {
        let positions = vec![
            mock_position(Stance::For, "Kent Beck", vec!["tdd"]),
        ];
        let challenge = mock_position(Stance::Challenge, "Taleb", vec![]);

        let response = CounselResponse::new(
            "Test".to_string(),
            positions,
            challenge,
            mock_provenance(),
        );

        assert!(!response.causal_hints.is_empty());
        assert!(response.causal_hints[0].contains("Kent Beck"));
        assert!(response.causal_hints[0].contains("tdd"));
        assert!(response.causal_hints[0].contains("FOR"));
    }

    #[test]
    fn test_counsel_response_summary_counts() {
        let positions = vec![
            mock_position(Stance::For, "A", vec![]),
            mock_position(Stance::For, "B", vec![]),
            mock_position(Stance::Against, "C", vec![]),
        ];
        let challenge = mock_position(Stance::Challenge, "D", vec![]);

        let response = CounselResponse::new(
            "Test".to_string(),
            positions,
            challenge,
            mock_provenance(),
        );

        assert!(response.summary.contains("2 position(s) FOR"));
        assert!(response.summary.contains("1 AGAINST"));
    }

    #[test]
    fn test_counsel_depth_default() {
        let depth: CounselDepth = Default::default();
        assert_eq!(depth, CounselDepth::Standard);
    }

    #[test]
    fn test_counsel_depth_serialization() {
        assert_eq!(serde_json::to_string(&CounselDepth::Quick).unwrap(), "\"quick\"");
        assert_eq!(serde_json::to_string(&CounselDepth::Standard).unwrap(), "\"standard\"");
        assert_eq!(serde_json::to_string(&CounselDepth::Deep).unwrap(), "\"deep\"");
    }

    #[test]
    fn test_counsel_request_default_context() {
        let request: CounselRequest = serde_json::from_str(r#"{"question": "Test?"}"#).unwrap();
        assert_eq!(request.question, "Test?");
        assert!(request.context.domain.is_none());
        assert!(request.context.constraints.is_empty());
        assert_eq!(request.context.depth, CounselDepth::Standard);
    }

    #[test]
    fn test_principle_posterior_serialization() {
        let posterior = PrinciplePosterior {
            alpha: 5.0,
            beta: 2.0,
            pulls: 7,
        };
        let json = serde_json::to_string(&posterior).unwrap();
        assert!(json.contains("\"alpha\":5.0"));
        assert!(json.contains("\"beta\":2.0"));
        assert!(json.contains("\"pulls\":7"));
    }

    #[test]
    fn test_record_outcome_request_optional_fields() {
        // Test that optional fields can be omitted
        let json = r#"{
            "decision_id": "test-123",
            "success": true
        }"#;
        let request: RecordOutcomeRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.decision_id, "test-123");
        assert!(request.success);
        assert!(request.notes.is_none());
        assert!(request.principle_ids.is_empty());
        assert!(request.domain.is_none());
        assert!(request.confidence_score.is_none());
        assert!(request.failure_stage.is_none());
    }

    #[test]
    fn test_counterfactual_response_diversity_delta() {
        let response = CounterfactualResponse {
            question: "Test".to_string(),
            excluded_principles: vec!["p1".to_string()],
            excluded_count: 1,
            alternative_positions: vec![],
            original_principle_ids: vec!["p1".to_string(), "p2".to_string()],
            new_principle_ids: vec!["p3".to_string(), "p4".to_string()],
            diversity_delta: 1.0, // Complete divergence
        };

        // Jaccard distance of 1.0 means no overlap between original and new
        assert_eq!(response.diversity_delta, 1.0);
    }
}
