//! LLM-as-Judge Evaluation
//!
//! Uses Claude Haiku to evaluate decision quality on multiple dimensions.
//! Fast (~100ms), cheap (~$0.001/judgment), and reasonably accurate.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration for LLM judge
#[derive(Debug, Clone)]
pub struct JudgeConfig {
    /// API base URL
    pub api_url: String,

    /// API key (from environment)
    pub api_key: String,

    /// Model to use (default: claude-3-haiku-20240307)
    pub model: String,

    /// Rubric to evaluate against
    pub rubric: JudgeRubric,

    /// Whether to do pairwise comparisons
    pub pairwise: bool,
}

impl Default for JudgeConfig {
    fn default() -> Self {
        Self {
            api_url: "https://api.anthropic.com/v1/messages".to_string(),
            api_key: std::env::var("ANTHROPIC_API_KEY").unwrap_or_default(),
            model: "claude-3-haiku-20240307".to_string(),
            rubric: JudgeRubric::default(),
            pairwise: false,
        }
    }
}

/// Evaluation rubric with criteria
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JudgeRubric {
    pub criteria: Vec<JudgeCriterion>,
}

impl Default for JudgeRubric {
    fn default() -> Self {
        Self {
            criteria: vec![
                JudgeCriterion {
                    name: "relevance".to_string(),
                    description: "Do the cited principles directly address the decision question?"
                        .to_string(),
                    weight: 0.25,
                    scale_descriptions: vec![
                        (1, "Principles are unrelated to the question".to_string()),
                        (2, "Some tangential relevance".to_string()),
                        (3, "Generally relevant but missing key aspects".to_string()),
                        (4, "Mostly relevant with minor gaps".to_string()),
                        (
                            5,
                            "Highly relevant, directly addresses the question".to_string(),
                        ),
                    ]
                    .into_iter()
                    .collect(),
                },
                JudgeCriterion {
                    name: "completeness".to_string(),
                    description: "Are important considerations and perspectives covered?"
                        .to_string(),
                    weight: 0.20,
                    scale_descriptions: vec![
                        (1, "Major perspectives missing".to_string()),
                        (2, "Several important considerations absent".to_string()),
                        (3, "Covers basics but lacks depth".to_string()),
                        (4, "Good coverage with minor omissions".to_string()),
                        (5, "Comprehensive coverage of all key angles".to_string()),
                    ]
                    .into_iter()
                    .collect(),
                },
                JudgeCriterion {
                    name: "actionability".to_string(),
                    description: "Can the user act on this advice immediately?".to_string(),
                    weight: 0.20,
                    scale_descriptions: vec![
                        (1, "Vague platitudes, no concrete guidance".to_string()),
                        (2, "Some direction but unclear next steps".to_string()),
                        (
                            3,
                            "Reasonable guidance but needs interpretation".to_string(),
                        ),
                        (4, "Clear actionable steps with minor ambiguity".to_string()),
                        (5, "Immediately actionable with specific steps".to_string()),
                    ]
                    .into_iter()
                    .collect(),
                },
                JudgeCriterion {
                    name: "balance".to_string(),
                    description: "Are FOR/AGAINST positions genuinely opposed and fair?"
                        .to_string(),
                    weight: 0.15,
                    scale_descriptions: vec![
                        (1, "One-sided or strawman arguments".to_string()),
                        (2, "Weak counter-arguments".to_string()),
                        (3, "Both sides present but uneven strength".to_string()),
                        (4, "Good balance with genuine tension".to_string()),
                        (
                            5,
                            "Excellent adversarial balance, both sides compelling".to_string(),
                        ),
                    ]
                    .into_iter()
                    .collect(),
                },
                JudgeCriterion {
                    name: "authority".to_string(),
                    description: "Are the cited thinkers credible for this domain?".to_string(),
                    weight: 0.20,
                    scale_descriptions: vec![
                        (1, "Thinkers have no relevant expertise".to_string()),
                        (2, "Tangentially related expertise".to_string()),
                        (3, "General relevant expertise".to_string()),
                        (4, "Strong domain expertise".to_string()),
                        (5, "Recognized authorities in the specific area".to_string()),
                    ]
                    .into_iter()
                    .collect(),
                },
            ],
        }
    }
}

/// Single evaluation criterion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JudgeCriterion {
    pub name: String,
    pub description: String,
    pub weight: f64,
    pub scale_descriptions: HashMap<u8, String>,
}

/// Judgment result for a single response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Judgment {
    pub question: String,
    pub scores: HashMap<String, u8>,
    pub weighted_score: f64,
    pub reasoning: String,
    pub strengths: Vec<String>,
    pub weaknesses: Vec<String>,
}

/// Results from running LLM judge on multiple responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JudgeResults {
    pub total_evaluated: usize,
    pub average_scores: HashMap<String, f64>,
    pub overall_weighted_score: f64,
    pub individual_judgments: Vec<Judgment>,
    pub pairwise_results: Option<PairwiseResults>,
}

