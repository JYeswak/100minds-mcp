//! Synthetic Question Generation for 100k-1M Scale Evaluation
//!
//! Uses LLM-powered generation with domain-specific templates to create
//! exhaustive test coverage across the entire decision parameter space.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Helper function to build variables HashMap from static data
fn build_variables(pairs: &[(&str, &[&str])]) -> HashMap<String, Vec<String>> {
    pairs.iter()
        .map(|(k, v)| (k.to_string(), v.iter().map(|s| s.to_string()).collect()))
        .collect()
}

/// Domain configuration for question generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainConfig {
    pub name: String,
    pub templates: Vec<String>,
    pub variables: HashMap<String, Vec<String>>,
    pub difficulty_modifiers: Vec<String>,
    pub stakeholder_variants: Vec<String>,
}

/// A synthetically generated question with full context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyntheticQuestion {
    pub id: String,
    pub question: String,
    pub domain: String,
    pub difficulty: u8,
    pub stakeholder: String,
    pub company_stage: String,
    pub urgency: String,
    pub template_id: String,
    pub variables_used: HashMap<String, String>,
}

/// Configuration for the synthetic question generator
#[derive(Debug, Clone)]
pub struct GeneratorConfig {
    pub domains: Vec<DomainConfig>,
    pub stakeholders: Vec<String>,
    pub company_stages: Vec<String>,
    pub urgency_levels: Vec<String>,
    pub target_count: usize,
}

impl Default for GeneratorConfig {
    fn default() -> Self {
        Self {
            domains: default_domains(),
            stakeholders: vec![
                "CTO".into(),
                "Tech Lead".into(),
                "Senior Engineer".into(),
                "Engineering Manager".into(),
                "Founder".into(),
                "Product Manager".into(),
                "DevOps Engineer".into(),
            ],
            company_stages: vec![
                "early-startup".into(),
                "growth-stage".into(),
                "scale-up".into(),
                "enterprise".into(),
                "mature-company".into(),
            ],
            urgency_levels: vec![
                "exploration".into(),
                "planning".into(),
                "implementation".into(),
                "crisis".into(),
                "post-mortem".into(),
            ],
            target_count: 100_000,
        }
    }
}

