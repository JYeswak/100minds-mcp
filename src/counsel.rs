//! Adversarial Counsel Engine
//!
//! The core of 100minds - generates multi-perspective debates on decisions.
//!
//! Philosophy:
//! - Taleb: Via negativa - what NOT to do is more valuable than what to do
//! - Popper: Only falsifiable advice is useful
//! - Feynman: If it can't be explained simply, it's not understood

use crate::db::{self, PrincipleMatch};
use crate::provenance::Provenance;
use crate::types::*;
use anyhow::Result;
use rusqlite::Connection;

/// The counsel engine that generates adversarial debates
pub struct CounselEngine<'a> {
    conn: &'a Connection,
    provenance: &'a Provenance,
}

impl<'a> CounselEngine<'a> {
    pub fn new(conn: &'a Connection, provenance: &'a Provenance) -> Self {
        Self { conn, provenance }
    }

    /// Generate adversarial counsel for a decision question
    pub fn counsel(&self, request: &CounselRequest) -> Result<CounselResponse> {
        // 1. Find relevant principles for this question
        let principles = self.find_relevant_principles(request)?;

        // 2. Generate positions from different perspectives
        let positions = self.generate_positions(request, &principles)?;

        // 3. Always generate a devil's advocate challenge
        let challenge = self.generate_challenge(request, &positions)?;

        // 4. Create provenance for this decision
        let provenance_info = self.create_provenance(request, &positions, &challenge)?;

        // 5. Build the response
        let mut response = CounselResponse::new(
            request.question.clone(),
            positions,
            challenge,
            provenance_info,
        );

        // 6. Detect urgency for swarm integration
        response.urgency_adjustment = self.detect_urgency(request, &response.positions);

        // 7. Store the decision in the database
        self.store_decision(&response, request)?;

        Ok(response)
    }

    /// Detect urgency based on question content and position analysis
    /// Returns "escalate" | "defer" | None
    fn detect_urgency(
        &self,
        request: &CounselRequest,
        positions: &[CounselPosition],
    ) -> Option<String> {
        let q = request.question.to_lowercase();

        // ESCALATE signals - indicates decision needs human review or senior attention
        let escalate_keywords = [
            "security",
            "vulnerable",
            "breach",
            "hack",
            "data loss",
            "corruption",
            "production down",
            "breaking change",
            "backwards compat",
            "legal",
            "compliance",
            "gdpr",
            "pii",
            "money",
            "billing",
            "payment",
            "deadline",
            "blocker",
            "critical",
        ];

        let escalate_score: usize = escalate_keywords
            .iter()
            .filter(|kw| q.contains(*kw))
            .count();

        // DEFER signals - indicates decision can wait for more information
        let defer_keywords = [
            "future",
            "eventually",
            "someday",
            "maybe",
            "nice to have",
            "phase 2",
            "later",
            "considering",
            "thinking about",
            "exploring",
            "research",
            "spike",
            "poc",
            "prototype",
        ];

        let defer_score: usize = defer_keywords.iter().filter(|kw| q.contains(*kw)).count();

        // Analyze position confidence spread
        let confidences: Vec<f64> = positions.iter().map(|p| p.confidence).collect();
        let avg_confidence = if !confidences.is_empty() {
            confidences.iter().sum::<f64>() / confidences.len() as f64
        } else {
            0.5
        };

        // Low confidence + high stakes = escalate
        if avg_confidence < 0.5 && escalate_score >= 1 {
            return Some("escalate".to_string());
        }

        // High confidence + escalate keywords = still escalate (stakes matter)
        if escalate_score >= 2 {
            return Some("escalate".to_string());
        }

        // Defer keywords present = defer
        if defer_score >= 2 {
            return Some("defer".to_string());
        }

        // Check for conflicting positions (FOR and AGAINST with similar confidence)
        let has_for = positions.iter().any(|p| p.stance == Stance::For);
        let has_against = positions.iter().any(|p| p.stance == Stance::Against);

        if has_for && has_against {
            let for_confidence: f64 = positions
                .iter()
                .filter(|p| p.stance == Stance::For)
                .map(|p| p.confidence)
                .sum::<f64>();
            let against_confidence: f64 = positions
                .iter()
                .filter(|p| p.stance == Stance::Against)
                .map(|p| p.confidence)
                .sum::<f64>();

            // Near-equal confidence = contentious decision, escalate
            let diff = (for_confidence - against_confidence).abs();
            if diff < 0.2 && (for_confidence + against_confidence) > 1.0 {
                return Some("escalate".to_string());
            }
        }

        None
    }

    /// Counterfactual simulation - what would we recommend without certain principles?
    /// Used to discover overlooked wisdom and understand principle importance
    pub fn counterfactual_counsel(
        &self,
        request: &CounselRequest,
        excluded_principles: &[String],
    ) -> Result<CounterfactualResponse> {
        // 1. Get original response for comparison
        let original = self.counsel(request)?;
        let original_principle_ids: std::collections::HashSet<String> =
            original.principle_ids.iter().cloned().collect();

        // 2. Find principles with exclusions
        let all_principles = self.find_relevant_principles(request)?;
        let filtered_principles: Vec<PrincipleMatch> = all_principles
            .into_iter()
            .filter(|p| !excluded_principles.contains(&p.id))
            .collect();

        // 3. Generate positions from remaining principles
        let alternative_positions =
            self.build_positions_from_principles(request, &filtered_principles)?;

        // 4. Calculate diversity delta
        let new_principle_ids: std::collections::HashSet<String> = alternative_positions
            .iter()
            .flat_map(|p| p.principles_cited.clone())
            .collect();

        // Jaccard distance: 1 - (intersection / union)
        let intersection = original_principle_ids
            .intersection(&new_principle_ids)
            .count();
        let union = original_principle_ids.union(&new_principle_ids).count();
        let diversity_delta = if union > 0 {
            1.0 - (intersection as f64 / union as f64)
        } else {
            0.0
        };

        Ok(CounterfactualResponse {
            question: request.question.clone(),
            excluded_principles: excluded_principles.to_vec(),
            excluded_count: excluded_principles.len(),
            alternative_positions,
            original_principle_ids: original.principle_ids,
            new_principle_ids: new_principle_ids.into_iter().collect(),
            diversity_delta,
        })
    }

    /// Build positions from a set of principles (used by counterfactual)
    fn build_positions_from_principles(
        &self,
        request: &CounselRequest,
        principles: &[PrincipleMatch],
    ) -> Result<Vec<CounselPosition>> {
        let mut positions = Vec::new();
        let num_positions = match request.context.depth {
            CounselDepth::Quick => 3,
            CounselDepth::Standard => 4,
            CounselDepth::Deep => 6,
        };

        // Alternate stances for balance
        let stances = [
            Stance::For,
            Stance::Against,
            Stance::Synthesize,
            Stance::Against,
            Stance::For,
            Stance::Synthesize,
        ];

        for (i, principle) in principles.iter().take(num_positions).enumerate() {
            let stance = stances[i % stances.len()];
            let thinker_name = self
                .conn
                .query_row(
                    "SELECT name FROM thinkers WHERE id = ?1",
                    [&principle.thinker_id],
                    |row| row.get::<_, String>(0),
                )
                .unwrap_or_else(|_| principle.thinker_id.clone());

            let position = CounselPosition {
                thinker: thinker_name,
                thinker_id: principle.thinker_id.clone(),
                stance,
                argument: format!(
                    "{}\n   → ACTION: Apply this in the next 60 seconds. What's ONE concrete step?",
                    principle.description
                ),
                principles_cited: vec![principle.id.clone()],
                confidence: principle.confidence,
                falsifiable_if: Some(format!(
                    "This {} is {} if the {} principle doesn't apply to this context",
                    if stance == Stance::For {
                        "recommendation"
                    } else {
                        "caution"
                    },
                    if stance == Stance::For {
                        "wrong"
                    } else {
                        "unnecessary"
                    },
                    principle.name
                )),
            };
            positions.push(position);
        }

        Ok(positions)
    }

