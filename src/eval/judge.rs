//! LLM-as-Judge Framework for Automated Evaluation
//!
//! Uses Claude Haiku for fast, cheap evaluation at scale.
//! Implements multi-criteria rubric scoring and pairwise comparison.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Evaluation rubric with weighted criteria
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JudgeRubric {
    pub criteria: Vec<JudgeCriterion>,
}

/// A single evaluation criterion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JudgeCriterion {
    pub name: String,
    pub weight: f64,
    pub prompt: String,
    pub scale: RatingScale,
}

/// Rating scale for criteria
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RatingScale {
    Scale1to5,
    Binary,
    Scale1to10,
}

impl Default for JudgeRubric {
    fn default() -> Self {
        Self {
            criteria: vec![
                JudgeCriterion {
                    name: "Relevance".into(),
                    weight: 0.30,
                    prompt: "Do the cited principles directly address the decision question? Rate 1-5 where 5 = perfectly relevant.".into(),
                    scale: RatingScale::Scale1to5,
                },
                JudgeCriterion {
                    name: "Completeness".into(),
                    weight: 0.20,
                    prompt: "Are important considerations missing? Rate 5 if comprehensive, 1 if major gaps.".into(),
                    scale: RatingScale::Scale1to5,
                },
                JudgeCriterion {
                    name: "Actionability".into(),
                    weight: 0.25,
                    prompt: "Can the user immediately act on this advice? Rate 5 if clear next steps, 1 if vague.".into(),
                    scale: RatingScale::Scale1to5,
                },
                JudgeCriterion {
                    name: "Balance".into(),
                    weight: 0.15,
                    prompt: "Are both FOR and AGAINST positions genuinely opposed? Rate 5 if balanced, 1 if one-sided.".into(),
                    scale: RatingScale::Scale1to5,
                },
                JudgeCriterion {
                    name: "Authority".into(),
                    weight: 0.10,
                    prompt: "Are the cited thinkers credible for this domain? Rate 5 if highly authoritative, 1 if questionable.".into(),
                    scale: RatingScale::Scale1to5,
                },
            ],
        }
    }
}

/// Result from LLM judge evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JudgeResult {
    pub question: String,
    pub scores: HashMap<String, f64>,
    pub overall_score: f64,
    pub reasoning: String,
    pub suggested_improvements: Vec<String>,
    pub confidence: f64,
}

/// Pairwise comparison result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairwiseResult {
    pub question: String,
    pub winner: PairwiseWinner,
    pub confidence: f64,
    pub reasoning: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PairwiseWinner {
    A,
    B,
    Tie,
}

/// Configuration for the judge
#[derive(Debug, Clone)]
pub struct JudgeConfig {
    pub model: String,
    pub rubric: JudgeRubric,
    pub temperature: f32,
    pub max_concurrent: usize,
}

impl Default for JudgeConfig {
    fn default() -> Self {
        Self {
            model: "claude-3-haiku-20240307".into(),
            rubric: JudgeRubric::default(),
            temperature: 0.0,  // Deterministic
            max_concurrent: 10,
        }
    }
}

/// Generate prompt for single-response evaluation
pub fn build_eval_prompt(
    question: &str,
    response_summary: &str,
    principles_cited: &[String],
    thinkers_cited: &[String],
    rubric: &JudgeRubric,
) -> String {
    let criteria_text: String = rubric.criteria.iter()
        .map(|c| format!("- **{}** ({}%): {}", c.name, (c.weight * 100.0) as u32, c.prompt))
        .collect::<Vec<_>>()
        .join("\n");

    format!(r#"You are evaluating the quality of a decision-support response.

## Question
{question}

## Response Summary
{response_summary}

## Principles Cited
{principles}

## Thinkers Cited
{thinkers}

## Evaluation Criteria
{criteria_text}

## Your Task
Evaluate this response on each criterion. Output JSON only:

```json
{{
  "scores": {{
    "Relevance": <1-5>,
    "Completeness": <1-5>,
    "Actionability": <1-5>,
    "Balance": <1-5>,
    "Authority": <1-5>
  }},
  "reasoning": "<brief explanation of scores>",
  "suggested_improvements": ["<improvement 1>", "<improvement 2>"],
  "confidence": <0.0-1.0>
}}
```
"#,
        question = question,
        response_summary = response_summary,
        principles = principles_cited.join(", "),
        thinkers = thinkers_cited.join(", "),
        criteria_text = criteria_text,
    )
}

/// Generate prompt for pairwise comparison
pub fn build_pairwise_prompt(
    question: &str,
    response_a: &str,
    response_b: &str,
) -> String {
    format!(r#"You are comparing two decision-support responses.

## Question
{question}

## Response A
{response_a}

## Response B
{response_b}

## Your Task
Which response better helps the user make their decision?
Consider: relevance to the question, actionable advice, balance of perspectives, credibility of sources.

Output JSON only:

```json
{{
  "winner": "A" | "B" | "tie",
  "confidence": <0.0-1.0>,
  "reasoning": "<why this response is better>"
}}
```
"#,
        question = question,
        response_a = response_a,
        response_b = response_b,
    )
}