/// Generate all domain configurations with rich templates
fn default_domains() -> Vec<DomainConfig> {
    vec![
        DomainConfig {
            name: "architecture".into(),
            templates: vec![
                "Should we {action} our {system_type}?".into(),
                "Is it time to {migrate_verb} from {tech_old} to {tech_new}?".into(),
                "Our {component} is {problem}. What should we do?".into(),
                "Should we adopt {pattern} for our {use_case}?".into(),
                "We're considering {architectural_decision}. Is this wise?".into(),
                "How should we {decomposition_action} our {monolith_component}?".into(),
                "Should we introduce {middleware_type} between {system_a} and {system_b}?".into(),
            ],
            variables: build_variables(&[
                ("action", &["rewrite", "refactor", "decompose", "consolidate", "modernize", "migrate"]),
                ("system_type", &["monolith", "API", "backend", "frontend", "data pipeline", "auth system"]),
                ("migrate_verb", &["migrate", "move", "transition", "switch"]),
                ("tech_old", &["Django", "Rails", "Spring", "PHP", "jQuery", "Angular 1.x", "MySQL"]),
                ("tech_new", &["FastAPI", "Next.js", "Go", "Rust", "React", "Vue", "PostgreSQL"]),
                ("component", &["database layer", "API gateway", "auth service", "notification system"]),
                ("problem", &["too slow", "unmaintainable", "doesn't scale", "has security issues"]),
                ("pattern", &["microservices", "event sourcing", "CQRS", "hexagonal architecture"]),
                ("use_case", &["real-time features", "multi-tenant SaaS", "high-write workloads"]),
                ("architectural_decision", &["breaking up our monolith", "adding a message queue", "going serverless"]),
                ("decomposition_action", &["split", "extract", "separate", "isolate"]),
                ("monolith_component", &["user service", "payment processing", "reporting"]),
                ("middleware_type", &["a message queue", "an API gateway", "a cache layer"]),
                ("system_a", &["our API", "the frontend", "the mobile app"]),
                ("system_b", &["the database", "third-party services", "the backend"]),
            ]),
            difficulty_modifiers: vec![
                "with a tight deadline".into(),
                "on a limited budget".into(),
                "with regulatory constraints".into(),
                "while maintaining backwards compatibility".into(),
            ],
            stakeholder_variants: vec![
                "Engineering wants X but PM says no".into(),
                "The board is pushing for speed".into(),
                "We have a customer deadline".into(),
            ],
        },
        DomainConfig {
            name: "scaling".into(),
            templates: vec![
                "We need to handle {multiplier}x our current traffic. How?".into(),
                "Should we scale {scale_direction}?".into(),
                "Is {scaling_pattern} right for our {workload_type}?".into(),
                "Our {bottleneck} can't keep up with demand. Options?".into(),
                "We're hitting {limit_type} limits. What's the path forward?".into(),
                "Should we add more {resource_type} or optimize existing ones?".into(),
                "Can we handle {event_type} without melting our servers?".into(),
            ],
            variables: build_variables(&[
                ("multiplier", &["10", "100", "1000", "10x", "100x"]),
                ("scale_direction", &["horizontally", "vertically", "out", "up"]),
                ("scaling_pattern", &["auto-scaling", "sharding", "read replicas", "CDN"]),
                ("workload_type", &["read-heavy", "write-heavy", "mixed", "bursty"]),
                ("bottleneck", &["database", "API servers", "message queue", "storage"]),
                ("limit_type", &["CPU", "memory", "disk I/O", "network", "connection pool"]),
                ("resource_type", &["servers", "database nodes", "cache instances"]),
                ("event_type", &["Black Friday", "a viral post", "product launch"]),
            ]),
            difficulty_modifiers: vec!["without downtime".into(), "with zero budget increase".into(), "overnight".into()],
            stakeholder_variants: vec![],
        },
        DomainConfig {
            name: "testing".into(),
            templates: vec![
                "Should we write tests {timing} code?".into(),
                "Our test suite takes {duration} to run. How do we fix this?".into(),
                "Should we adopt {testing_approach} for our {test_target}?".into(),
                "How much test coverage is enough for {risk_level} systems?".into(),
                "Our tests are {test_problem}. What's the solution?".into(),
                "Should we {test_action} our {test_type} tests?".into(),
                "Is {test_philosophy} worth the investment?".into(),
            ],
            variables: build_variables(&[
                ("timing", &["before", "after", "during", "alongside"]),
                ("duration", &["2 hours", "30 minutes", "all night", "too long"]),
                ("testing_approach", &["TDD", "BDD", "property-based testing", "mutation testing"]),
                ("test_target", &["API", "frontend", "integration points", "business logic"]),
                ("risk_level", &["mission-critical", "financial", "healthcare", "normal"]),
                ("test_problem", &["flaky", "slow", "not catching bugs", "hard to maintain"]),
                ("test_action", &["delete", "rewrite", "parallelize", "mock out"]),
                ("test_type", &["unit", "integration", "e2e", "performance"]),
                ("test_philosophy", &["100% coverage", "TDD", "testing in production"]),
            ]),
            difficulty_modifiers: vec!["for a legacy codebase".into(), "with limited CI resources".into()],
            stakeholder_variants: vec![],
        },
        DomainConfig {
            name: "management".into(),
            templates: vec![
                "Should we add {count} engineers to our {timeline} project?".into(),
                "Is it time to split our team into {structure}?".into(),
                "How do we handle {team_problem}?".into(),
                "Should we {hiring_action} for this {skill_type} work?".into(),
                "Our {process} isn't working. What should change?".into(),
                "Is {management_approach} right for our team size?".into(),
            ],
            variables: build_variables(&[
                ("count", &["2", "5", "10", "more"]),
                ("timeline", &["late", "behind-schedule", "struggling", "on-fire"]),
                ("structure", &["smaller squads", "feature teams", "platform + product"]),
                ("team_problem", &["low morale", "constant firefighting", "knowledge silos"]),
                ("hiring_action", &["hire contractors", "build in-house expertise", "outsource"]),
                ("skill_type", &["ML", "infrastructure", "mobile", "security"]),
                ("process", &["sprint planning", "code review", "oncall rotation"]),
                ("management_approach", &["Scrum", "Kanban", "SAFe", "no process"]),
            ]),
            difficulty_modifiers: vec!["during a hiring freeze".into(), "with remote-first constraints".into()],
            stakeholder_variants: vec![],
        },
        DomainConfig {
            name: "performance".into(),
            templates: vec![
                "Our {component} is slow. Should we {optimization_action}?".into(),
                "Is {optimization_technique} worth implementing for our {perf_use_case}?".into(),
                "How do we identify what's making our {system} slow?".into(),
                "Should we rewrite {slow_part} in {fast_language}?".into(),
                "Our page loads in {duration}. How do we get to {target}?".into(),
                "Should we add {caching_strategy} to improve response times?".into(),
            ],
            variables: build_variables(&[
                ("component", &["API", "database queries", "frontend bundle", "search"]),
                ("optimization_action", &["add caching", "optimize queries", "add indexes"]),
                ("optimization_technique", &["memoization", "lazy loading", "connection pooling"]),
                ("perf_use_case", &["dashboard loads", "search results", "report generation"]),
                ("system", &["application", "database", "backend", "frontend"]),
                ("slow_part", &["the hot path", "image processing", "data transformation"]),
                ("fast_language", &["Rust", "Go", "C++", "WASM"]),
                ("duration", &["5 seconds", "10 seconds", "30 seconds"]),
                ("target", &["under 1 second", "sub-500ms", "instant"]),
                ("caching_strategy", &["Redis", "in-memory cache", "CDN", "edge caching"]),
            ]),
            difficulty_modifiers: vec!["without increasing costs".into(), "while maintaining correctness".into()],
            stakeholder_variants: vec![],
        },
        DomainConfig {
            name: "build-vs-buy".into(),
            templates: vec![
                "Should we build our own {build_system} or use {vendor}?".into(),
                "Is {saas_product} worth the cost vs building in-house?".into(),
                "Our {vendor_contract} contract is up. Build or renew?".into(),
                "Should we {replace_action} our custom {build_system} with {off_shelf}?".into(),
                "We need {capability}. Build, buy, or partner?".into(),
            ],
            variables: build_variables(&[
                ("build_system", &["auth system", "billing", "analytics", "search"]),
                ("vendor", &["Auth0", "Stripe", "Segment", "Algolia"]),
                ("saas_product", &["DataDog", "PagerDuty", "LaunchDarkly", "Amplitude"]),
                ("vendor_contract", &["Stripe", "AWS", "DataDog"]),
                ("replace_action", &["replace", "sunset", "migrate away from"]),
                ("off_shelf", &["an open-source solution", "a SaaS product", "a cloud service"]),
                ("capability", &["real-time analytics", "ML infrastructure", "notification system"]),
            ]),
            difficulty_modifiers: vec!["with compliance requirements".into(), "when vendor lock-in is a concern".into()],
            stakeholder_variants: vec![],
        },
        DomainConfig {
            name: "tech-debt".into(),
            templates: vec![
                "Our {codebase_area} has {debt_level} technical debt. What now?".into(),
                "Should we stop features and address our {debt_type}?".into(),
                "The code is so tangled that {symptom}. What should we do?".into(),
                "Is it time to pay down our {debt_category} debt?".into(),
                "Every change {breaking_symptom}. How do we fix this?".into(),
                "Our {legacy_system} hasn't been touched in {time}. Rewrite or leave it?".into(),
            ],
            variables: build_variables(&[
                ("codebase_area", &["backend", "frontend", "data layer", "API"]),
                ("debt_level", &["crushing", "significant", "growing", "concerning"]),
                ("debt_type", &["architectural debt", "testing debt", "documentation debt"]),
                ("symptom", &["every change breaks something", "nobody understands it", "deployments are scary"]),
                ("debt_category", &["infrastructure", "code quality", "dependency", "design"]),
                ("breaking_symptom", &["breaks something unexpected", "requires touching 20 files", "causes regressions"]),
                ("legacy_system", &["billing system", "user management", "reporting module"]),
                ("time", &["2 years", "5 years", "since the founders built it"]),
            ]),
            difficulty_modifiers: vec!["while shipping features".into(), "with limited engineering time".into()],
            stakeholder_variants: vec![],
        },
        DomainConfig {
            name: "database".into(),
            templates: vec![
                "Should we switch from {db_old} to {db_new}?".into(),
                "Our database is {db_problem}. What are our options?".into(),
                "Is {db_pattern} appropriate for our {data_use_case}?".into(),
                "Should we add {db_feature} to improve {performance_aspect}?".into(),
                "How do we handle {data_challenge} at our scale?".into(),
            ],
            variables: build_variables(&[
                ("db_old", &["PostgreSQL", "MySQL", "MongoDB", "SQLite", "Oracle"]),
                ("db_new", &["PostgreSQL", "CockroachDB", "TiDB", "ScyllaDB", "DynamoDB"]),
                ("db_problem", &["hitting connection limits", "running out of disk", "too slow"]),
                ("db_pattern", &["sharding", "read replicas", "multi-master", "CQRS"]),
                ("data_use_case", &["analytics workload", "OLTP", "time-series data", "graph data"]),
                ("db_feature", &["read replicas", "connection pooling", "materialized views"]),
                ("performance_aspect", &["read latency", "write throughput", "query performance"]),
                ("data_challenge", &["schema migrations", "data consistency", "backup/restore"]),
            ]),
            difficulty_modifiers: vec!["with zero downtime".into(), "while maintaining ACID guarantees".into()],
            stakeholder_variants: vec![],
        },
        DomainConfig {
            name: "security".into(),
            templates: vec![
                "Should we implement {security_measure} for our {sec_system}?".into(),
                "How do we handle {security_concern} in our {sec_context}?".into(),
                "Is {auth_approach} the right choice for {sec_use_case}?".into(),
                "Our {security_audit} found {finding}. How serious is this?".into(),
                "Should we {security_action} before {deadline}?".into(),
            ],
            variables: build_variables(&[
                ("security_measure", &["MFA", "SSO", "encryption at rest", "WAF"]),
                ("sec_system", &["customer portal", "admin panel", "API", "mobile app"]),
                ("security_concern", &["credential rotation", "secret management", "access control"]),
                ("sec_context", &["multi-tenant environment", "regulated industry", "public API"]),
                ("auth_approach", &["OAuth 2.0", "JWT", "session tokens", "API keys"]),
                ("sec_use_case", &["B2B SaaS", "consumer app", "internal tools", "partner integrations"]),
                ("security_audit", &["pentest", "security review", "compliance audit"]),
                ("finding", &["SQL injection", "IDOR vulnerabilities", "weak encryption"]),
                ("security_action", &["fix critical vulnerabilities", "achieve SOC2", "implement zero-trust"]),
                ("deadline", &["a big customer deal", "compliance deadline", "board meeting"]),
            ]),
            difficulty_modifiers: vec!["without disrupting users".into(), "with a compliance deadline".into()],
            stakeholder_variants: vec![],
        },
        DomainConfig {
            name: "devops".into(),
            templates: vec![
                "Should we move to {infra_platform} from {current_setup}?".into(),
                "Is {devops_practice} worth implementing for our team?".into(),
                "Our {deployment_issue} is causing problems. Solutions?".into(),
                "Should we adopt {devops_tool} for {devops_use_case}?".into(),
                "How do we improve our {devops_metric}?".into(),
            ],
            variables: build_variables(&[
                ("infra_platform", &["Kubernetes", "serverless", "ECS", "bare metal"]),
                ("current_setup", &["VMs", "Heroku", "Docker Compose", "manual deploys"]),
                ("devops_practice", &["GitOps", "trunk-based development", "feature flags"]),
                ("deployment_issue", &["slow deployments", "frequent rollbacks", "environment drift"]),
                ("devops_tool", &["Terraform", "Pulumi", "ArgoCD", "Datadog"]),
                ("devops_use_case", &["infrastructure as code", "monitoring", "CI/CD", "secrets management"]),
                ("devops_metric", &["deployment frequency", "MTTR", "change failure rate"]),
            ]),
            difficulty_modifiers: vec!["with a small ops team".into(), "during a cloud migration".into()],
            stakeholder_variants: vec![],
        },
    ]
}