/// Pairwise comparison results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairwiseResults {
    pub wins: u32,
    pub losses: u32,
    pub ties: u32,
    pub win_rate: f64,
}

/// Run LLM judge evaluation
pub async fn evaluate_responses(
    config: &JudgeConfig,
    responses: &[EvalResponse],
) -> Result<JudgeResults> {
    if config.api_key.is_empty() {
        return Err(anyhow::anyhow!(
            "ANTHROPIC_API_KEY not set. LLM judge requires API access."
        ));
    }

    let client = reqwest::Client::new();
    let mut judgments = Vec::new();
    let mut score_sums: HashMap<String, f64> = HashMap::new();

    for response in responses {
        let judgment = evaluate_single(&client, config, response).await?;

        // Accumulate scores
        for (criterion, score) in &judgment.scores {
            *score_sums.entry(criterion.clone()).or_insert(0.0) += *score as f64;
        }

        judgments.push(judgment);
    }

    // Calculate averages
    let n = responses.len() as f64;
    let average_scores: HashMap<String, f64> =
        score_sums.into_iter().map(|(k, v)| (k, v / n)).collect();

    // Calculate weighted average
    let overall = config
        .rubric
        .criteria
        .iter()
        .map(|c| average_scores.get(&c.name).unwrap_or(&0.0) * c.weight)
        .sum::<f64>()
        / config.rubric.criteria.iter().map(|c| c.weight).sum::<f64>()
        * 5.0;

    Ok(JudgeResults {
        total_evaluated: responses.len(),
        average_scores,
        overall_weighted_score: overall,
        individual_judgments: judgments,
        pairwise_results: None,
    })
}

/// Response to evaluate
#[derive(Debug, Clone)]
pub struct EvalResponse {
    pub question: String,
    pub response_text: String,
    pub thinkers_cited: Vec<String>,
    pub principles_cited: Vec<String>,
}

/// Evaluate a single response
async fn evaluate_single(
    client: &reqwest::Client,
    config: &JudgeConfig,
    response: &EvalResponse,
) -> Result<Judgment> {
    let prompt = build_judge_prompt(config, response);

    let request_body = serde_json::json!({
        "model": config.model,
        "max_tokens": 1024,
        "messages": [
            {
                "role": "user",
                "content": prompt
            }
        ]
    });

    let api_response = client
        .post(&config.api_url)
        .header("x-api-key", &config.api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&request_body)
        .send()
        .await?;

    if !api_response.status().is_success() {
        let error_text = api_response.text().await?;
        return Err(anyhow::anyhow!("API error: {}", error_text));
    }

    let response_json: serde_json::Value = api_response.json().await?;

    // Parse the judge's response
    let content = response_json["content"][0]["text"].as_str().unwrap_or("");

    parse_judgment(content, &response.question)
}