    /// Find principles relevant to the question
    fn find_relevant_principles(&self, request: &CounselRequest) -> Result<Vec<PrincipleMatch>> {
        let mut all_matches = Vec::new();

        // FIRST: Direct keyword search on question (highest relevance)
        let question_matches = db::search_principles(self.conn, &request.question, 20)?;
        all_matches.extend(question_matches);

        // SECOND: Expand with semantic synonyms for common patterns
        let expanded_query = self.expand_query_keywords(&request.question);
        if expanded_query != request.question {
            let expanded_matches = db::search_principles(self.conn, &expanded_query, 10)?;
            all_matches.extend(expanded_matches);
        }

        // SECOND: Domain-based search
        let detected_domains = self.detect_domains(&request.question);
        for domain in &detected_domains {
            let domain_matches = db::get_principles_by_domain(self.conn, domain)?;
            all_matches.extend(domain_matches);
        }

        // User-specified domain
        if let Some(domain) = &request.context.domain {
            let domain_matches = db::get_principles_by_domain(self.conn, domain)?;
            all_matches.extend(domain_matches);
        }

        // Deduplicate by principle ID
        all_matches.sort_by(|a, b| a.id.cmp(&b.id));
        all_matches.dedup_by(|a, b| a.id == b.id);

        // Score each principle by relevance to the question
        for principle in &mut all_matches {
            principle.relevance_score =
                self.score_principle_relevance(&request.question, principle);
        }

        // Sort by relevance score (highest first)
        all_matches.sort_by(|a, b| b.relevance_score.partial_cmp(&a.relevance_score).unwrap());

        Ok(all_matches)
    }

    /// Expand query with semantic synonyms for common patterns
    fn expand_query_keywords(&self, question: &str) -> String {
        let q = question.to_lowercase();
        let mut expanded = question.to_string();

        // Performance/caching questions → optimization principles
        let perf_triggers = [
            "cache",
            "caching",
            "redis",
            "memcache",
            "cdn",
            "fast",
            "slow",
            "latency",
            "performance",
            "optimize",
            "speed",
        ];
        if perf_triggers.iter().any(|t| q.contains(t)) {
            expanded.push_str(" premature optimization simplest YAGNI");
        }

        // Adding/new feature questions → YAGNI, simplicity
        let add_triggers = ["add", "adding", "should we", "implement", "build", "create"];
        if add_triggers.iter().any(|t| q.contains(t)) {
            expanded.push_str(" YAGNI simplest overengineering speculative");
        }

        // Scale/growth questions → Brooks, premature scaling
        let scale_triggers = [
            "scale",
            "scaling",
            "grow",
            "growth",
            "more users",
            "traffic",
        ];
        if scale_triggers.iter().any(|t| q.contains(t)) {
            expanded.push_str(" premature decomposition monolith incremental");
        }

        // Rewrite/refactor questions
        let rewrite_triggers = [
            "rewrite",
            "refactor",
            "rebuild",
            "from scratch",
            "legacy",
            "messy",
            "tangled",
            "spaghetti",
            "cleanup",
            "clean up",
        ];
        if rewrite_triggers.iter().any(|t| q.contains(t)) {
            expanded.push_str(" strangler incremental migration second system incremental design technical debt Kent Beck Ward Cunningham");
        }

        // Team/hiring questions
        let team_triggers = ["hire", "team", "people", "developer", "engineer", "staff"];
        if team_triggers.iter().any(|t| q.contains(t)) {
            expanded.push_str(" Brooks Law late project communication");
        }

        // Testing questions → TDD, test-first, test pyramid
        let test_triggers = [
            "test",
            "tests",
            "testing",
            "tdd",
            "mock",
            "stub",
            "coverage",
            "unit test",
            "integration test",
            "before code",
            "after code",
        ];
        if test_triggers.iter().any(|t| q.contains(t)) {
            expanded.push_str(
                " TDD test-first red-green-refactor test pyramid Kent Beck Michael Feathers",
            );
        }

        // Architecture questions → YAGNI, simplest, monolith first, incremental
        let arch_triggers = [
            "microservice",
            "monolith",
            "api",
            "architecture",
            "service",
            "distributed",
            "event sourcing",
            "cqrs",
            "graphql",
            "rest",
            "websocket",
            "serverless",
            "container",
            "kubernetes",
        ];
        if arch_triggers.iter().any(|t| q.contains(t)) {
            expanded.push_str(" YAGNI simplest monolith first incremental design strangler Sam Newman Martin Fowler");
        }

        // Database questions → profile, simplest, right tool
        let db_triggers = [
            "database",
            "sql",
            "nosql",
            "postgres",
            "mysql",
            "mongo",
            "query",
            "index",
            "schema",
            "migration",
        ];
        if db_triggers.iter().any(|t| q.contains(t)) {
            expanded.push_str(" simplest right tool profile before YAGNI data gravity");
        }

        expanded
    }