/// Generate a single synthetic question from a template
pub fn generate_question(
    domain: &DomainConfig,
    template_idx: usize,
    variable_choices: &HashMap<String, usize>,
    stakeholder: &str,
    company_stage: &str,
    urgency: &str,
    difficulty_modifier: Option<&str>,
    id: usize,
) -> SyntheticQuestion {
    let template = &domain.templates[template_idx % domain.templates.len()];
    let mut question = template.clone();
    let mut variables_used = HashMap::new();

    // Substitute all variables
    for (var_name, values) in &domain.variables {
        let placeholder = format!("{{{}}}", var_name);
        if question.contains(&placeholder) {
            let choice_idx = variable_choices.get(var_name).copied().unwrap_or(0);
            let value = &values[choice_idx % values.len()];
            question = question.replace(&placeholder, value);
            variables_used.insert(var_name.clone(), value.clone());
        }
    }

    // Add difficulty modifier if provided
    if let Some(modifier) = difficulty_modifier {
        question = format!("{} {}", question, modifier);
    }

    SyntheticQuestion {
        id: format!("syn-{}-{:06}", domain.name, id),
        question,
        domain: domain.name.clone(),
        difficulty: (variable_choices.values().sum::<usize>() % 5 + 1) as u8,
        stakeholder: stakeholder.into(),
        company_stage: company_stage.into(),
        urgency: urgency.into(),
        template_id: format!("{}-{}", domain.name, template_idx),
        variables_used,
    }
}

