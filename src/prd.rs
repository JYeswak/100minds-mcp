//! PRD Generation and Processing
//!
//! 100minds-powered PRD creation that applies philosophical frameworks:
//! - Brooks's Law: Max 5 stories per PRD
//! - YAGNI: No speculative features
//! - Conceptual Integrity: Single domain per PRD
//! - 80/20: Prioritize high-impact stories first
//!
//! Output format is compatible with prd-to-beads → Zesty pipeline.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// A complete PRD document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prd {
    #[serde(default, alias = "prd_id")]
    pub id: String,
    #[serde(default, alias = "name")]
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_path: Option<String>,
    pub stories: Vec<Story>,

    /// 100minds metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minds_metadata: Option<MindsMetadata>,
}

/// A story within a PRD
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Story {
    pub id: String,
    pub title: String,
    pub description: String,
    #[serde(rename = "type", default = "default_story_type")]
    pub story_type: String,
    #[serde(default = "default_priority", deserialize_with = "deserialize_priority")]
    pub priority: String,
    #[serde(rename = "dependsOn", alias = "dependencies", default)]
    pub depends_on: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none", alias = "acceptanceCriteria")]
    pub acceptance_criteria: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
}

/// Deserialize priority from either string ("P2") or integer (2)
fn deserialize_priority<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::{self, Visitor};

    struct PriorityVisitor;

    impl<'de> Visitor<'de> for PriorityVisitor {
        type Value = String;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a string like 'P2' or an integer like 2")
        }

        fn visit_str<E>(self, value: &str) -> Result<String, E>
        where
            E: de::Error,
        {
            Ok(value.to_string())
        }

        fn visit_string<E>(self, value: String) -> Result<String, E>
        where
            E: de::Error,
        {
            Ok(value)
        }

        fn visit_i64<E>(self, value: i64) -> Result<String, E>
        where
            E: de::Error,
        {
            Ok(format!("P{}", value))
        }

        fn visit_u64<E>(self, value: u64) -> Result<String, E>
        where
            E: de::Error,
        {
            Ok(format!("P{}", value))
        }
    }

    deserializer.deserialize_any(PriorityVisitor)
}

fn default_story_type() -> String {
    "feature".to_string()
}

fn default_priority() -> String {
    "P2".to_string()
}

/// Metadata added by 100minds validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MindsMetadata {
    pub validation_score: f64,
    pub principles_applied: Vec<String>,
    pub split_recommendation: Option<SplitRecommendation>,
    pub scope_analysis: ScopeAnalysis,
    pub warnings: Vec<String>,
}