    /// Detect relevant domains from question keywords
    fn detect_domains(&self, question: &str) -> Vec<String> {
        let q = question.to_lowercase();
        let mut domains = Vec::new();

        // Product/startup keywords -> entrepreneurship
        let startup_keywords = [
            "useful", "product", "customer", "focus", "build", "launch", "market", "startup",
            "business", "revenue", "user", "feature", "mvp", "lean", "growth",
        ];
        if startup_keywords.iter().any(|k| q.contains(k)) {
            domains.push("entrepreneurship".to_string());
        }

        // Technical/architecture keywords -> software-architecture, systems
        let arch_keywords = [
            "microservices",
            "monolith",
            "architecture",
            "database",
            "api",
            "distributed",
            "migration",
            "service",
            "refactor",
            "legacy",
            "cqrs",
            "event sourcing",
            "bounded context",
            "caching",
            "cache",
            "redis",
            "memcached",
            "cdn",
            "optimize",
            "rewrite",
            "rebuild",
            "greenfield",
            "brownfield",
            "deploy",
            "kubernetes",
            "docker",
            "container",
            "serverless",
            "lambda",
            "rest",
            "graphql",
            "grpc",
            "websocket",
            "queue",
            "kafka",
            "rabbitmq",
            "postgres",
            "mysql",
            "mongodb",
            "elasticsearch",
        ];
        if arch_keywords.iter().any(|k| q.contains(k)) {
            domains.push("software-architecture".to_string());
        }

        let systems_keywords = [
            "scale",
            "performance",
            "system",
            "design",
            "complexity",
            "latency",
            "throughput",
            "bottleneck",
            "optimize",
            "fast",
            "slow",
            "load",
            "traffic",
            "concurrent",
        ];
        if systems_keywords.iter().any(|k| q.contains(k)) {
            domains.push("systems-thinking".to_string());
            domains.push("management-theory".to_string());
        }

        // AI/ML keywords
        let ai_keywords = [
            "ai",
            "machine learning",
            "model",
            "neural",
            "training",
            "inference",
            "llm",
            "gpt",
            "claude",
        ];
        if ai_keywords.iter().any(|k| q.contains(k)) {
            domains.push("ai-ml".to_string());
        }

        // Ethics/safety keywords
        let ethics_keywords = ["ethics", "safety", "risk", "harm", "bias", "fair"];
        if ethics_keywords.iter().any(|k| q.contains(k)) {
            domains.push("philosophy-ethics".to_string());
        }

        // Process/management keywords
        let mgmt_keywords = ["process", "team", "kanban", "agile", "workflow", "quality"];
        if mgmt_keywords.iter().any(|k| q.contains(k)) {
            domains.push("management-theory".to_string());
        }

        // Testing/TDD keywords -> software-practices
        let test_keywords = [
            "test",
            "tests",
            "testing",
            "tdd",
            "mock",
            "stub",
            "coverage",
            "unit",
            "integration",
            "flaky",
            "before code",
            "after code",
        ];
        if test_keywords.iter().any(|k| q.contains(k)) {
            domains.push("software-practices".to_string());
        }

        // Default: entrepreneurship (most broadly applicable)
        if domains.is_empty() {
            domains.push("entrepreneurship".to_string());
        }

        domains
    }

    /// Score a principle's relevance to the question
    fn score_principle_relevance(&self, question: &str, principle: &PrincipleMatch) -> f64 {
        let q_lower = question.to_lowercase();

        // Important keywords from question (longer words more meaningful)
        let q_words: Vec<&str> = q_lower
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| w.len() > 3) // Only meaningful words
            .collect();

        let p_lower = principle.description.to_lowercase();
        let name_lower = principle.name.to_lowercase();

        let mut score = 0.0;

        // Check each question word - use stem matching (first 4+ chars)
        for word in &q_words {
            let stem = if word.len() > 5 {
                &word[..word.len() - 2]
            } else {
                word
            };

            // Stem match in description (handles focus/focused, build/building)
            if p_lower.contains(stem) {
                score += 3.0;
            }
            // Stem match in name (highest value)
            if name_lower.contains(stem) {
                score += 5.0;
            }
        }

        // Boost high-value principle types (these are gold)
        let high_value_keywords = [
            "80/20",
            "focus",
            "lean",
            "fear",
            "compound",
            "eliminate",
            "pareto",
            "yagni",
            "simplest",
            "overengineer",
            "speculative",
        ];
        for kw in high_value_keywords {
            if name_lower.contains(kw) || p_lower.contains(kw) {
                score += 4.0; // Strong boost for known-good frameworks
            }
        }

        // Boost architecture-specific terms (Sam Newman, Fowler, etc.)
        let arch_keywords = [
            "microservices",
            "monolith",
            "database",
            "service",
            "distributed",
            "migration",
            "bounded",
            "aggregate",
            "cqrs",
            "event sourcing",
            "strangler",
            "circuit breaker",
            "failure",
            "legacy",
            "rewrite",
            "incremental",
            "deploy",
            "resilience",
            "cache",
            "caching",
            "premature",
            "optimization",
            "latency",
            "throughput",
            "scale",
            "simple",
            "simplicity",
            "complexity",
            "yagni",
            "needless",
        ];
        for kw in arch_keywords {
            if name_lower.contains(kw) {
                score += 6.0; // Very strong boost for architecture principles
            }
            if p_lower.contains(kw) {
                score += 3.0;
            }
        }

        // Performance/optimization specific boosts (Knuth, Gregg, etc.)
        let perf_keywords = [
            "premature",
            "optimization",
            "fast",
            "slow",
            "performance",
            "measure",
            "profile",
            "bottleneck",
            "efficient",
            "speed",
            "flame",
            "latency",
            "throughput",
        ];
        let question_is_perf = q_lower.contains("slow")
            || q_lower.contains("fast")
            || q_lower.contains("performance")
            || q_lower.contains("optimize");
        for kw in perf_keywords {
            if name_lower.contains(kw) {
                score += if question_is_perf { 12.0 } else { 3.0 };
            }
            if p_lower.contains(kw) {
                score += if question_is_perf { 6.0 } else { 2.0 };
            }
        }
        // Extra boost for "Profile Before Optimizing" on performance questions
        if question_is_perf && (name_lower.contains("profile") || name_lower.contains("premature"))
        {
            score += 15.0;
        }

        // Testing/TDD specific boosts (Kent Beck, Feathers, etc.)
        let test_keywords = [
            "test",
            "tdd",
            "red-green",
            "test-first",
            "mock",
            "stub",
            "coverage",
            "unit",
            "integration",
            "pyramid",
            "isolation",
        ];
        let question_mentions_test = q_lower.contains("test");
        // Detect TDD-specific questions (not just any test mention)
        let question_is_tdd =
            q_lower.contains("before") && q_lower.contains("after") && q_lower.contains("test");
        for kw in test_keywords {
            if name_lower.contains(kw) {
                score += if question_mentions_test { 10.0 } else { 2.0 };
            }
            if p_lower.contains(kw) {
                score += if question_mentions_test { 5.0 } else { 1.0 };
            }
        }
        // HUGE boost for TDD/Test-First principles on TDD questions
        if question_is_tdd
            && (name_lower.contains("tdd")
                || name_lower.contains("test-first")
                || name_lower.contains("test first")
                || name_lower.contains("red-green"))
        {
            score += 30.0; // Override other signals for explicit TDD questions
        }

        // Legacy code / tangled code specific boosts (Feathers, seams, etc.)
        let legacy_keywords = [
            "legacy",
            "seam",
            "tangled",
            "breaks",
            "brittle",
            "fragile",
            "coupling",
            "dependency",
            "working effectively",
            "characterization",
        ];
        let question_is_legacy = q_lower.contains("tangled")
            || q_lower.contains("breaks")
            || q_lower.contains("legacy")
            || q_lower.contains("old code")
            || q_lower.contains("every change");
        for kw in legacy_keywords {
            if name_lower.contains(kw) {
                score += if question_is_legacy { 15.0 } else { 2.0 };
            }
            if p_lower.contains(kw) {
                score += if question_is_legacy { 8.0 } else { 1.0 };
            }
        }
        // Michael Feathers' principles are gold for legacy code
        if question_is_legacy
            && (name_lower.contains("feathers")
                || p_lower.contains("seam")
                || name_lower.contains("legacy")
                || p_lower.contains("working effectively"))
        {
            score += 20.0;
        }