/// Generate all possible combinations for a domain (exhaustive)
pub fn generate_exhaustive(config: &GeneratorConfig) -> Vec<SyntheticQuestion> {
    let mut questions = Vec::new();
    let mut id = 0;

    for domain in &config.domains {
        for (template_idx, _template) in domain.templates.iter().enumerate() {
            let var_names: Vec<_> = domain.variables.keys().cloned().collect();
            let var_sizes: Vec<_> = var_names.iter()
                .map(|k| domain.variables.get(k).map(|v| v.len()).unwrap_or(1))
                .collect();

            let mut indices: Vec<usize> = vec![0; var_names.len()];

            loop {
                let mut choices = HashMap::new();
                for (i, name) in var_names.iter().enumerate() {
                    choices.insert(name.clone(), indices[i]);
                }

                for stakeholder in &config.stakeholders {
                    for stage in &config.company_stages {
                        for urgency in &config.urgency_levels {
                            let q = generate_question(
                                domain, template_idx, &choices,
                                stakeholder, stage, urgency, None, id,
                            );
                            questions.push(q);
                            id += 1;
                            if id >= config.target_count { return questions; }
                        }
                    }
                }

                let mut carry = true;
                for i in 0..indices.len() {
                    if carry {
                        indices[i] += 1;
                        if indices[i] >= var_sizes[i] { indices[i] = 0; } else { carry = false; }
                    }
                }
                if carry { break; }
            }
        }
    }
    questions
}