/// Parse JSON response from judge
pub fn parse_eval_response(json_str: &str) -> Result<JudgeResult> {
    // Extract JSON from potential markdown code block
    let clean = extract_json(json_str);

    #[derive(Deserialize)]
    struct RawResponse {
        scores: HashMap<String, f64>,
        reasoning: String,
        suggested_improvements: Vec<String>,
        confidence: f64,
    }

    let raw: RawResponse = serde_json::from_str(&clean)?;

    // Calculate weighted overall score
    let rubric = JudgeRubric::default();
    let overall_score = rubric.criteria.iter()
        .filter_map(|c| raw.scores.get(&c.name).map(|s| s * c.weight))
        .sum();

    Ok(JudgeResult {
        question: String::new(),  // Filled in by caller
        scores: raw.scores,
        overall_score,
        reasoning: raw.reasoning,
        suggested_improvements: raw.suggested_improvements,
        confidence: raw.confidence,
    })
}

/// Parse pairwise comparison response
pub fn parse_pairwise_response(json_str: &str) -> Result<PairwiseResult> {
    let clean = extract_json(json_str);

    #[derive(Deserialize)]
    struct RawPairwise {
        winner: String,
        confidence: f64,
        reasoning: String,
    }

    let raw: RawPairwise = serde_json::from_str(&clean)?;

    let winner = match raw.winner.to_lowercase().as_str() {
        "a" => PairwiseWinner::A,
        "b" => PairwiseWinner::B,
        _ => PairwiseWinner::Tie,
    };

    Ok(PairwiseResult {
        question: String::new(),
        winner,
        confidence: raw.confidence,
        reasoning: raw.reasoning,
    })
}

/// Extract JSON from markdown code block if present
fn extract_json(text: &str) -> String {
    // Try to find JSON in code block
    if let Some(start) = text.find("```json") {
        if let Some(end) = text[start..].find("```\n").or_else(|| text[start..].rfind("```")) {
            let json_start = start + 7;  // Skip "```json"
            return text[json_start..start + end].trim().to_string();
        }
    }

    // Try to find raw JSON
    if let Some(start) = text.find('{') {
        if let Some(end) = text.rfind('}') {
            return text[start..=end].to_string();
        }
    }

    text.to_string()
}