/// Recommendation to split a PRD
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SplitRecommendation {
    pub should_split: bool,
    pub reason: String,
    pub suggested_prds: Vec<SuggestedPrd>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestedPrd {
    pub title: String,
    pub story_ids: Vec<String>,
    pub rationale: String,
}

/// Scope analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScopeAnalysis {
    pub in_scope: Vec<String>,
    pub out_of_scope: Vec<String>,
    pub deferred: Vec<DeferredItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeferredItem {
    pub item: String,
    pub reason: String,
    pub suggested_prd: Option<String>,
}

/// Analyze a PRD and add 100minds metadata
pub fn analyze_prd(prd: &mut Prd) -> MindsMetadata {
    let mut score: f64 = 100.0;
    let mut principles = Vec::new();
    let mut warnings = Vec::new();
    let mut in_scope = Vec::new();
    let mut out_of_scope = Vec::new();
    let mut deferred = Vec::new();

    let story_count = prd.stories.len();

    // === BROOKS'S LAW: Story count ===
    if story_count > 10 {
        score -= 30.0;
        warnings.push(format!(
            "Brooks's Law: {} stories is too many. Communication overhead grows quadratically.",
            story_count
        ));
        principles.push("Brooks's Law".to_string());
    } else if story_count > 5 {
        score -= 10.0;
        warnings.push(format!(
            "Brooks's Law: {} stories is borderline. Consider splitting.",
            story_count
        ));
        principles.push("Brooks's Law".to_string());
    }

    // === YAGNI: Speculative language ===
    let speculative = ["future", "might", "could", "maybe", "eventually", "someday", "phase 2", "later"];
    for story in &prd.stories {
        let combined = format!("{} {}", story.title, story.description).to_lowercase();
        for kw in &speculative {
            if combined.contains(kw) {
                score -= 5.0;
                warnings.push(format!(
                    "YAGNI: Story '{}' contains speculative language '{}'",
                    story.id, kw
                ));
                if !principles.contains(&"YAGNI".to_string()) {
                    principles.push("YAGNI".to_string());
                }

                // Mark for deferral
                deferred.push(DeferredItem {
                    item: story.title.clone(),
                    reason: format!("Contains speculative language: '{}'", kw),
                    suggested_prd: Some("future-enhancements".to_string()),
                });
                break;
            }
        }
    }

    // === CONCEPTUAL INTEGRITY: Domain analysis ===
    let domains = detect_domains(&prd.stories);
    if domains.len() > 2 {
        score -= 15.0;
        warnings.push(format!(
            "Conceptual Integrity: PRD spans {} domains: {:?}. Split by domain.",
            domains.len(), domains
        ));
        principles.push("Conceptual Integrity".to_string());
    }

    // === Build scope analysis ===
    for story in &prd.stories {
        // Check if story seems in-scope vs speculative
        let combined = format!("{} {}", story.title, story.description).to_lowercase();
        let is_speculative = speculative.iter().any(|kw| combined.contains(kw));

        if is_speculative {
            out_of_scope.push(format!("{}: {}", story.id, story.title));
        } else {
            in_scope.push(format!("{}: {}", story.id, story.title));
        }
    }

    // === SPLIT RECOMMENDATION ===
    let split_recommendation = generate_split_recommendation(prd, &domains);

    // === 80/20: Identify high-impact stories ===
    if story_count > 3 {
        principles.push("80/20 Analysis".to_string());
        // First story is usually highest impact by convention
        if let Some(first) = prd.stories.first() {
            warnings.push(format!(
                "80/20: Ensure '{}' delivers the most value. If not, reorder.",
                first.title
            ));
        }
    }

    MindsMetadata {
        validation_score: score.max(0.0).min(100.0),
        principles_applied: principles,
        split_recommendation: Some(split_recommendation),
        scope_analysis: ScopeAnalysis {
            in_scope,
            out_of_scope,
            deferred,
        },
        warnings,
    }
}

/// Detect domains from stories
fn detect_domains(stories: &[Story]) -> HashSet<String> {
    let mut domains = HashSet::new();

    for story in stories {
        let combined = format!("{} {}", story.title, story.description).to_lowercase();

        if combined.contains("ui") || combined.contains("frontend") || combined.contains("component") || combined.contains("react") {
            domains.insert("frontend".to_string());
        }
        if combined.contains("api") || combined.contains("endpoint") || combined.contains("backend") || combined.contains("server") {
            domains.insert("backend".to_string());
        }
        if combined.contains("database") || combined.contains("schema") || combined.contains("migration") || combined.contains("sql") {
            domains.insert("database".to_string());
        }
        if combined.contains("test") || combined.contains("spec") || combined.contains("e2e") {
            domains.insert("testing".to_string());
        }
        if combined.contains("deploy") || combined.contains("ci") || combined.contains("docker") || combined.contains("k8s") {
            domains.insert("devops".to_string());
        }
        if combined.contains("auth") || combined.contains("login") || combined.contains("permission") {
            domains.insert("auth".to_string());
        }
    }

    domains
}

/// Generate split recommendation
fn generate_split_recommendation(prd: &Prd, domains: &HashSet<String>) -> SplitRecommendation {
    let story_count = prd.stories.len();

    // Rule 1: More than 5 stories = split
    if story_count > 5 {
        let mut suggested = Vec::new();

        // Split by domain if multiple domains
        if domains.len() > 1 {
            for domain in domains {
                let domain_stories: Vec<String> = prd.stories.iter()
                    .filter(|s| {
                        let combined = format!("{} {}", s.title, s.description).to_lowercase();
                        match domain.as_str() {
                            "frontend" => combined.contains("ui") || combined.contains("frontend") || combined.contains("component"),
                            "backend" => combined.contains("api") || combined.contains("endpoint") || combined.contains("backend"),
                            "database" => combined.contains("database") || combined.contains("schema") || combined.contains("migration"),
                            "testing" => combined.contains("test") || combined.contains("spec"),
                            "devops" => combined.contains("deploy") || combined.contains("ci") || combined.contains("docker"),
                            "auth" => combined.contains("auth") || combined.contains("login"),
                            _ => false,
                        }
                    })
                    .map(|s| s.id.clone())
                    .collect();

                if !domain_stories.is_empty() {
                    suggested.push(SuggestedPrd {
                        title: format!("{} - {} Layer", prd.title, domain),
                        story_ids: domain_stories,
                        rationale: format!("Isolate {} concerns for focused execution", domain),
                    });
                }
            }
        } else {
            // Split by count (first 5, then next 5, etc.)
            let chunks: Vec<_> = prd.stories.chunks(5).collect();
            for (i, chunk) in chunks.iter().enumerate() {
                suggested.push(SuggestedPrd {
                    title: format!("{} - Part {}", prd.title, i + 1),
                    story_ids: chunk.iter().map(|s| s.id.clone()).collect(),
                    rationale: "Keep each PRD to 5 stories max (Brooks's Law)".to_string(),
                });
            }
        }

        return SplitRecommendation {
            should_split: true,
            reason: format!(
                "{} stories exceeds recommended max of 5. Split into {} PRDs.",
                story_count,
                suggested.len()
            ),
            suggested_prds: suggested,
        };
    }

    // Rule 2: Multiple distinct domains = consider split
    if domains.len() > 2 {
        return SplitRecommendation {
            should_split: true,
            reason: format!(
                "PRD spans {} domains ({:?}). Consider one PRD per layer.",
                domains.len(), domains
            ),
            suggested_prds: vec![],
        };
    }

    SplitRecommendation {
        should_split: false,
        reason: "PRD is well-scoped".to_string(),
        suggested_prds: vec![],
    }
}

/// Generate a new PRD from a description
pub fn generate_prd(
    id: &str,
    title: &str,
    description: &str,
    project_path: Option<&str>,
    raw_stories: Vec<RawStory>,
) -> Prd {
    let mut stories = Vec::new();
    let mut cleanup_stories = Vec::new();

    // Process stories, separating cleanup from main
    for (_i, raw) in raw_stories.into_iter().enumerate() {
        let is_cleanup = raw.title.to_lowercase().contains("cleanup")
            || raw.title.to_lowercase().contains("verify")
            || raw.title.to_lowercase().contains("lint")
            || raw.title.to_lowercase().contains("test");

        let story_id = if is_cleanup {
            format!("CL-{:03}", cleanup_stories.len() + 1)
        } else {
            format!("US-{:03}", stories.len() + 1)
        };

        // Build description with PROJECT line
        let full_description = if let Some(path) = project_path {
            format!("{}\n\nPROJECT: {}", raw.description, path)
        } else {
            raw.description.clone()
        };

        let story = Story {
            id: story_id,
            title: raw.title,
            description: full_description,
            story_type: if is_cleanup { "task".to_string() } else { "feature".to_string() },
            priority: raw.priority.unwrap_or_else(|| "P2".to_string()),
            depends_on: raw.depends_on.unwrap_or_default(),
            acceptance_criteria: raw.acceptance_criteria,
            status: Some("open".to_string()),
        };

        if is_cleanup {
            cleanup_stories.push(story);
        } else {
            stories.push(story);
        }
    }

    // Add cleanup stories at the end, dependent on all US stories
    let us_ids: Vec<String> = stories.iter().map(|s| s.id.clone()).collect();
    for mut cleanup in cleanup_stories {
        if cleanup.depends_on.is_empty() {
            cleanup.depends_on = us_ids.clone();
        }
        stories.push(cleanup);
    }

    // Auto-chain stories that don't have explicit dependencies
    // Each US-00X depends on US-00(X-1) by default
    for i in 1..stories.len() {
        let should_chain = stories[i].depends_on.is_empty()
            && stories[i].id.starts_with("US-")
            && stories.get(i - 1).map(|p| p.id.starts_with("US-")).unwrap_or(false);

        if should_chain {
            let prev_id = stories[i - 1].id.clone();
            stories[i].depends_on.push(prev_id);
        }
    }

    Prd {
        id: id.to_string(),
        title: title.to_string(),
        description: description.to_string(),
        project_path: project_path.map(String::from),
        stories,
        minds_metadata: None,
    }
}

/// Raw story input (before processing)
#[derive(Debug, Clone)]
pub struct RawStory {
    pub title: String,
    pub description: String,
    pub priority: Option<String>,
    pub depends_on: Option<Vec<String>>,
    pub acceptance_criteria: Option<Vec<String>>,
}

/// Output PRD as JSON string
pub fn to_json(prd: &Prd) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(prd)
}