/// Build the prompt for the LLM judge
fn build_judge_prompt(config: &JudgeConfig, response: &EvalResponse) -> String {
    let criteria_text: String = config
        .rubric
        .criteria
        .iter()
        .map(|c| {
            let scale = c
                .scale_descriptions
                .iter()
                .map(|(k, v)| format!("  {}: {}", k, v))
                .collect::<Vec<_>>()
                .join("\n");
            format!(
                "### {}\n{}\n{}",
                c.name.to_uppercase(),
                c.description,
                scale
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    format!(
        r#"You are evaluating the quality of decision advice from an AI wisdom system.

## Question Asked
{question}

## Response to Evaluate
{response}

## Thinkers Cited
{thinkers}

## Principles Applied
{principles}

## Evaluation Criteria

{criteria}

## Instructions

Rate each criterion from 1-5 using the scales above. Be rigorous and honest.

Respond in this exact format:

SCORES:
relevance: [1-5]
completeness: [1-5]
actionability: [1-5]
balance: [1-5]
authority: [1-5]

STRENGTHS:
- [strength 1]
- [strength 2]

WEAKNESSES:
- [weakness 1]
- [weakness 2]

REASONING:
[2-3 sentences explaining the overall assessment]
"#,
        question = response.question,
        response = response.response_text,
        thinkers = response.thinkers_cited.join(", "),
        principles = response.principles_cited.join(", "),
        criteria = criteria_text,
    )
}

/// Parse the LLM's judgment response
fn parse_judgment(content: &str, question: &str) -> Result<Judgment> {
    let mut scores = HashMap::new();
    let mut strengths = Vec::new();
    let mut weaknesses = Vec::new();
    let mut reasoning = String::new();

    // Parse scores
    for criterion in [
        "relevance",
        "completeness",
        "actionability",
        "balance",
        "authority",
    ] {
        if let Some(line) = content.lines().find(|l| l.starts_with(criterion)) {
            if let Some(score_str) = line.split(':').nth(1) {
                if let Ok(score) = score_str.trim().parse::<u8>() {
                    scores.insert(criterion.to_string(), score.clamp(1, 5));
                }
            }
        }
    }

    // Fill in defaults for missing scores
    for criterion in [
        "relevance",
        "completeness",
        "actionability",
        "balance",
        "authority",
    ] {
        scores.entry(criterion.to_string()).or_insert(3);
    }

    // Parse strengths
    let mut in_strengths = false;
    let mut in_weaknesses = false;
    let mut in_reasoning = false;

    for line in content.lines() {
        let line = line.trim();

        if line.starts_with("STRENGTHS:") {
            in_strengths = true;
            in_weaknesses = false;
            in_reasoning = false;
            continue;
        }
        if line.starts_with("WEAKNESSES:") {
            in_strengths = false;
            in_weaknesses = true;
            in_reasoning = false;
            continue;
        }
        if line.starts_with("REASONING:") {
            in_strengths = false;
            in_weaknesses = false;
            in_reasoning = true;
            continue;
        }

        if in_strengths && line.starts_with('-') {
            strengths.push(line.trim_start_matches('-').trim().to_string());
        }
        if in_weaknesses && line.starts_with('-') {
            weaknesses.push(line.trim_start_matches('-').trim().to_string());
        }
        if in_reasoning && !line.is_empty() {
            if !reasoning.is_empty() {
                reasoning.push(' ');
            }
            reasoning.push_str(line);
        }
    }

    // Calculate weighted score
    let default_rubric = JudgeRubric::default();
    let weighted_score = default_rubric
        .criteria
        .iter()
        .map(|c| *scores.get(&c.name).unwrap_or(&3) as f64 * c.weight)
        .sum::<f64>()
        / default_rubric
            .criteria
            .iter()
            .map(|c| c.weight)
            .sum::<f64>();

    Ok(Judgment {
        question: question.to_string(),
        scores,
        weighted_score,
        reasoning,
        strengths,
        weaknesses,
    })
}

/// Run pairwise comparison between 100minds and a baseline
pub async fn compare_pairwise(
    config: &JudgeConfig,
    pairs: &[(EvalResponse, EvalResponse)], // (100minds, baseline)
) -> Result<PairwiseResults> {
    if config.api_key.is_empty() {
        return Err(anyhow::anyhow!("ANTHROPIC_API_KEY not set"));
    }

    let client = reqwest::Client::new();
    let mut wins = 0u32;
    let mut losses = 0u32;
    let mut ties = 0u32;

    for (a, b) in pairs {
        let winner = compare_single(&client, config, a, b).await?;
        match winner.as_str() {
            "A" => wins += 1,
            "B" => losses += 1,
            _ => ties += 1,
        }
    }

    let total = (wins + losses + ties) as f64;
    let win_rate = if total > 0.0 {
        wins as f64 / total
    } else {
        0.5
    };

    Ok(PairwiseResults {
        wins,
        losses,
        ties,
        win_rate,
    })
}

/// Compare two responses head-to-head
async fn compare_single(
    client: &reqwest::Client,
    config: &JudgeConfig,
    a: &EvalResponse,
    b: &EvalResponse,
) -> Result<String> {
    let prompt = format!(
        r#"Compare these two responses to the question: "{}"

RESPONSE A:
{}

RESPONSE B:
{}

Which response provides better decision guidance? Consider:
- Relevance to the question
- Actionability of advice
- Balance of perspectives
- Credibility of sources

Reply with ONLY one of: "A", "B", or "TIE"
"#,
        a.question, a.response_text, b.response_text
    );

    let request_body = serde_json::json!({
        "model": config.model,
        "max_tokens": 10,
        "messages": [{"role": "user", "content": prompt}]
    });

    let response = client
        .post(&config.api_url)
        .header("x-api-key", &config.api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&request_body)
        .send()
        .await?;

    let json: serde_json::Value = response.json().await?;
    let result = json["content"][0]["text"]
        .as_str()
        .unwrap_or("TIE")
        .trim()
        .to_uppercase();

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_judgment() {
        let content = r#"
SCORES:
relevance: 4
completeness: 3
actionability: 5
balance: 4
authority: 4

STRENGTHS:
- Clear actionable advice
- Good use of Brooks' Law

WEAKNESSES:
- Missing cost considerations

REASONING:
Overall a solid response with good balance. Could improve coverage of financial aspects.
"#;

        let judgment = parse_judgment(content, "Test question").unwrap();

        assert_eq!(judgment.scores.get("relevance"), Some(&4));
        assert_eq!(judgment.scores.get("actionability"), Some(&5));
        assert!(judgment.strengths.len() >= 2);
        assert!(judgment.weaknesses.len() >= 1);
        assert!(!judgment.reasoning.is_empty());
    }

    #[test]
    fn test_default_rubric() {
        let rubric = JudgeRubric::default();
        assert_eq!(rubric.criteria.len(), 5);

        let total_weight: f64 = rubric.criteria.iter().map(|c| c.weight).sum();
        assert!((total_weight - 1.0).abs() < 0.01);
    }
}
