//! MCP Server for 100minds Decision Intelligence
//!
//! 10x BETTER THAN GENERIC MENTAL MODEL MCPs:
//! 1. Named thinkers, not abstract frameworks
//! 2. Decision templates with guided trees
//! 3. Synergies and tensions between principles
//! 4. Blind spots and anti-pattern detection
//! 5. Outcome learning that adjusts confidence
//! 6. PRD validation with specific principle violations
//! 7. Cryptographic provenance chain
//! 8. Integration with PRD → beads pipeline

use crate::db::{self};
use crate::templates::{self, DecisionTemplate};
use anyhow::Result;
use rusqlite::Connection;
use serde::Serialize;
use serde_json::{json, Value};

// ============================================================================
// MCP TOOL DEFINITIONS - The full API surface
// ============================================================================

/// MCP Tool definitions - comprehensive decision intelligence toolkit
pub fn get_tools() -> Vec<Value> {
    vec![
        // CORE: Adversarial Wisdom Council
        json!({
            "name": "counsel",
            "description": "Get adversarial wisdom council on a decision. Returns FOR, AGAINST, SYNTHESIZE positions from named thinkers (Fred Brooks, Sam Newman, Kent Beck, etc.) with specific principles. Unlike generic 'mental models' tools, this provides: (1) Named authority with citations, (2) Adversarial debate format, (3) Falsification conditions per position, (4) Actionable next steps. 10x better than CognitiveCompass or ThinkingPatterns MCPs.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "question": {
                        "type": "string",
                        "description": "The decision question to get counsel on"
                    },
                    "domain": {
                        "type": "string",
                        "description": "Optional domain hint (software-architecture, entrepreneurship, ai-ml, management-theory)"
                    },
                    "depth": {
                        "type": "string",
                        "enum": ["quick", "standard", "deep"],
                        "description": "How many perspectives to include (quick=3, standard=4, deep=6)"
                    }
                },
                "required": ["question"]
            }
        }),
        // NEW: Decision Template matching
        json!({
            "name": "get_decision_template",
            "description": "Get a guided decision tree for common decisions. Returns: (1) Step-by-step questions to answer, (2) Recommendations based on your situation, (3) Synergies between principles, (4) Tensions to resolve, (5) Blind spots to check, (6) Anti-patterns to avoid. Templates available: monolith-vs-microservices, rewrite-vs-refactor, build-vs-buy, scale-team, technical-debt, mvp-scope, architecture-migration, database-choice.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "template_id": {
                        "type": "string",
                        "description": "Template ID (e.g., 'monolith-vs-microservices') or describe your decision to auto-match"
                    },
                    "question": {
                        "type": "string",
                        "description": "Optional: describe your decision to auto-match to templates"
                    }
                }
            }
        }),
        // NEW: Blind spot analysis
        json!({
            "name": "check_blind_spots",
            "description": "Proactively identify what you might be missing. Returns critical, high, medium, and low severity blind spots for your decision context. Each blind spot includes a CHECK QUESTION you must answer.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "decision_context": {
                        "type": "string",
                        "description": "Describe the decision you're making"
                    },
                    "template_id": {
                        "type": "string",
                        "description": "Optional: specific template to check blind spots from"
                    }
                },
                "required": ["decision_context"]
            }
        }),
        // NEW: Anti-pattern detection
        json!({
            "name": "detect_anti_patterns",
            "description": "Check for known bad patterns in your approach. Returns anti-patterns with symptoms, source thinker, and cures.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "description": {
                        "type": "string",
                        "description": "Describe your current approach/plan"
                    },
                    "domain": {
                        "type": "string",
                        "description": "Optional domain filter"
                    }
                },
                "required": ["description"]
            }
        }),
        // PRD validation with principle violations
        json!({
            "name": "validate_prd",
            "description": "Validate a PRD against philosophical frameworks. Returns: score (0-100), warnings (error/warning/info), suggestions from thinkers, principles applied. Checks: Brooks's Law (>5 stories), YAGNI (speculative language), Conceptual Integrity (mixed domains), Incremental Migration (big-bang keywords), dependency integrity.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "prd_path": {
                        "type": "string",
                        "description": "Path to the PRD JSON file"
                    },
                    "prd_content": {
                        "type": "string",
                        "description": "PRD JSON content (alternative to prd_path)"
                    }
                }
            }
        }),
        // NEW: Pre-work context injection
        json!({
            "name": "pre_work_context",
            "description": "Get relevant frameworks BEFORE starting work on a task. Injects wisdom into your context. Use at the START of any bead/task.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "task_title": {
                        "type": "string",
                        "description": "Title of the task"
                    },
                    "task_description": {
                        "type": "string",
                        "description": "Description of what you're doing"
                    },
                    "task_type": {
                        "type": "string",
                        "enum": ["feature", "bug", "refactor", "research", "audit", "cleanup"],
                        "description": "Type of task"
                    }
                },
                "required": ["task_title", "task_description"]
            }
        }),
        // Outcome recording for learning
        json!({
            "name": "record_outcome",
            "description": "Record the outcome of a decision for learning. CRITICAL for 100minds to get smarter over time. Adjusts confidence in principles based on success/failure.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "decision_id": {
                        "type": "string",
                        "description": "The decision ID from a previous counsel call"
                    },
                    "success": {
                        "type": "boolean",
                        "description": "Whether the decision led to a successful outcome"
                    },
                    "notes": {
                        "type": "string",
                        "description": "Optional notes about what happened"
                    }
                },
                "required": ["decision_id", "success"]
            }
        }),
        // Principle search
        json!({
            "name": "search_principles",
            "description": "Search 66 thinkers and 345+ principles. FTS5 full-text search with confidence scores.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Search query for principles"
                    },
                    "domain": {
                        "type": "string",
                        "description": "Optional domain filter"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum results to return (default 10)"
                    }
                },
                "required": ["query"]
            }
        }),
        // NEW: Get synergies
        json!({
            "name": "get_synergies",
            "description": "Find principles that work well together. Returns principle combinations with combined power descriptions.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "principle": {
                        "type": "string",
                        "description": "A principle name to find synergies for"
                    },
                    "template_id": {
                        "type": "string",
                        "description": "Or: get all synergies from a template"
                    }
                }
            }
        }),
        // NEW: Get tensions
        json!({
            "name": "get_tensions",
            "description": "Find principles that conflict - you must pick one. Returns when to pick A vs B.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "principle_a": {
                        "type": "string",
                        "description": "First principle"
                    },
                    "principle_b": {
                        "type": "string",
                        "description": "Second principle"
                    },
                    "template_id": {
                        "type": "string",
                        "description": "Or: get all tensions from a template"
                    }
                }
            }
        }),
        // NEW: Wisdom stats
        json!({
            "name": "wisdom_stats",
            "description": "Get statistics on decision outcomes. Which principles have the best track record?",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "domain": {
                        "type": "string",
                        "description": "Optional domain filter"
                    },
                    "min_decisions": {
                        "type": "integer",
                        "description": "Minimum decisions to include (default 3)"
                    }
                }
            }
        }),
        // Audit trail
        json!({
            "name": "audit_decision",
            "description": "Get full provenance chain for a decision. Ed25519 signatures + SHA-256 hash chain.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "decision_id": {
                        "type": "string",
                        "description": "The decision ID to audit"
                    },
                    "verify": {
                        "type": "boolean",
                        "description": "Verify cryptographic signatures"
                    }
                },
                "required": ["decision_id"]
            }
        }),
        // ============================================================================
        // SWARM INTEGRATION TOOLS (v2) - For Zesty/swarmd coordination
        // ============================================================================

        // Sync Thompson posteriors for distributed learning
        json!({
            "name": "sync_posteriors",
            "description": "Get Thompson Sampling posteriors for all principles. Used by swarm daemons to synchronize learning across workers. Returns alpha/beta/pulls for each principle, optionally filtered by timestamp.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "since_ts": {
                        "type": "integer",
                        "description": "Unix epoch timestamp - only return posteriors updated since this time"
                    },
                    "domain": {
                        "type": "string",
                        "description": "Optional domain filter to get domain-specific posteriors"
                    }
                }
            }
        }),
        // Batch outcome recording for catch-up sync
        json!({
            "name": "record_outcomes_batch",
            "description": "Record multiple decision outcomes in batch. Used for offline worker catch-up or daemon restart recovery. Each outcome updates Thompson posteriors.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "outcomes": {
                        "type": "array",
                        "description": "Array of outcome records",
                        "items": {
                            "type": "object",
                            "properties": {
                                "decision_id": { "type": "string" },
                                "success": { "type": "boolean" },
                                "principle_ids": {
                                    "type": "array",
                                    "items": { "type": "string" }
                                },
                                "domain": { "type": "string" },
                                "confidence_score": { "type": "number" },
                                "failure_stage": { "type": "string" }
                            },
                            "required": ["decision_id", "success"]
                        }
                    }
                },
                "required": ["outcomes"]
            }
        }),
        // Counterfactual simulation (Phase 2)
        json!({
            "name": "counterfactual_sim",
            "description": "Simulate counsel response excluding specific principles. Used to understand principle importance and explore alternatives.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "question": {
                        "type": "string",
                        "description": "The decision question"
                    },
                    "excluded_principles": {
                        "type": "array",
                        "description": "Principle IDs to exclude from selection",
                        "items": { "type": "string" }
                    },
                    "domain": {
                        "type": "string",
                        "description": "Optional domain hint"
                    }
                },
                "required": ["question", "excluded_principles"]
            }
        }),
    ]
}