/// Generate a random sample of questions
pub fn generate_sample(config: &GeneratorConfig, count: usize, seed: u64) -> Vec<SyntheticQuestion> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut questions = Vec::with_capacity(count);

    for i in 0..count {
        let mut hasher = DefaultHasher::new();
        (seed, i).hash(&mut hasher);
        let hash = hasher.finish();

        let domain_idx = (hash % config.domains.len() as u64) as usize;
        let domain = &config.domains[domain_idx];
        let template_idx = ((hash >> 8) % domain.templates.len() as u64) as usize;
        let stakeholder_idx = ((hash >> 16) % config.stakeholders.len() as u64) as usize;
        let stage_idx = ((hash >> 24) % config.company_stages.len() as u64) as usize;
        let urgency_idx = ((hash >> 32) % config.urgency_levels.len() as u64) as usize;

        let mut choices = HashMap::new();
        let mut shift = 40u64;
        for (var_name, values) in &domain.variables {
            let choice_idx = ((hash >> shift) % values.len() as u64) as usize;
            choices.insert(var_name.clone(), choice_idx);
            shift = (shift + 8) % 64;
        }

        let q = generate_question(
            domain, template_idx, &choices,
            &config.stakeholders[stakeholder_idx],
            &config.company_stages[stage_idx],
            &config.urgency_levels[urgency_idx],
            None, i,
        );
        questions.push(q);
    }
    questions
}