/// Aggregate judge results across many evaluations
pub fn aggregate_judge_results(results: &[JudgeResult]) -> JudgeSummary {
    if results.is_empty() {
        return JudgeSummary::default();
    }

    let n = results.len() as f64;

    // Average scores per criterion
    let mut criterion_averages: HashMap<String, f64> = HashMap::new();
    for result in results {
        for (criterion, score) in &result.scores {
            *criterion_averages.entry(criterion.clone()).or_insert(0.0) += score / n;
        }
    }

    // Overall average
    let overall_average = results.iter().map(|r| r.overall_score).sum::<f64>() / n;

    // Confidence-weighted average
    let total_confidence: f64 = results.iter().map(|r| r.confidence).sum();
    let confidence_weighted = if total_confidence > 0.0 {
        results.iter()
            .map(|r| r.overall_score * r.confidence)
            .sum::<f64>() / total_confidence
    } else {
        overall_average
    };

    // Standard deviation
    let variance = results.iter()
        .map(|r| (r.overall_score - overall_average).powi(2))
        .sum::<f64>() / n;
    let std_dev = variance.sqrt();

    // Common suggested improvements
    let mut improvement_counts: HashMap<&str, usize> = HashMap::new();
    for result in results {
        for improvement in &result.suggested_improvements {
            // Normalize to lowercase and count
            *improvement_counts.entry(improvement.as_str()).or_insert(0) += 1;
        }
    }
    let mut common_improvements: Vec<_> = improvement_counts.into_iter().collect();
    common_improvements.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
    let top_improvements: Vec<String> = common_improvements.into_iter()
        .take(5)
        .map(|(imp, _)| imp.to_string())
        .collect();

    JudgeSummary {
        total_evaluated: results.len(),
        overall_average,
        confidence_weighted_average: confidence_weighted,
        std_deviation: std_dev,
        criterion_averages,
        common_improvements: top_improvements,
        low_scorers: results.iter()
            .filter(|r| r.overall_score < 2.5)
            .count(),
        high_scorers: results.iter()
            .filter(|r| r.overall_score >= 4.0)
            .count(),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct JudgeSummary {
    pub total_evaluated: usize,
    pub overall_average: f64,
    pub confidence_weighted_average: f64,
    pub std_deviation: f64,
    pub criterion_averages: HashMap<String, f64>,
    pub common_improvements: Vec<String>,
    pub low_scorers: usize,
    pub high_scorers: usize,
}

/// Synthetic ground truth generator using LLM consensus
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyntheticGroundTruth {
    pub expected_principles: Vec<String>,
    pub expected_thinkers: Vec<String>,
    pub anti_principles: Vec<String>,
    pub difficulty: u8,
    pub confidence: f64,
}

/// Prompt for generating ground truth annotations
pub fn build_annotation_prompt(question: &str, available_principles: &[String]) -> String {
    format!(r#"You are annotating ground truth for a decision-support benchmark.

## Question
{question}

## Available Principles (sample)
{principles}

## Your Task
For this decision question, identify:
1. Which principles SHOULD be cited (most relevant to answering this question)
2. Which thinkers SHOULD appear (credible authorities for this domain)
3. Which principles would be WRONG to cite (anti-patterns for this question)
4. Difficulty level (1=easy, 5=complex multi-factor decision)

Output JSON only:

```json
{{
  "expected_principles": ["<principle name 1>", "<principle name 2>"],
  "expected_thinkers": ["<thinker name 1>", "<thinker name 2>"],
  "anti_principles": ["<principle that would be wrong>"],
  "difficulty": <1-5>,
  "confidence": <0.0-1.0>
}}
```

Be selective - only include principles that DIRECTLY address this specific question.
"#,
        question = question,
        principles = available_principles.join(", "),
    )
}

/// Parse annotation response
pub fn parse_annotation_response(json_str: &str) -> Result<SyntheticGroundTruth> {
    let clean = extract_json(json_str);
    let truth: SyntheticGroundTruth = serde_json::from_str(&clean)?;
    Ok(truth)
}

/// Consensus-based annotation (multi-judge)
pub fn build_consensus_annotation(
    annotations: &[SyntheticGroundTruth],
    min_agreement: usize,
) -> SyntheticGroundTruth {
    let mut principle_counts: HashMap<String, usize> = HashMap::new();
    let mut thinker_counts: HashMap<String, usize> = HashMap::new();
    let mut anti_counts: HashMap<String, usize> = HashMap::new();

    for ann in annotations {
        for p in &ann.expected_principles {
            *principle_counts.entry(p.to_lowercase()).or_insert(0) += 1;
        }
        for t in &ann.expected_thinkers {
            *thinker_counts.entry(t.to_lowercase()).or_insert(0) += 1;
        }
        for a in &ann.anti_principles {
            *anti_counts.entry(a.to_lowercase()).or_insert(0) += 1;
        }
    }

    // Only keep items with sufficient agreement
    let expected_principles: Vec<String> = principle_counts.into_iter()
        .filter(|(_, count)| *count >= min_agreement)
        .map(|(p, _)| p)
        .collect();

    let expected_thinkers: Vec<String> = thinker_counts.into_iter()
        .filter(|(_, count)| *count >= min_agreement)
        .map(|(t, _)| t)
        .collect();

    let anti_principles: Vec<String> = anti_counts.into_iter()
        .filter(|(_, count)| *count >= min_agreement)
        .map(|(a, _)| a)
        .collect();

    // Average difficulty and confidence
    let n = annotations.len() as f64;
    let avg_difficulty = annotations.iter()
        .map(|a| a.difficulty as f64)
        .sum::<f64>() / n;
    let avg_confidence = annotations.iter()
        .map(|a| a.confidence)
        .sum::<f64>() / n;

    SyntheticGroundTruth {
        expected_principles,
        expected_thinkers,
        anti_principles,
        difficulty: avg_difficulty.round() as u8,
        confidence: avg_confidence,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_eval_response() {
        let json = r#"```json
{
  "scores": {
    "Relevance": 4,
    "Completeness": 3,
    "Actionability": 5,
    "Balance": 4,
    "Authority": 4
  },
  "reasoning": "Good response overall",
  "suggested_improvements": ["Add more examples"],
  "confidence": 0.85
}
```"#;

        let result = parse_eval_response(json).unwrap();
        assert_eq!(result.scores.get("Relevance"), Some(&4.0));
        assert_eq!(result.confidence, 0.85);
    }

    #[test]
    fn test_parse_pairwise_response() {
        let json = r#"{"winner": "A", "confidence": 0.9, "reasoning": "More relevant"}"#;
        let result = parse_pairwise_response(json).unwrap();
        assert_eq!(result.winner, PairwiseWinner::A);
    }

    #[test]
    fn test_aggregate_judge_results() {
        let results = vec![
            JudgeResult {
                question: "Q1".into(),
                scores: [("Relevance".into(), 4.0)].into_iter().collect(),
                overall_score: 4.0,
                reasoning: String::new(),
                suggested_improvements: vec!["Add examples".into()],
                confidence: 0.8,
            },
            JudgeResult {
                question: "Q2".into(),
                scores: [("Relevance".into(), 3.0)].into_iter().collect(),
                overall_score: 3.0,
                reasoning: String::new(),
                suggested_improvements: vec!["Add examples".into()],
                confidence: 0.9,
            },
        ];

        let summary = aggregate_judge_results(&results);
        assert_eq!(summary.total_evaluated, 2);
        assert!((summary.overall_average - 3.5).abs() < 0.01);
    }
}