/// Parse PRD from JSON string
pub fn from_json(json: &str) -> Result<Prd, serde_json::Error> {
    serde_json::from_str(json)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_prd() {
        let stories = vec![
            RawStory {
                title: "Add login page".to_string(),
                description: "Create login form".to_string(),
                priority: None,
                depends_on: None,
                acceptance_criteria: Some(vec!["Form validates input".to_string()]),
            },
            RawStory {
                title: "Add logout".to_string(),
                description: "Add logout button".to_string(),
                priority: None,
                depends_on: None,
                acceptance_criteria: None,
            },
            RawStory {
                title: "Cleanup: Lint check".to_string(),
                description: "Run ESLint".to_string(),
                priority: None,
                depends_on: None,
                acceptance_criteria: None,
            },
        ];

        let prd = generate_prd(
            "prd-auth-001",
            "User Authentication",
            "Add login/logout functionality",
            Some("/path/to/project"),
            stories,
        );

        assert_eq!(prd.stories.len(), 3);
        assert_eq!(prd.stories[0].id, "US-001");
        assert_eq!(prd.stories[1].id, "US-002");
        assert_eq!(prd.stories[2].id, "CL-001");

        // Cleanup should depend on US stories
        assert!(prd.stories[2].depends_on.contains(&"US-001".to_string()));
        assert!(prd.stories[2].depends_on.contains(&"US-002".to_string()));

        // US-002 should auto-chain to US-001
        assert!(prd.stories[1].depends_on.contains(&"US-001".to_string()));
    }

    #[test]
    fn test_analyze_prd_too_many_stories() {
        let stories: Vec<Story> = (1..=12).map(|i| Story {
            id: format!("US-{:03}", i),
            title: format!("Story {}", i),
            description: "Description".to_string(),
            story_type: "feature".to_string(),
            priority: "P2".to_string(),
            depends_on: vec![],
            acceptance_criteria: None,
            status: None,
        }).collect();

        let mut prd = Prd {
            id: "test".to_string(),
            title: "Test".to_string(),
            description: "Test".to_string(),
            project_path: None,
            stories,
            minds_metadata: None,
        };

        let meta = analyze_prd(&mut prd);

        assert!(meta.validation_score < 80.0);
        assert!(meta.principles_applied.contains(&"Brooks's Law".to_string()));
        assert!(meta.split_recommendation.as_ref().unwrap().should_split);
    }
}