// ============================================================================
// PRD VALIDATION - Philosophical framework enforcement
// ============================================================================

#[derive(Debug, Serialize)]
pub struct PrdValidation {
    pub valid: bool,
    pub score: f64,
    pub warnings: Vec<PrdWarning>,
    pub suggestions: Vec<PrdSuggestion>,
    pub principles_applied: Vec<String>,
    pub blind_spots_to_check: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct PrdWarning {
    pub severity: String, // "error", "warning", "info"
    pub principle: String,
    pub thinker: String,
    pub message: String,
    pub story_ids: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct PrdSuggestion {
    pub principle: String,
    pub thinker: String,
    pub suggestion: String,
}

/// Validate a PRD against 100minds principles
pub fn validate_prd(_conn: &Connection, prd_json: &str) -> Result<PrdValidation> {
    let prd: Value = serde_json::from_str(prd_json)?;

    let mut warnings = Vec::new();
    let mut suggestions = Vec::new();
    let mut principles_applied = Vec::new();
    let mut blind_spots = Vec::new();
    let mut score: f64 = 100.0;

    // Extract stories
    let stories = prd
        .get("stories")
        .and_then(|s| s.as_array())
        .map(|a| a.to_vec())
        .unwrap_or_default();

    let story_count = stories.len();

    // === BROOKS'S LAW: Too many stories ===
    if story_count > 10 {
        warnings.push(PrdWarning {
            severity: "error".to_string(),
            principle: "Brooks's Law".to_string(),
            thinker: "Fred Brooks".to_string(),
            message: format!(
                "PRD has {} stories. Communication overhead grows quadratically. \
                Split into multiple PRDs of 3-5 stories each.",
                story_count
            ),
            story_ids: vec![],
        });
        score -= 30.0;
        principles_applied.push("Brooks's Law".to_string());
    } else if story_count > 5 {
        warnings.push(PrdWarning {
            severity: "warning".to_string(),
            principle: "Brooks's Law".to_string(),
            thinker: "Fred Brooks".to_string(),
            message: format!(
                "PRD has {} stories. Consider splitting for better coordination.",
                story_count
            ),
            story_ids: vec![],
        });
        score -= 10.0;
        principles_applied.push("Brooks's Law".to_string());
    }

    // === KENT BECK: YAGNI - Check for speculative features ===
    let speculative_keywords = [
        "future",
        "might",
        "could",
        "maybe",
        "eventually",
        "someday",
        "later",
        "phase 2",
    ];
    for story in &stories {
        let title = story.get("title").and_then(|t| t.as_str()).unwrap_or("");
        let desc = story
            .get("description")
            .and_then(|d| d.as_str())
            .unwrap_or("");
        let combined = format!("{} {}", title, desc).to_lowercase();

        for kw in speculative_keywords {
            if combined.contains(kw) {
                let story_id = story.get("id").and_then(|i| i.as_str()).unwrap_or("?");
                warnings.push(PrdWarning {
                    severity: "warning".to_string(),
                    principle: "YAGNI".to_string(),
                    thinker: "Kent Beck".to_string(),
                    message: format!(
                        "Story '{}' contains speculative language '{}'. \
                        Focus on requirements you need NOW.",
                        story_id, kw
                    ),
                    story_ids: vec![story_id.to_string()],
                });
                score -= 5.0;
                if !principles_applied.contains(&"YAGNI".to_string()) {
                    principles_applied.push("YAGNI".to_string());
                }
                break;
            }
        }
    }

    // === MARTIN FOWLER: Monolith First - Check for premature decomposition ===
    let decomposition_keywords = [
        "microservice",
        "separate service",
        "extract",
        "split into",
        "new service",
    ];
    for story in &stories {
        let desc = story
            .get("description")
            .and_then(|d| d.as_str())
            .unwrap_or("")
            .to_lowercase();

        for kw in decomposition_keywords {
            if desc.contains(kw) {
                let story_id = story.get("id").and_then(|i| i.as_str()).unwrap_or("?");
                warnings.push(PrdWarning {
                    severity: "info".to_string(),
                    principle: "Monolith First".to_string(),
                    thinker: "Martin Fowler".to_string(),
                    message: format!(
                        "Story '{}' mentions service decomposition. \
                        Ensure you understand domain boundaries before extracting services.",
                        story_id
                    ),
                    story_ids: vec![story_id.to_string()],
                });
                if !principles_applied.contains(&"Monolith First".to_string()) {
                    principles_applied.push("Monolith First".to_string());
                }
                blind_spots.push("Do you have a team per service? (Conway's Law)".to_string());
                break;
            }
        }
    }

    // === FRED BROOKS: Conceptual Integrity - Check for mixed concerns ===
    let domains_mentioned: std::collections::HashSet<String> = stories
        .iter()
        .filter_map(|s| s.get("title").and_then(|t| t.as_str()))
        .flat_map(|title| {
            let t = title.to_lowercase();
            let mut domains = vec![];
            if t.contains("ui")
                || t.contains("frontend")
                || t.contains("component")
                || t.contains("react")
            {
                domains.push("frontend".to_string());
            }
            if t.contains("api")
                || t.contains("endpoint")
                || t.contains("backend")
                || t.contains("server")
            {
                domains.push("backend".to_string());
            }
            if t.contains("database")
                || t.contains("schema")
                || t.contains("migration")
                || t.contains("sql")
            {
                domains.push("database".to_string());
            }
            if t.contains("test") || t.contains("spec") || t.contains("e2e") {
                domains.push("testing".to_string());
            }
            if t.contains("deploy") || t.contains("ci") || t.contains("docker") || t.contains("k8s")
            {
                domains.push("devops".to_string());
            }
            domains
        })
        .collect();

    if domains_mentioned.len() > 2 {
        warnings.push(PrdWarning {
            severity: "warning".to_string(),
            principle: "Conceptual Integrity".to_string(),
            thinker: "Fred Brooks".to_string(),
            message: format!(
                "PRD spans {} domains: {:?}. \
                Consider separate PRDs for each layer to maintain conceptual integrity.",
                domains_mentioned.len(),
                domains_mentioned
            ),
            story_ids: vec![],
        });
        score -= 10.0;
        principles_applied.push("Conceptual Integrity".to_string());
    }

    // === TIM FERRISS: 80/20 - Check for high-impact focus ===
    if story_count > 0 {
        suggestions.push(PrdSuggestion {
            principle: "80/20 Analysis".to_string(),
            thinker: "Tim Ferriss".to_string(),
            suggestion: format!(
                "Which 1-2 of these {} stories would deliver 80% of the value? \
                Consider prioritizing those and deferring the rest.",
                story_count
            ),
        });
        principles_applied.push("80/20 Analysis".to_string());
        blind_spots.push("Have you identified the highest-impact story?".to_string());
    }

    // === SAM NEWMAN: Incremental Migration ===
    let big_bang_keywords = [
        "rewrite",
        "replace all",
        "complete overhaul",
        "full migration",
        "rebuild from scratch",
    ];
    for story in &stories {
        let desc = story
            .get("description")
            .and_then(|d| d.as_str())
            .unwrap_or("")
            .to_lowercase();

        for kw in big_bang_keywords {
            if desc.contains(kw) {
                let story_id = story.get("id").and_then(|i| i.as_str()).unwrap_or("?");
                warnings.push(PrdWarning {
                    severity: "error".to_string(),
                    principle: "Incremental Migration".to_string(),
                    thinker: "Sam Newman".to_string(),
                    message: format!(
                        "Story '{}' suggests big-bang approach. \
                        Migrate incrementally - extract one piece, stabilize, repeat.",
                        story_id
                    ),
                    story_ids: vec![story_id.to_string()],
                });
                score -= 15.0;
                if !principles_applied.contains(&"Incremental Migration".to_string()) {
                    principles_applied.push("Incremental Migration".to_string());
                }
                blind_spots.push("What's your rollback plan if the rewrite fails?".to_string());
                break;
            }
        }
    }

    // === Check for missing dependencies ===
    let story_ids: std::collections::HashSet<String> = stories
        .iter()
        .filter_map(|s| s.get("id").and_then(|i| i.as_str()).map(String::from))
        .collect();

    for story in &stories {
        if let Some(deps) = story.get("dependsOn").and_then(|d| d.as_array()) {
            for dep in deps {
                if let Some(dep_id) = dep.as_str() {
                    if !story_ids.contains(dep_id) {
                        let story_id = story.get("id").and_then(|i| i.as_str()).unwrap_or("?");
                        warnings.push(PrdWarning {
                            severity: "error".to_string(),
                            principle: "Dependency Integrity".to_string(),
                            thinker: "System".to_string(),
                            message: format!(
                                "Story '{}' depends on '{}' which doesn't exist in this PRD.",
                                story_id, dep_id
                            ),
                            story_ids: vec![story_id.to_string()],
                        });
                        score -= 20.0;
                    }
                }
            }
        }
    }

    // === ERIC EVANS: Bounded Context - Check for unclear boundaries ===
    let boundary_keywords = ["shared", "common", "global", "universal", "generic"];
    for story in &stories {
        let title = story
            .get("title")
            .and_then(|t| t.as_str())
            .unwrap_or("")
            .to_lowercase();

        for kw in boundary_keywords {
            if title.contains(kw) {
                let story_id = story.get("id").and_then(|i| i.as_str()).unwrap_or("?");
                warnings.push(PrdWarning {
                    severity: "info".to_string(),
                    principle: "Bounded Context".to_string(),
                    thinker: "Eric Evans".to_string(),
                    message: format!(
                        "Story '{}' uses '{}' - ensure you're not forcing a single model across contexts.",
                        story_id, kw
                    ),
                    story_ids: vec![story_id.to_string()],
                });
                if !principles_applied.contains(&"Bounded Context".to_string()) {
                    principles_applied.push("Bounded Context".to_string());
                }
                break;
            }
        }
    }

    // === ROBERT MARTIN: Single Responsibility - Check for overloaded stories ===
    for story in &stories {
        let title = story.get("title").and_then(|t| t.as_str()).unwrap_or("");
        let desc = story
            .get("description")
            .and_then(|d| d.as_str())
            .unwrap_or("");

        // Check for "and" patterns that suggest multiple responsibilities
        let and_count = title.to_lowercase().matches(" and ").count()
            + desc.to_lowercase().matches(" and ").count();

        if and_count >= 2 {
            let story_id = story.get("id").and_then(|i| i.as_str()).unwrap_or("?");
            warnings.push(PrdWarning {
                severity: "warning".to_string(),
                principle: "Single Responsibility".to_string(),
                thinker: "Robert C. Martin".to_string(),
                message: format!(
                    "Story '{}' may have multiple responsibilities (contains {} 'and' patterns). \
                    Consider splitting into separate stories.",
                    story_id, and_count
                ),
                story_ids: vec![story_id.to_string()],
            });
            score -= 5.0;
            if !principles_applied.contains(&"Single Responsibility".to_string()) {
                principles_applied.push("Single Responsibility".to_string());
            }
        }
    }

    // Clamp score
    score = score.max(0.0).min(100.0);

    let valid = score >= 70.0 && !warnings.iter().any(|w| w.severity == "error");

    Ok(PrdValidation {
        valid,
        score,
        warnings,
        suggestions,
        principles_applied,
        blind_spots_to_check: blind_spots,
    })
}

// ============================================================================
// PRE-WORK CONTEXT - Inject wisdom before starting a task
// ============================================================================

#[derive(Debug, Serialize)]
pub struct PreWorkContext {
    pub task_title: String,
    pub task_type: String,
    pub relevant_principles: Vec<RelevantPrinciple>,
    pub blind_spots: Vec<String>,
    pub anti_patterns_to_avoid: Vec<String>,
    pub key_questions: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct RelevantPrinciple {
    pub name: String,
    pub thinker: String,
    pub description: String,
    pub action: String,
}

/// Get context for a task before starting work
pub fn get_pre_work_context(
    conn: &Connection,
    title: &str,
    description: &str,
    task_type: Option<&str>,
) -> Result<PreWorkContext> {
    let task_type = task_type.unwrap_or("feature");
    let query = format!("{} {}", title, description);

    // Find relevant principles
    let principles = db::search_principles(conn, &query, 5)?;

    let relevant_principles: Vec<RelevantPrinciple> = principles
        .iter()
        .map(|p| {
            let thinker_name = conn
                .query_row(
                    "SELECT name FROM thinkers WHERE id = ?1",
                    [&p.thinker_id],
                    |row| row.get::<_, String>(0),
                )
                .unwrap_or_else(|_| p.thinker_id.clone());

            RelevantPrinciple {
                name: p.name.clone(),
                thinker: thinker_name,
                description: p.description.clone(),
                action: generate_action(&p.name, &p.description),
            }
        })
        .collect();

    // Generate blind spots based on task type
    let blind_spots = match task_type {
        "feature" => vec![
            "Who is the user and what's their actual pain point?".to_string(),
            "What's the simplest solution that could work?".to_string(),
            "How will you know this feature is successful?".to_string(),
        ],
        "bug" => vec![
            "Have you reproduced the bug yourself?".to_string(),
            "What changed recently that might have caused this?".to_string(),
            "Is this a symptom or the root cause?".to_string(),
        ],
        "refactor" => vec![
            "Is there a failing test that proves the problem?".to_string(),
            "Are you improving or just changing?".to_string(),
            "What's the smallest refactor that addresses the issue?".to_string(),
        ],
        "research" => vec![
            "What's the specific question you're answering?".to_string(),
            "What would change your recommendation?".to_string(),
            "Who else has solved this problem?".to_string(),
        ],
        _ => vec![
            "What does 'done' look like?".to_string(),
            "What could go wrong?".to_string(),
            "Who needs to know about this?".to_string(),
        ],
    };

    // Generate anti-patterns based on query keywords
    let anti_patterns = generate_anti_patterns_for_context(&query);

    // Generate key questions
    let key_questions = vec![
        "Before starting: What's the ONE thing that would make this task fail?".to_string(),
        "During: Am I making this more complex than necessary?".to_string(),
        "After: Did I leave the code better than I found it?".to_string(),
    ];

    Ok(PreWorkContext {
        task_title: title.to_string(),
        task_type: task_type.to_string(),
        relevant_principles,
        blind_spots,
        anti_patterns_to_avoid: anti_patterns,
        key_questions,
    })
}

fn generate_action(name: &str, description: &str) -> String {
    let name_lower = name.to_lowercase();
    let desc_lower = description.to_lowercase();

    if name_lower.contains("80/20") || desc_lower.contains("high-impact") {
        return "List 5 things. Circle the ONE that matters most. Do only that.".to_string();
    }
    if name_lower.contains("yagni") || desc_lower.contains("you ain't gonna need") {
        return "Remove anything that isn't required for THIS iteration.".to_string();
    }
    if name_lower.contains("simple") || desc_lower.contains("complexity") {
        return "Can you explain this in one sentence? If not, simplify.".to_string();
    }
    if name_lower.contains("incremental") || desc_lower.contains("strangler") {
        return "What's the smallest change you can ship and validate?".to_string();
    }
    if name_lower.contains("boy scout") {
        return "Before you're done, clean up one thing you touched.".to_string();
    }

    "Apply this principle to your next decision point.".to_string()
}

fn generate_anti_patterns_for_context(query: &str) -> Vec<String> {
    let q_lower = query.to_lowercase();
    let mut patterns = Vec::new();

    if q_lower.contains("rewrite") || q_lower.contains("rebuild") {
        patterns.push(
            "Second System Effect: Don't add features the old system didn't have".to_string(),
        );
    }
    if q_lower.contains("service") || q_lower.contains("microservice") {
        patterns.push("Distributed Monolith: If services can't deploy independently, they're not microservices".to_string());
    }
    if q_lower.contains("team") || q_lower.contains("hire") {
        patterns.push("Brooks's Law: Adding people to a late project makes it later".to_string());
    }
    if q_lower.contains("database") || q_lower.contains("schema") {
        patterns.push(
            "Premature Optimization: Get it working first, optimize when you have metrics"
                .to_string(),
        );
    }
    if q_lower.contains("feature") || q_lower.contains("new") {
        patterns
            .push("Feature Creep: Is this in scope? Would users pay for just this?".to_string());
    }

    if patterns.is_empty() {
        patterns.push("Over-engineering: Build the simplest thing that works".to_string());
    }

    patterns
}

// ============================================================================
// TEMPLATE MATCHING AND BLIND SPOT DETECTION
// ============================================================================

/// Get matching decision templates
pub fn get_matching_templates(question: &str) -> Vec<TemplateMatch> {
    templates::match_templates(question)
        .into_iter()
        .map(|(t, score)| TemplateMatch {
            template: t,
            match_score: score,
        })
        .collect()
}

#[derive(Debug, Serialize)]
pub struct TemplateMatch {
    pub template: DecisionTemplate,
    pub match_score: f64,
}

/// Check blind spots for a decision context
pub fn check_blind_spots(context: &str, template_id: Option<&str>) -> BlindSpotAnalysis {
    let mut all_blind_spots = Vec::new();

    // Get blind spots from matching templates
    if let Some(id) = template_id {
        for template in templates::get_templates() {
            if template.id == id {
                for bs in &template.blind_spots {
                    all_blind_spots.push(BlindSpotResult {
                        name: bs.name.clone(),
                        description: bs.description.clone(),
                        check_question: bs.check_question.clone(),
                        severity: format!("{:?}", bs.severity),
                        source_template: template.name.clone(),
                    });
                }
            }
        }
    } else {
        // Match templates from context
        for (template, _score) in templates::match_templates(context) {
            for bs in &template.blind_spots {
                all_blind_spots.push(BlindSpotResult {
                    name: bs.name.clone(),
                    description: bs.description.clone(),
                    check_question: bs.check_question.clone(),
                    severity: format!("{:?}", bs.severity),
                    source_template: template.name.clone(),
                });
            }
        }
    }

    // Add generic blind spots based on keywords
    let context_lower = context.to_lowercase();

    if !context_lower.contains("time") && !context_lower.contains("deadline") {
        all_blind_spots.push(BlindSpotResult {
            name: "Timeline".to_string(),
            description: "You haven't mentioned time constraints".to_string(),
            check_question: "What's the deadline? What happens if you miss it?".to_string(),
            severity: "Medium".to_string(),
            source_template: "Generic".to_string(),
        });
    }

    if !context_lower.contains("rollback") && !context_lower.contains("revert") {
        all_blind_spots.push(BlindSpotResult {
            name: "Rollback Plan".to_string(),
            description: "No rollback strategy mentioned".to_string(),
            check_question: "What if this fails? How do you undo it?".to_string(),
            severity: "High".to_string(),
            source_template: "Generic".to_string(),
        });
    }

    // Sort by severity
    all_blind_spots.sort_by(|a, b| {
        let severity_order = |s: &str| match s {
            "Critical" => 0,
            "High" => 1,
            "Medium" => 2,
            _ => 3,
        };
        severity_order(&a.severity).cmp(&severity_order(&b.severity))
    });

    let critical_count = all_blind_spots
        .iter()
        .filter(|b| b.severity == "Critical")
        .count() as u32;

    BlindSpotAnalysis {
        context: context.to_string(),
        blind_spots: all_blind_spots,
        critical_count,
    }
}

#[derive(Debug, Serialize)]
pub struct BlindSpotAnalysis {
    pub context: String,
    pub blind_spots: Vec<BlindSpotResult>,
    pub critical_count: u32,
}

#[derive(Debug, Serialize, Clone)]
pub struct BlindSpotResult {
    pub name: String,
    pub description: String,
    pub check_question: String,
    pub severity: String,
    pub source_template: String,
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_small_prd() {
        let prd = r#"{
            "stories": [
                {"id": "US-001", "title": "Add login", "description": "Add user login"},
                {"id": "US-002", "title": "Add logout", "description": "Add user logout"}
            ]
        }"#;

        let conn = rusqlite::Connection::open_in_memory().unwrap();
        let result = validate_prd(&conn, prd).unwrap();

        assert!(result.valid);
        assert!(result.score >= 90.0);
    }

    #[test]
    fn test_validate_oversized_prd() {
        let stories: Vec<_> = (1..=15)
            .map(|i| {
                format!(
                    r#"{{"id": "US-{:03}", "title": "Story {}", "description": ""}}"#,
                    i, i
                )
            })
            .collect();

        let prd = format!(r#"{{"stories": [{}]}}"#, stories.join(","));

        let conn = rusqlite::Connection::open_in_memory().unwrap();
        let result = validate_prd(&conn, &prd).unwrap();

        assert!(!result.valid);
        assert!(result
            .warnings
            .iter()
            .any(|w| w.principle == "Brooks's Law"));
    }

    #[test]
    fn test_validate_speculative_prd() {
        let prd = r#"{
            "stories": [
                {"id": "US-001", "title": "Add login", "description": "Add user login"},
                {"id": "US-002", "title": "Future: Add OAuth", "description": "Maybe add OAuth someday"}
            ]
        }"#;

        let conn = rusqlite::Connection::open_in_memory().unwrap();
        let result = validate_prd(&conn, prd).unwrap();

        assert!(result.warnings.iter().any(|w| w.principle == "YAGNI"));
    }

    #[test]
    fn test_template_matching() {
        let matches =
            get_matching_templates("Should we use microservices or stay with our monolith?");
        assert!(!matches.is_empty());
        assert!(matches[0].template.id == "monolith-vs-microservices");
    }

    #[test]
    fn test_blind_spots() {
        let analysis = check_blind_spots("We want to migrate to microservices", None);
        assert!(analysis.blind_spots.len() > 0);
    }
}