/// Hard negative generator - questions that SEEM related but shouldn't cite a principle
pub fn generate_hard_negatives(principle_name: &str, principle_domain: &str) -> Vec<String> {
    let mut negatives = Vec::new();
    negatives.push(format!("What are the main criticisms of {}?", principle_name));
    negatives.push(format!("When does {} NOT apply?", principle_name));
    negatives.push(format!("Give me examples where {} failed", principle_name));

    match principle_domain {
        "architecture" => {
            negatives.push("Should I use TDD for this feature?".into());
            negatives.push("How do I motivate my team?".into());
        }
        "testing" => {
            negatives.push("Should we adopt microservices?".into());
            negatives.push("How do we scale the database?".into());
        }
        "management" => {
            negatives.push("What caching strategy should we use?".into());
            negatives.push("Should we rewrite in Rust?".into());
        }
        _ => {}
    }

    negatives.push(format!("I heard about {} but it doesn't apply to us, right?", principle_name));
    negatives.push(format!("My friend says {} is outdated. Is that true?", principle_name));
    negatives
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_sample() {
        let config = GeneratorConfig::default();
        let questions = generate_sample(&config, 100, 42);
        assert_eq!(questions.len(), 100);

        let domains: std::collections::HashSet<_> = questions.iter().map(|q| q.domain.as_str()).collect();
        assert!(domains.len() > 3, "Should have multiple domains");

        for q in &questions {
            assert!(!q.question.is_empty());
            assert!(!q.question.contains('{'), "Unsubstituted variable: {}", q.question);
        }
    }

    #[test]
    fn test_deterministic_generation() {
        let config = GeneratorConfig::default();
        let q1 = generate_sample(&config, 10, 123);
        let q2 = generate_sample(&config, 10, 123);
        for (a, b) in q1.iter().zip(q2.iter()) {
            assert_eq!(a.question, b.question, "Should be deterministic");
        }
    }

    #[test]
    fn test_hard_negatives() {
        let negatives = generate_hard_negatives("YAGNI", "architecture");
        assert!(!negatives.is_empty());
        assert!(negatives.iter().any(|n| n.contains("criticism")));
    }
}