        // Refactoring/code cleanup specific boosts
        let refactor_keywords = [
            "refactor",
            "messy",
            "cleanup",
            "clean",
            "spaghetti",
            "improve",
            "incremental design",
            "technical debt",
        ];
        let question_is_refactor = q_lower.contains("refactor")
            || q_lower.contains("messy")
            || q_lower.contains("cleanup")
            || q_lower.contains("clean up")
            || q_lower.contains("before adding");
        for kw in refactor_keywords {
            if name_lower.contains(kw) {
                score += if question_is_refactor { 15.0 } else { 2.0 };
            }
            if p_lower.contains(kw) {
                score += if question_is_refactor { 8.0 } else { 1.0 };
            }
        }
        // Kent Beck and Ward Cunningham are authorities for refactoring
        if question_is_refactor
            && (name_lower.contains("incremental")
                || name_lower.contains("debt")
                || p_lower.contains("incremental")
                || p_lower.contains("tech debt")
                || p_lower.contains("technical debt"))
        {
            score += 25.0; // Strong boost for refactoring-related principles
        }

        // Match question keywords to principle name/description (exact terms)
        for word in &q_words {
            if name_lower.contains(word) || p_lower.contains(word) {
                score += 4.0; // Strong match on exact question terms
            }
        }

        // Project management / team scaling specific boosts
        let pm_keywords = [
            "late",
            "deadline",
            "team",
            "people",
            "adding",
            "hire",
            "staff",
            "communication",
            "overhead",
            "brooks",
            "mythical",
        ];
        for kw in pm_keywords {
            if name_lower.contains(kw) || p_lower.contains(kw) {
                score += 5.0;
            }
        }

        // Extra boost for matching question keywords in principle name (most relevant)
        for word in &q_words {
            if name_lower.contains(word) {
                score += 3.0; // Additional name match boost
            }
        }

        // Database/migration specific boosts
        let db_keywords = [
            "database",
            "migrate",
            "migration",
            "oracle",
            "postgres",
            "mysql",
            "nosql",
            "sql",
            "schema",
            "query",
            "data model",
        ];
        let question_mentions_db = q_lower.contains("database")
            || q_lower.contains("oracle")
            || q_lower.contains("postgres")
            || q_lower.contains("migrate");
        for kw in db_keywords {
            if name_lower.contains(kw) || p_lower.contains(kw) {
                score += if question_mentions_db { 8.0 } else { 2.0 };
            }
        }

        // Build vs buy specific boosts
        let build_buy_keywords = [
            "build",
            "buy",
            "vendor",
            "custom",
            "off-the-shelf",
            "integrate",
            "tco",
            "total cost",
            "maintenance",
            "saas",
            "third-party",
            "hosted",
            "managed",
        ];
        // "build...or use X" is the same as "build vs buy"
        let question_is_build_buy = (q_lower.contains("build")
            && (q_lower.contains("buy") || q_lower.contains("use ")))
            || q_lower.contains("vendor")
            || q_lower.contains("custom")
            || q_lower.contains("hosted")
            || q_lower.contains("managed")
            || (q_lower.contains("our own") && q_lower.contains("or "));

        // HUGE boost for principle literally named "Build vs Buy"
        if question_is_build_buy && (name_lower.contains("build") && name_lower.contains("buy")) {
            score += 50.0; // This is THE principle for this question
        }

        for kw in build_buy_keywords {
            if name_lower.contains(kw) || p_lower.contains(kw) {
                score += if question_is_build_buy { 8.0 } else { 2.0 };
            }
        }

        // Technical debt / rewrite specific boosts
        let debt_keywords = [
            "technical debt",
            "debt",
            "rewrite",
            "refactor",
            "legacy",
            "strangler",
            "incremental",
            "migration",
            "modernize",
        ];
        let question_is_debt = q_lower.contains("debt")
            || q_lower.contains("rewrite")
            || q_lower.contains("refactor")
            || q_lower.contains("legacy");
        for kw in debt_keywords {
            if name_lower.contains(kw) || p_lower.contains(kw) {
                score += if question_is_debt { 8.0 } else { 2.0 };
            }
        }

        // CONTEXTUAL THOMPSON SAMPLING BOOST
        // Query domain-specific confidence from contextual_arms table
        let detected_domain = if question_mentions_test {
            "testing"
        } else if question_is_build_buy {
            "architecture"
        } else if question_is_debt {
            "practices"
        } else if question_mentions_db {
            "architecture"
        } else {
            "entrepreneurship"
        };

        // Get contextual confidence for this principle in the detected domain
        if let Ok((alpha, beta, sample_count)) = self.conn.query_row::<(f64, f64, i64), _, _>(
            "SELECT alpha, beta, sample_count FROM contextual_arms
             WHERE principle_id = ?1 AND domain = ?2",
            rusqlite::params![&principle.id, detected_domain],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        ) {
            // Boost score by contextual confidence (0-1 range, scale to 0-15)
            let ctx_conf = alpha / (alpha + beta);
            score += ctx_conf * 15.0;

            // FEEL-GOOD THOMPSON SAMPLING (FG-TS) EXPLORATION BONUS
            // Per NeurIPS 2025 (arXiv 2507.15290): 25% improvement over vanilla TS
            // Formula: bonus = (c / sqrt(α + β)) * decay^pulls
            // This solves the orphan/cold-start problem by giving undersampled principles a chance
            let n = alpha + beta;
            let pulls = sample_count as u32;
            let optimism_constant = 3.0; // Increased from 2.0 for stronger exploration
            let bonus_decay: f64 = 0.98; // Slower decay (was 0.95) to maintain exploration longer
            let raw_bonus = optimism_constant / n.sqrt();
            let decayed_bonus = raw_bonus * bonus_decay.powi(pulls as i32);
            let fg_ts_bonus = decayed_bonus.min(1.0); // Cap at 1.0 (was 0.5)

            // Apply FG-TS bonus scaled to score range - STRONGER exploration
            score += fg_ts_bonus * 15.0; // Scale 0-1.0 to 0-15 points (was 0-5)
        } else {
            // COLD ARM: No data for this principle in this domain
            // Give maximum exploration bonus to discover effectiveness
            let cold_arm_bonus = 15.0; // Increased from 5.0 for aggressive orphan exploration
            score += cold_arm_bonus;
        }

        score
    }

    /// Generate positions from different stances - ensuring thinker diversity AND relevance
    /// Uses epsilon-greedy exploration to discover underutilized principles
    fn generate_positions(
        &self,
        request: &CounselRequest,
        principles: &[PrincipleMatch],
    ) -> Result<Vec<CounselPosition>> {
        use rand::Rng;
        let mut rng = rand::thread_rng();

        let mut positions = Vec::new();
        let target_count = match request.context.depth {
            CounselDepth::Quick => 3,
            CounselDepth::Standard => 4,
            CounselDepth::Deep => 6,
        };

        // Score and sort principles by relevance to question
        let mut scored: Vec<_> = principles.iter()
            .map(|p| {
                let mut score = self.score_principle_relevance(&request.question, p);

                // CAP maximum score to prevent keyword dominance (Knuth optimization problem)
                score = score.min(80.0);  // Reduced from 100 to compress score range

                // DIVERSITY PENALTY: Reduce score for frequently-cited principles
                // This prevents the same principles from always winning
                // Query the contextual_arms table for citation frequency
                if let Ok(sample_count) = self.conn.query_row::<i64, _, _>(
                    "SELECT COALESCE(SUM(sample_count), 0) FROM contextual_arms WHERE principle_id = ?1",
                    rusqlite::params![&p.id],
                    |row| row.get(0),
                ) {
                    // Apply diminishing returns penalty for over-selected principles
                    // Formula: penalty = log(1 + citations) * 3 (increased from 2)
                    if sample_count > 30 {  // Lower threshold
                        let penalty = (1.0 + sample_count as f64).ln() * 3.0;
                        score -= penalty.min(20.0);  // Increased cap from 15
                    }
                }

                // SOFTMAX TEMPERATURE: Add random noise to break ties and increase diversity
                // This is like temperature in softmax - higher noise = more exploration
                let noise = rng.gen::<f64>() * 15.0;  // 0-15 random points
                score += noise;

                (p, score.max(0.0))  // Ensure score doesn't go negative
            })
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        // Check if we have ANY highly relevant principles (score > 5.0 = direct match)
        let top_score = scored.first().map(|(_, s)| *s).unwrap_or(0.0);
        let has_strong_match = top_score >= 5.0;

        // We need at least one FOR, one AGAINST, one SYNTHESIZE
        let stances = [
            Stance::For,
            Stance::Against,
            Stance::Synthesize,
            Stance::Against,
        ];

        // Track which thinkers we've used to ensure diversity
        let mut used_thinkers: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut used_principles: std::collections::HashSet<String> =
            std::collections::HashSet::new();

        // EPSILON-GREEDY EXPLORATION: 20% chance to explore lower-ranked principles
        let epsilon = 0.20;

        let mut principle_idx = 0;
        for stance in stances.iter().take(target_count) {
            // Epsilon-greedy: occasionally pick from the "tail" (positions 5-20) instead of top
            let explore_mode = rng.gen::<f64>() < epsilon && scored.len() > 10;

            if explore_mode {
                // Sample from principles ranked 5-20 (exploration zone)
                let explore_start = 5.min(scored.len().saturating_sub(1));
                let explore_end = 20.min(scored.len());
                let explore_range = explore_start..explore_end;

                if explore_range.len() > 0 {
                    for _ in 0..10 {
                        // Try up to 10 times to find unused thinker
                        let idx = rng.gen_range(explore_range.clone());
                        let (principle, score) = &scored[idx];

                        if used_thinkers.contains(&principle.thinker_id)
                            || used_principles.contains(&principle.id)
                        {
                            continue;
                        }

                        // Still require minimum relevance for exploration
                        if *score < 2.0 {
                            continue;
                        }

                        used_thinkers.insert(principle.thinker_id.clone());
                        used_principles.insert(principle.id.clone());
                        let position = self.build_position(request, principle, *stance)?;
                        positions.push(position);
                        break;
                    }
                    if positions.len() > positions.len().saturating_sub(1) {
                        continue; // Successfully added via exploration
                    }
                }
            }

            // Standard greedy selection: find next principle from a different thinker
            while principle_idx < scored.len() {
                let (principle, score) = &scored[principle_idx];
                principle_idx += 1;

                // Skip if we already used this thinker or principle
                if used_thinkers.contains(&principle.thinker_id)
                    || used_principles.contains(&principle.id)
                {
                    continue;
                }

                // If no strong matches, be stricter about what we include
                if !has_strong_match && *score < 3.0 {
                    continue; // Skip weak matches when nothing is strongly relevant
                }

                // Skip very low relevance principles
                if *score < 1.0 && principle_idx > 2 {
                    continue;
                }

                used_thinkers.insert(principle.thinker_id.clone());
                used_principles.insert(principle.id.clone());
                let position = self.build_position(request, principle, *stance)?;
                positions.push(position);
                break;
            }
        }

        // If we couldn't find enough relevant positions, add meta-guidance
        if positions.is_empty() {
            positions.push(CounselPosition {
                thinker: "100minds".to_string(),
                thinker_id: "_meta".to_string(),
                stance: Stance::Synthesize,
                argument: format!(
                    "No highly relevant frameworks found for this specific question. \
                    Consider: (1) Break the question into smaller parts, \
                    (2) Identify the core trade-off, (3) Look for analogies in other domains. \
                    Top score was {:.1} - try more specific terms.",
                    top_score
                ),
                principles_cited: vec!["Meta-reasoning".to_string()],
                confidence: 0.3,
                falsifiable_if: None,
            });
        }

        Ok(positions)
    }

    /// Build a single counsel position
    fn build_position(
        &self,
        request: &CounselRequest,
        principle: &PrincipleMatch,
        stance: Stance,
    ) -> Result<CounselPosition> {
        // Get thinker name
        let thinker_name = self.get_thinker_name(&principle.thinker_id)?;

        // Build argument based on stance and principle
        let argument = self.build_argument(request, principle, stance);

        // Determine falsification condition
        let falsifiable_if = self.build_falsification(request, principle, stance);

        Ok(CounselPosition {
            thinker: thinker_name,
            thinker_id: principle.thinker_id.clone(),
            stance,
            argument,
            principles_cited: vec![principle.id.clone()], // Use ID for outcome recording
            confidence: principle.confidence,
            falsifiable_if: Some(falsifiable_if),
        })
    }

    /// Build the argument text - principle + Socratic question for application
    fn build_argument(
        &self,
        _request: &CounselRequest,
        principle: &PrincipleMatch,
        stance: Stance,
    ) -> String {
        let principle_text = &principle.description;

        // Generate a Socratic question to help apply the principle
        let socratic = self.generate_socratic_question(&principle.name, principle_text);

        match stance {
            Stance::For => {
                format!("{}\n   → {}", principle_text, socratic)
            }
            Stance::Against => {
                format!("{}\n   → {}", principle_text, socratic)
            }
            Stance::Synthesize => {
                format!("{}\n   → {}", principle_text, socratic)
            }
            Stance::Challenge => {
                format!("{}\n   → {}", principle_text, socratic)
            }
        }
    }

    /// Generate an ACTION prompt to immediately apply the principle
    fn generate_socratic_question(&self, name: &str, description: &str) -> String {
        let name_lower = name.to_lowercase();
        let desc_lower = description.to_lowercase();

        // ACTION prompts - specific, immediate, doable in 60 seconds
        if name_lower.contains("80/20")
            || desc_lower.contains("80/20")
            || desc_lower.contains("high-impact")
        {
            return "ACTION: List 5 things you're working on. Circle the ONE that matters most. Do only that.".to_string();
        }
        if name_lower.contains("fear") || desc_lower.contains("fear") {
            return "ACTION: Write the worst case in one sentence. Then write how you'd recover. Now decide.".to_string();
        }
        if name_lower.contains("focus")
            || desc_lower.contains("focus")
            || desc_lower.contains("distraction")
        {
            return "ACTION: Name ONE thing to stop doing today. Block it. Protect your focus."
                .to_string();
        }
        if name_lower.contains("compound") || desc_lower.contains("compound") {
            return "ACTION: What takes 5 minutes today that pays off in 6 months? Do it now."
                .to_string();
        }
        if desc_lower.contains("eliminate")
            || desc_lower.contains("remove")
            || desc_lower.contains("cut")
        {
            return "ACTION: Delete one feature/task/commitment right now. What won't you miss?"
                .to_string();
        }
        if desc_lower.contains("customer") || desc_lower.contains("user") {
            return "ACTION: Message ONE user right now. Ask: 'What's frustrating you?'"
                .to_string();
        }
        if desc_lower.contains("track") || desc_lower.contains("measure") {
            return "ACTION: Pick ONE number that proves success. Write it down. Check it daily."
                .to_string();
        }
        if desc_lower.contains("automat") {
            return "ACTION: What did you do manually 3+ times this week? Automate it today."
                .to_string();
        }
        if desc_lower.contains("quality") || desc_lower.contains("defect") {
            return "ACTION: Find your last 3 bugs. What's the common cause? Fix that root."
                .to_string();
        }
        if desc_lower.contains("simple") || desc_lower.contains("complex") {
            return "ACTION: Describe your solution in one sentence. If you can't, simplify."
                .to_string();
        }
        if desc_lower.contains("start")
            || desc_lower.contains("begin")
            || desc_lower.contains("now")
        {
            return "ACTION: What's the smallest thing you can ship TODAY? Do that.".to_string();
        }
        if desc_lower.contains("jit")
            || desc_lower.contains("just-in-time")
            || desc_lower.contains("needed")
        {
            return "ACTION: What are you building that nobody asked for yet? Stop. Wait for pull."
                .to_string();
        }

        // Default: still actionable
        "ACTION: Apply this in the next 60 seconds. What's ONE concrete step?".to_string()
    }

    /// Build falsification condition (Popper's principle)
    fn build_falsification(
        &self,
        _request: &CounselRequest,
        principle: &PrincipleMatch,
        stance: Stance,
    ) -> String {
        match stance {
            Stance::For => {
                format!(
                    "This recommendation is wrong if the {} principle doesn't apply to this context",
                    principle.name
                )
            }
            Stance::Against => {
                format!(
                    "This caution is unnecessary if you've already validated against {}",
                    principle.name
                )
            }
            Stance::Synthesize => {
                "This synthesis fails if the trade-offs don't actually balance".to_string()
            }
            Stance::Challenge => {
                "This challenge is invalid if you have direct evidence addressing it".to_string()
            }
        }
    }

    /// Generate devil's advocate challenge
    fn generate_challenge(
        &self,
        request: &CounselRequest,
        positions: &[CounselPosition],
    ) -> Result<CounselPosition> {
        // Find what's missing from the positions
        let missing_considerations = self.find_missing_considerations(request, positions);

        let argument = if missing_considerations.is_empty() {
            format!(
                "The positions above assume your question is well-formed. \
                Have you considered: What problem are you actually solving? \
                What would 'success' look like? Who else should you consult?"
            )
        } else {
            format!(
                "Missing considerations: {}. \
                The positions above may be incomplete without addressing these.",
                missing_considerations.join(", ")
            )
        };

        Ok(CounselPosition {
            thinker: "Devil's Advocate".to_string(),
            thinker_id: "_challenge".to_string(),
            stance: Stance::Challenge,
            argument,
            principles_cited: vec!["Socratic Method".to_string()],
            confidence: 0.95, // Challenges are high confidence - always worth considering
            falsifiable_if: Some(
                "This challenge is invalid if you have direct evidence addressing it".to_string(),
            ),
        })
    }

    /// Find what considerations might be missing
    fn find_missing_considerations(
        &self,
        request: &CounselRequest,
        _positions: &[CounselPosition],
    ) -> Vec<String> {
        let mut missing = Vec::new();

        // Check for common missing elements
        let question_lower = request.question.to_lowercase();

        if !question_lower.contains("team") && !question_lower.contains("people") {
            missing.push("team capacity and expertise".to_string());
        }

        if !question_lower.contains("time") && !question_lower.contains("deadline") {
            missing.push("timeline constraints".to_string());
        }

        if !question_lower.contains("cost") && !question_lower.contains("budget") {
            missing.push("resource/budget implications".to_string());
        }

        if !question_lower.contains("risk") && !question_lower.contains("fail") {
            missing.push("failure scenarios and rollback plans".to_string());
        }

        if request.context.constraints.is_empty() {
            missing.push("explicit constraints".to_string());
        }

        missing
    }

    /// Get thinker name by ID
    fn get_thinker_name(&self, thinker_id: &str) -> Result<String> {
        let name: String = self
            .conn
            .query_row(
                "SELECT name FROM thinkers WHERE id = ?1",
                [thinker_id],
                |row| row.get(0),
            )
            .unwrap_or_else(|_| thinker_id.to_string());

        Ok(name)
    }

    /// Create provenance info for this counsel
    fn create_provenance(
        &self,
        request: &CounselRequest,
        positions: &[CounselPosition],
        challenge: &CounselPosition,
    ) -> Result<ProvenanceInfo> {
        // Get previous hash for chain
        let previous_hash = db::get_latest_decision_hash(self.conn)?;

        // Create content to hash
        let content = serde_json::json!({
            "question": request.question,
            "context": request.context,
            "positions": positions,
            "challenge": challenge,
        });

        let content_bytes = serde_json::to_vec(&content)?;

        // Hash and sign
        let content_hash = self.provenance.hash(&content_bytes);
        let signature = self.provenance.sign(&content_bytes)?;
        let pubkey = self.provenance.public_key_hex();

        Ok(ProvenanceInfo {
            content_hash,
            previous_hash,
            signature,
            agent_pubkey: pubkey,
        })
    }

    /// Store the decision in database
    fn store_decision(&self, response: &CounselResponse, request: &CounselRequest) -> Result<()> {
        let context_json = serde_json::to_string(&request.context)?;
        let counsel_json = serde_json::to_string(&response)?;

        db::insert_decision(
            self.conn,
            &response.decision_id,
            &request.question,
            Some(&context_json),
            &counsel_json,
            response.provenance.previous_hash.as_deref(),
            &response.provenance.content_hash,
            &response.provenance.signature,
            &response.provenance.agent_pubkey,
        )?;

        Ok(())
    }

    /// Record outcome and apply learning adjustments
    pub fn record_outcome(&self, request: &RecordOutcomeRequest) -> Result<()> {
        // 1. Update the decision record
        db::record_outcome(
            self.conn,
            &request.decision_id,
            request.success,
            request.notes.as_deref(),
        )?;

        // 2. Get the decision to find which principles were used
        let counsel_json: String = self.conn.query_row(
            "SELECT counsel_json FROM decisions WHERE id = ?1",
            [&request.decision_id],
            |row| row.get(0),
        )?;

        let counsel: CounselResponse = serde_json::from_str(&counsel_json)?;

        // 3. Apply adjustments based on outcome
        let adjustment = if request.success { 0.05 } else { -0.08 };

        for position in &counsel.positions {
            for principle_name in &position.principles_cited {
                // Find principle ID
                if let Ok(principle_id) = self.conn.query_row::<String, _, _>(
                    "SELECT id FROM principles WHERE name = ?1",
                    [principle_name],
                    |row| row.get(0),
                ) {
                    let _ = db::apply_adjustment(
                        self.conn,
                        &principle_id,
                        None, // TODO: extract context pattern
                        adjustment,
                        &request.decision_id,
                    );
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::provenance::Provenance;
    use rusqlite::Connection;
    use tempfile::tempdir;

    fn setup_test_db() -> (Connection, tempfile::TempDir) {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let conn = db::init_db(&db_path).unwrap();
        (conn, dir)
    }

    fn setup_provenance() -> (Provenance, tempfile::TempDir) {
        let dir = tempdir().unwrap();
        let key_path = dir.path().join("test.key");
        let provenance = Provenance::init(&key_path).unwrap();
        (provenance, dir)
    }

    // =========================================================================
    // detect_domains tests
    // =========================================================================

    #[test]
    fn test_detect_domains_entrepreneurship() {
        let (conn, _db_dir) = setup_test_db();
        let (provenance, _dir) = setup_provenance();
        let engine = CounselEngine::new(&conn, &provenance);

        let domains = engine.detect_domains("How do we build a product customers will buy?");
        assert!(domains.contains(&"entrepreneurship".to_string()));
    }

    #[test]
    fn test_detect_domains_architecture() {
        let (conn, _db_dir) = setup_test_db();
        let (provenance, _dir) = setup_provenance();
        let engine = CounselEngine::new(&conn, &provenance);

        let domains = engine.detect_domains("Should we use microservices or a monolith?");
        assert!(domains.contains(&"software-architecture".to_string()));
    }

    #[test]
    fn test_detect_domains_systems() {
        let (conn, _db_dir) = setup_test_db();
        let (provenance, _dir) = setup_provenance();
        let engine = CounselEngine::new(&conn, &provenance);

        let domains = engine.detect_domains("How do we scale the system for more traffic?");
        assert!(domains.contains(&"systems-thinking".to_string()));
    }

    #[test]
    fn test_detect_domains_ai() {
        let (conn, _db_dir) = setup_test_db();
        let (provenance, _dir) = setup_provenance();
        let engine = CounselEngine::new(&conn, &provenance);

        let domains = engine.detect_domains("Should we train our own AI model or use GPT?");
        assert!(domains.contains(&"ai-ml".to_string()));
    }

    #[test]
    fn test_detect_domains_testing() {
        let (conn, _db_dir) = setup_test_db();
        let (provenance, _dir) = setup_provenance();
        let engine = CounselEngine::new(&conn, &provenance);

        let domains = engine.detect_domains("Should we write tests before or after code?");
        assert!(domains.contains(&"software-practices".to_string()));
    }

    #[test]
    fn test_detect_domains_default() {
        let (conn, _db_dir) = setup_test_db();
        let (provenance, _dir) = setup_provenance();
        let engine = CounselEngine::new(&conn, &provenance);

        // Random question should default to entrepreneurship
        let domains = engine.detect_domains("What should I have for lunch?");
        assert!(domains.contains(&"entrepreneurship".to_string()));
    }

    // =========================================================================
    // expand_query_keywords tests
    // =========================================================================

    #[test]
    fn test_expand_query_perf_keywords() {
        let (conn, _db_dir) = setup_test_db();
        let (provenance, _dir) = setup_provenance();
        let engine = CounselEngine::new(&conn, &provenance);

        let expanded = engine.expand_query_keywords("Should we add caching?");
        assert!(expanded.contains("YAGNI"), "Should expand with YAGNI");
        assert!(
            expanded.contains("premature"),
            "Should expand with premature optimization"
        );
    }

    #[test]
    fn test_expand_query_rewrite_keywords() {
        let (conn, _db_dir) = setup_test_db();
        let (provenance, _dir) = setup_provenance();
        let engine = CounselEngine::new(&conn, &provenance);

        let expanded = engine.expand_query_keywords("Should we rewrite the legacy system?");
        assert!(
            expanded.contains("strangler"),
            "Should expand with strangler"
        );
        assert!(
            expanded.contains("incremental"),
            "Should expand with incremental"
        );
    }

    #[test]
    fn test_expand_query_team_keywords() {
        let (conn, _db_dir) = setup_test_db();
        let (provenance, _dir) = setup_provenance();
        let engine = CounselEngine::new(&conn, &provenance);

        let expanded = engine.expand_query_keywords("Should we hire more developers?");
        assert!(expanded.contains("Brooks"), "Should expand with Brooks Law");
    }

    #[test]
    fn test_expand_query_test_keywords() {
        let (conn, _db_dir) = setup_test_db();
        let (provenance, _dir) = setup_provenance();
        let engine = CounselEngine::new(&conn, &provenance);

        let expanded = engine.expand_query_keywords("Should we add more tests?");
        assert!(expanded.contains("TDD"), "Should expand with TDD");
        assert!(
            expanded.contains("Kent Beck"),
            "Should expand with Kent Beck"
        );
    }

    #[test]
    fn test_expand_query_no_expansion() {
        let (conn, _db_dir) = setup_test_db();
        let (provenance, _dir) = setup_provenance();
        let engine = CounselEngine::new(&conn, &provenance);

        let original = "What color should the button be?";
        let expanded = engine.expand_query_keywords(original);
        assert_eq!(expanded, original, "Non-matching query should not expand");
    }

    // =========================================================================
    // generate_socratic_question tests
    // =========================================================================

    #[test]
    fn test_socratic_80_20() {
        let (conn, _db_dir) = setup_test_db();
        let (provenance, _dir) = setup_provenance();
        let engine = CounselEngine::new(&conn, &provenance);

        let question = engine.generate_socratic_question("80/20 Rule", "Focus on high-impact");
        assert!(question.contains("ACTION"), "Should be actionable");
        assert!(question.contains("ONE"), "Should focus on single action");
    }

    #[test]
    fn test_socratic_fear() {
        let (conn, _db_dir) = setup_test_db();
        let (provenance, _dir) = setup_provenance();
        let engine = CounselEngine::new(&conn, &provenance);

        let question = engine.generate_socratic_question("Fear Setting", "Confront your fears");
        assert!(question.contains("worst case"), "Should address worst case");
        assert!(question.contains("recover"), "Should include recovery");
    }

    #[test]
    fn test_socratic_focus() {
        let (conn, _db_dir) = setup_test_db();
        let (provenance, _dir) = setup_provenance();
        let engine = CounselEngine::new(&conn, &provenance);

        let question = engine.generate_socratic_question("Deep Focus", "Eliminate distraction");
        assert!(question.contains("stop doing"), "Should prompt elimination");
    }

    #[test]
    fn test_socratic_default() {
        let (conn, _db_dir) = setup_test_db();
        let (provenance, _dir) = setup_provenance();
        let engine = CounselEngine::new(&conn, &provenance);

        let question = engine.generate_socratic_question("Unknown", "Some description");
        assert!(question.contains("ACTION"), "Default should be actionable");
        assert!(
            question.contains("60 seconds"),
            "Should prompt immediate action"
        );
    }

    // =========================================================================
    // find_missing_considerations tests
    // =========================================================================

    #[test]
    fn test_missing_considerations_no_team() {
        let (conn, _db_dir) = setup_test_db();
        let (provenance, _dir) = setup_provenance();
        let engine = CounselEngine::new(&conn, &provenance);

        let request = CounselRequest {
            question: "Should we use microservices?".to_string(),
            context: CounselContext::default(),
        };
        let positions: Vec<CounselPosition> = vec![];

        let missing = engine.find_missing_considerations(&request, &positions);
        assert!(
            missing.iter().any(|m| m.contains("team")),
            "Should flag missing team consideration"
        );
    }

    #[test]
    fn test_missing_considerations_no_time() {
        let (conn, _db_dir) = setup_test_db();
        let (provenance, _dir) = setup_provenance();
        let engine = CounselEngine::new(&conn, &provenance);

        let request = CounselRequest {
            question: "Should we refactor the codebase?".to_string(),
            context: CounselContext::default(),
        };
        let positions: Vec<CounselPosition> = vec![];

        let missing = engine.find_missing_considerations(&request, &positions);
        assert!(
            missing.iter().any(|m| m.contains("timeline")),
            "Should flag missing timeline"
        );
    }

    #[test]
    fn test_missing_considerations_has_context() {
        let (conn, _db_dir) = setup_test_db();
        let (provenance, _dir) = setup_provenance();
        let engine = CounselEngine::new(&conn, &provenance);

        let request = CounselRequest {
            question: "Should we hire more people to meet the deadline with cost constraints?"
                .to_string(),
            context: CounselContext {
                constraints: vec!["Budget: $100k".to_string()],
                ..Default::default()
            },
        };
        let positions: Vec<CounselPosition> = vec![];

        let missing = engine.find_missing_considerations(&request, &positions);
        // Should NOT flag team, time, or cost since they're mentioned
        assert!(!missing.iter().any(|m| m.contains("timeline")));
        assert!(!missing.iter().any(|m| m.contains("team")));
        assert!(!missing.iter().any(|m| m.contains("resource")));
        // But explicit constraints should not be flagged
        assert!(!missing.iter().any(|m| m.contains("explicit constraints")));
    }

    // =========================================================================
    // detect_urgency tests (using mock positions)
    // =========================================================================

    fn mock_position_with_confidence(stance: Stance, confidence: f64) -> CounselPosition {
        CounselPosition {
            thinker: "Test".to_string(),
            thinker_id: "test".to_string(),
            stance,
            argument: "Test argument".to_string(),
            principles_cited: vec![],
            confidence,
            falsifiable_if: None,
        }
    }

    #[test]
    fn test_detect_urgency_security_escalates() {
        let (conn, _db_dir) = setup_test_db();
        let (provenance, _dir) = setup_provenance();
        let engine = CounselEngine::new(&conn, &provenance);

        // Need 2+ escalate keywords OR low confidence to trigger escalation
        let request = CounselRequest {
            question: "There's a critical security vulnerability in production".to_string(),
            context: CounselContext::default(),
        };
        let positions = vec![mock_position_with_confidence(Stance::For, 0.8)];

        let urgency = engine.detect_urgency(&request, &positions);
        assert_eq!(urgency, Some("escalate".to_string()));
    }

    #[test]
    fn test_detect_urgency_future_defers() {
        let (conn, _db_dir) = setup_test_db();
        let (provenance, _dir) = setup_provenance();
        let engine = CounselEngine::new(&conn, &provenance);

        let request = CounselRequest {
            question: "Eventually we should maybe consider exploring this for phase 2".to_string(),
            context: CounselContext::default(),
        };
        let positions = vec![mock_position_with_confidence(Stance::For, 0.8)];

        let urgency = engine.detect_urgency(&request, &positions);
        assert_eq!(urgency, Some("defer".to_string()));
    }

    #[test]
    fn test_detect_urgency_low_confidence_escalates() {
        let (conn, _db_dir) = setup_test_db();
        let (provenance, _dir) = setup_provenance();
        let engine = CounselEngine::new(&conn, &provenance);

        let request = CounselRequest {
            question: "Is there a security issue here?".to_string(),
            context: CounselContext::default(),
        };
        // Low confidence positions on a security question
        let positions = vec![mock_position_with_confidence(Stance::For, 0.3)];

        let urgency = engine.detect_urgency(&request, &positions);
        assert_eq!(urgency, Some("escalate".to_string()));
    }

    #[test]
    fn test_detect_urgency_contentious_escalates() {
        let (conn, _db_dir) = setup_test_db();
        let (provenance, _dir) = setup_provenance();
        let engine = CounselEngine::new(&conn, &provenance);

        let request = CounselRequest {
            question: "Should we use approach A or B?".to_string(),
            context: CounselContext::default(),
        };
        // Similar confidence FOR and AGAINST = contentious
        let positions = vec![
            mock_position_with_confidence(Stance::For, 0.8),
            mock_position_with_confidence(Stance::Against, 0.75),
        ];

        let urgency = engine.detect_urgency(&request, &positions);
        assert_eq!(urgency, Some("escalate".to_string()));
    }

    #[test]
    fn test_detect_urgency_normal_returns_none() {
        let (conn, _db_dir) = setup_test_db();
        let (provenance, _dir) = setup_provenance();
        let engine = CounselEngine::new(&conn, &provenance);

        let request = CounselRequest {
            question: "What naming convention should we use?".to_string(),
            context: CounselContext::default(),
        };
        let positions = vec![mock_position_with_confidence(Stance::For, 0.8)];

        let urgency = engine.detect_urgency(&request, &positions);
        assert_eq!(urgency, None);
    }

    // =========================================================================
    // build_falsification tests
    // =========================================================================

    #[test]
    fn test_build_falsification_for_stance() {
        let (conn, _db_dir) = setup_test_db();
        let (provenance, _dir) = setup_provenance();
        let engine = CounselEngine::new(&conn, &provenance);

        let request = CounselRequest {
            question: "Test".to_string(),
            context: CounselContext::default(),
        };
        let principle = db::PrincipleMatch {
            id: "test".to_string(),
            thinker_id: "thinker".to_string(),
            name: "YAGNI".to_string(),
            description: "You Ain't Gonna Need It".to_string(),
            confidence: 0.8,
            relevance_score: 1.0,
        };

        let falsification = engine.build_falsification(&request, &principle, Stance::For);
        assert!(
            falsification.contains("wrong"),
            "FOR stance should mention being wrong"
        );
        assert!(
            falsification.contains("YAGNI"),
            "Should reference principle name"
        );
    }

    #[test]
    fn test_build_falsification_against_stance() {
        let (conn, _db_dir) = setup_test_db();
        let (provenance, _dir) = setup_provenance();
        let engine = CounselEngine::new(&conn, &provenance);

        let request = CounselRequest {
            question: "Test".to_string(),
            context: CounselContext::default(),
        };
        let principle = db::PrincipleMatch {
            id: "test".to_string(),
            thinker_id: "thinker".to_string(),
            name: "Brooks Law".to_string(),
            description: "Adding people late makes projects later".to_string(),
            confidence: 0.8,
            relevance_score: 1.0,
        };

        let falsification = engine.build_falsification(&request, &principle, Stance::Against);
        assert!(
            falsification.contains("unnecessary"),
            "AGAINST stance should mention being unnecessary"
        );
    }
}
