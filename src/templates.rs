//! Decision Templates: Pre-built decision patterns for common situations
//!
//! These aren't generic frameworks - they're opinionated decision trees
//! built from real-world patterns with specific principle combinations.
//!
//! Philosophy: "Give me a lever long enough and I'll move the world" - Archimedes
//! The right template + the right principles = 10x faster decisions.

use serde::{Deserialize, Serialize};

/// A decision template for a common situation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionTemplate {
    pub id: String,
    pub name: String,
    pub description: String,
    pub domain: String,

    /// Trigger patterns that indicate this template applies
    pub triggers: Vec<TriggerPattern>,

    /// The decision tree to follow
    pub tree: DecisionTree,

    /// Principles that synergize for this decision type
    pub synergies: Vec<PrincipleSynergy>,

    /// Principles that conflict (pick one, not both)
    pub tensions: Vec<PrincipleTension>,

    /// Common blind spots when making this decision
    pub blind_spots: Vec<BlindSpot>,

    /// Anti-patterns to avoid
    pub anti_patterns: Vec<AntiPattern>,

    /// Historical success rate from outcomes
    pub success_rate: f64,
    pub times_used: u32,
}

/// Pattern to detect when a template applies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerPattern {
    /// Keywords that suggest this template
    pub keywords: Vec<String>,
    /// Phrases that strongly indicate this template
    pub phrases: Vec<String>,
    /// Minimum confidence to trigger (0.0-1.0)
    pub min_confidence: f64,
}

/// A node in the decision tree
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionTree {
    pub question: String,
    pub help_text: Option<String>,
    pub options: Vec<DecisionOption>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionOption {
    pub label: String,
    pub description: String,
    /// Principles to apply if this option is chosen
    pub principles: Vec<String>,
    /// Next question, or None if this is a leaf
    pub next: Option<Box<DecisionTree>>,
    /// Final recommendation if this is a leaf
    pub recommendation: Option<String>,
}

/// Principles that work well together
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrincipleSynergy {
    pub principles: Vec<String>,
    pub thinkers: Vec<String>,
    pub why: String,
    pub combined_power: String,
}

/// Principles that conflict - must choose one
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrincipleTension {
    pub principle_a: String,
    pub principle_b: String,
    pub thinker_a: String,
    pub thinker_b: String,
    pub when_to_pick_a: String,
    pub when_to_pick_b: String,
}

/// Something commonly overlooked
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlindSpot {
    pub name: String,
    pub description: String,
    pub check_question: String,
    pub severity: BlindSpotSeverity,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum BlindSpotSeverity {
    Critical, // Will definitely cause failure if missed
    High,     // Likely to cause problems
    Medium,   // Should consider
    Low,      // Nice to check
}

/// Pattern to avoid
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AntiPattern {
    pub name: String,
    pub description: String,
    pub symptoms: Vec<String>,
    pub cure: String,
    pub source_thinker: String,
}

/// Get all built-in decision templates
pub fn get_templates() -> Vec<DecisionTemplate> {
    vec![
        monolith_vs_microservices(),
        rewrite_vs_refactor(),
        build_vs_buy(),
        scale_team(),
        technical_debt(),
        mvp_scope(),
        architecture_migration(),
        database_choice(),
        feature_prioritization(),
        api_design(),
        testing_strategy(),
        performance_optimization(),
    ]
}

/// Match question to templates
pub fn match_templates(question: &str) -> Vec<(DecisionTemplate, f64)> {
    let q_lower = question.to_lowercase();
    let mut matches = Vec::new();

    for template in get_templates() {
        let mut score = 0.0;

        for trigger in &template.triggers {
            // Check keywords
            for keyword in &trigger.keywords {
                if q_lower.contains(&keyword.to_lowercase()) {
                    score += 1.0;
                }
            }

            // Check phrases (higher weight)
            for phrase in &trigger.phrases {
                if q_lower.contains(&phrase.to_lowercase()) {
                    score += 3.0;
                }
            }
        }

        if score >= 2.0 {
            matches.push((template, score));
        }
    }

    // Sort by score descending
    matches.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    matches
}

// ============================================================================
// TEMPLATE DEFINITIONS - These are the 10x differentiators
// ============================================================================

fn monolith_vs_microservices() -> DecisionTemplate {
    DecisionTemplate {
        id: "monolith-vs-microservices".to_string(),
        name: "Monolith vs Microservices".to_string(),
        description: "Should we build a monolith or use microservices?".to_string(),
        domain: "software-architecture".to_string(),

        triggers: vec![TriggerPattern {
            keywords: vec![
                "microservices".to_string(),
                "monolith".to_string(),
                "architecture".to_string(),
                "services".to_string(),
                "decompose".to_string(),
            ],
            phrases: vec![
                "should we use microservices".to_string(),
                "monolith or microservices".to_string(),
                "break into services".to_string(),
                "service architecture".to_string(),
            ],
            min_confidence: 0.6,
        }],

        tree: DecisionTree {
            question: "Is this a NEW project or EXISTING system?".to_string(),
            help_text: Some("The answer changes everything about this decision".to_string()),
            options: vec![
                DecisionOption {
                    label: "NEW project".to_string(),
                    description: "Starting from scratch".to_string(),
                    principles: vec!["Monolith First".to_string()],
                    recommendation: None,
                    next: Some(Box::new(DecisionTree {
                        question: "How well do you understand the domain?".to_string(),
                        help_text: None,
                        options: vec![
                            DecisionOption {
                                label: "Not well - still learning".to_string(),
                                description: "Domain boundaries unclear".to_string(),
                                principles: vec!["Bounded Context".to_string()],
                                next: None,
                                recommendation: Some(
                                    "BUILD A MONOLITH. Martin Fowler: 'You need to understand \
                                    the domain before you can identify service boundaries. \
                                    A premature split creates distributed monolith - worst of both worlds.'".to_string()
                                ),
                            },
                            DecisionOption {
                                label: "Very well - clear boundaries".to_string(),
                                description: "Domain is mature and understood".to_string(),
                                principles: vec!["Database Per Service".to_string()],
                                recommendation: None,
                                next: Some(Box::new(DecisionTree {
                                    question: "Do you have a team per service?".to_string(),
                                    help_text: Some("Conway's Law: architecture mirrors org structure".to_string()),
                                    options: vec![
                                        DecisionOption {
                                            label: "Yes - team per service".to_string(),
                                            description: "Each service has dedicated ownership".to_string(),
                                            principles: vec!["Model Around Business Capabilities".to_string()],
                                            next: None,
                                            recommendation: Some(
                                                "MICROSERVICES ARE VIABLE. You have: clear domain, team per service. \
                                                But consider: start with 2-3 services, not 20. Sam Newman: 'The goal \
                                                is independent deployability, not the maximum number of services.'".to_string()
                                            ),
                                        },
                                        DecisionOption {
                                            label: "No - small team, many services".to_string(),
                                            description: "One team maintaining multiple services".to_string(),
                                            principles: vec!["Brooks's Law".to_string()],
                                            next: None,
                                            recommendation: Some(
                                                "BUILD A MODULAR MONOLITH. You understand the domain but don't have \
                                                the team. Fred Brooks: 'More services with the same team = more \
                                                coordination overhead, slower delivery.'".to_string()
                                            ),
                                        },
                                    ],
                                })),
                            },
                        ],
                    })),
                },
                DecisionOption {
                    label: "EXISTING monolith".to_string(),
                    description: "Want to migrate to microservices".to_string(),
                    principles: vec!["Strangler Fig".to_string(), "Incremental Migration".to_string()],
                    recommendation: None,
                    next: Some(Box::new(DecisionTree {
                        question: "What's driving the migration?".to_string(),
                        help_text: None,
                        options: vec![
                            DecisionOption {
                                label: "Scaling issues".to_string(),
                                description: "System can't handle load".to_string(),
                                principles: vec!["Design for Failure".to_string()],
                                next: None,
                                recommendation: Some(
                                    "EXTRACT HOTSPOTS ONLY. Identify the 20% of code that handles 80% of load. \
                                    Extract just those. Sam Newman: 'Don't decompose for scale you don't have.'".to_string()
                                ),
                            },
                            DecisionOption {
                                label: "Team scaling".to_string(),
                                description: "Too many people stepping on each other".to_string(),
                                principles: vec!["Conceptual Integrity".to_string()],
                                next: None,
                                recommendation: Some(
                                    "MODULARIZE FIRST, EXTRACT LATER. Clear module boundaries within the monolith \
                                    solve 80% of team coordination. Only extract when modules have clear ownership.".to_string()
                                ),
                            },
                            DecisionOption {
                                label: "Technology modernization".to_string(),
                                description: "Want to use new tech stack".to_string(),
                                principles: vec!["No Silver Bullet".to_string()],
                                next: None,
                                recommendation: Some(
                                    "CAUTION: THIS IS USUALLY THE WRONG REASON. Fred Brooks: 'There is no single \
                                    development that will give 10x improvement.' New tech rarely justifies the \
                                    cost of distribution. Consider: can you modernize within the monolith?".to_string()
                                ),
                            },
                        ],
                    })),
                },
            ],
        },

        synergies: vec![
            PrincipleSynergy {
                principles: vec!["Monolith First".to_string(), "Bounded Context".to_string()],
                thinkers: vec!["Martin Fowler".to_string(), "Eric Evans".to_string()],
                why: "Understand domains in a monolith before extracting services".to_string(),
                combined_power: "Build a modular monolith with clear boundaries that CAN become services later".to_string(),
            },
            PrincipleSynergy {
                principles: vec!["Strangler Fig".to_string(), "Incremental Migration".to_string()],
                thinkers: vec!["Martin Fowler".to_string(), "Sam Newman".to_string()],
                why: "Both advocate gradual replacement over big-bang rewrites".to_string(),
                combined_power: "Migrate piece by piece with continuous production validation".to_string(),
            },
        ],

        tensions: vec![
            PrincipleTension {
                principle_a: "Database Per Service".to_string(),
                principle_b: "ACID Transactions".to_string(),
                thinker_a: "Sam Newman".to_string(),
                thinker_b: "Traditional RDBMS".to_string(),
                when_to_pick_a: "Independent deployability is more valuable than strong consistency".to_string(),
                when_to_pick_b: "Data integrity is critical (financial transactions, inventory)".to_string(),
            },
        ],

        blind_spots: vec![
            BlindSpot {
                name: "Operational Complexity".to_string(),
                description: "Microservices require sophisticated DevOps".to_string(),
                check_question: "Do you have: container orchestration, service mesh, distributed tracing, centralized logging?".to_string(),
                severity: BlindSpotSeverity::Critical,
            },
            BlindSpot {
                name: "Network Failures".to_string(),
                description: "Every service call can fail".to_string(),
                check_question: "Do you have: circuit breakers, timeouts, retry policies, fallbacks?".to_string(),
                severity: BlindSpotSeverity::Critical,
            },
            BlindSpot {
                name: "Data Consistency".to_string(),
                description: "No more ACID across services".to_string(),
                check_question: "How will you handle: eventual consistency, saga patterns, compensation?".to_string(),
                severity: BlindSpotSeverity::High,
            },
        ],

        anti_patterns: vec![
            AntiPattern {
                name: "Distributed Monolith".to_string(),
                description: "Services that must deploy together".to_string(),
                symptoms: vec![
                    "Can't deploy one service without deploying others".to_string(),
                    "Shared database between services".to_string(),
                    "Chatty synchronous communication".to_string(),
                ],
                cure: "If services can't deploy independently, they're not microservices. Merge them.".to_string(),
                source_thinker: "Sam Newman".to_string(),
            },
            AntiPattern {
                name: "Premature Decomposition".to_string(),
                description: "Splitting before understanding domain".to_string(),
                symptoms: vec![
                    "Constantly moving code between services".to_string(),
                    "Services that don't align with business capabilities".to_string(),
                    "Technical layer services (auth-service, logging-service)".to_string(),
                ],
                cure: "Merge back into monolith. Learn the domain. Try again.".to_string(),
                source_thinker: "Martin Fowler".to_string(),
            },
        ],

        success_rate: 0.0,
        times_used: 0,
    }
}

fn rewrite_vs_refactor() -> DecisionTemplate {
    DecisionTemplate {
        id: "rewrite-vs-refactor".to_string(),
        name: "Rewrite vs Refactor".to_string(),
        description: "Should we rewrite this system from scratch or incrementally improve it?".to_string(),
        domain: "software-architecture".to_string(),

        triggers: vec![TriggerPattern {
            keywords: vec![
                "rewrite".to_string(),
                "refactor".to_string(),
                "legacy".to_string(),
                "rebuild".to_string(),
                "from scratch".to_string(),
                "greenfield".to_string(),
            ],
            phrases: vec![
                "should we rewrite".to_string(),
                "start over".to_string(),
                "rewrite from scratch".to_string(),
                "legacy system".to_string(),
                "technical debt".to_string(),
            ],
            min_confidence: 0.6,
        }],

        tree: DecisionTree {
            question: "Can you articulate what's SPECIFICALLY broken?".to_string(),
            help_text: Some("'It's a mess' is not specific enough".to_string()),
            options: vec![
                DecisionOption {
                    label: "Yes - I can list specific problems".to_string(),
                    description: "Clear, enumerable issues".to_string(),
                    principles: vec!["Boy Scout Rule".to_string()],
                    recommendation: None,
                    next: Some(Box::new(DecisionTree {
                        question: "How many of these problems require architectural changes?".to_string(),
                        help_text: None,
                        options: vec![
                            DecisionOption {
                                label: "Few - mostly code quality issues".to_string(),
                                description: "Bad naming, duplication, poor tests".to_string(),
                                principles: vec!["Boy Scout Rule".to_string()],
                                next: None,
                                recommendation: Some(
                                    "REFACTOR. Clean as you go. Martin: 'Leave the code better than you found it.' \
                                    No big rewrite needed - just discipline over time.".to_string()
                                ),
                            },
                            DecisionOption {
                                label: "Many - core architecture is wrong".to_string(),
                                description: "Fundamental structure prevents progress".to_string(),
                                principles: vec!["Strangler Fig".to_string()],
                                recommendation: None,
                                next: Some(Box::new(DecisionTree {
                                    question: "Is the system in production with real users?".to_string(),
                                    help_text: None,
                                    options: vec![
                                        DecisionOption {
                                            label: "Yes - significant production traffic".to_string(),
                                            description: "Users depend on this daily".to_string(),
                                            principles: vec!["Incremental Migration".to_string()],
                                            next: None,
                                            recommendation: Some(
                                                "STRANGLER FIG PATTERN. Never big-bang. Build new alongside old, \
                                                migrate traffic gradually. Fowler: 'The new system grows while \
                                                the old shrinks, like a strangler fig around a tree.'".to_string()
                                            ),
                                        },
                                        DecisionOption {
                                            label: "No - internal/low usage".to_string(),
                                            description: "Few users, low risk".to_string(),
                                            principles: vec!["Plan to Throw One Away".to_string()],
                                            next: None,
                                            recommendation: Some(
                                                "REWRITE IS VIABLE (but still risky). Brooks: 'Plan to throw one \
                                                away - you will anyway.' If you rewrite: (1) Time-box ruthlessly, \
                                                (2) Feature-freeze the old system, (3) Have rollback plan.".to_string()
                                            ),
                                        },
                                    ],
                                })),
                            },
                        ],
                    })),
                },
                DecisionOption {
                    label: "No - it just 'feels' bad".to_string(),
                    description: "General discomfort, no specifics".to_string(),
                    principles: vec!["Second-System Effect".to_string()],
                    next: None,
                    recommendation: Some(
                        "DO NOT REWRITE. This is the #1 trap. Fred Brooks: 'The second system is the \
                        most dangerous - designers add everything they couldn't fit in the first.' \
                        You'll recreate the same problems. Instead: (1) Identify ONE specific pain \
                        point, (2) Fix just that, (3) Repeat.".to_string()
                    ),
                },
            ],
        },

        synergies: vec![
            PrincipleSynergy {
                principles: vec!["Boy Scout Rule".to_string(), "Single Responsibility".to_string()],
                thinkers: vec!["Robert C. Martin".to_string()],
                why: "Small improvements accumulate into major transformation".to_string(),
                combined_power: "Refactor toward SRP while maintaining working software".to_string(),
            },
        ],

        tensions: vec![
            PrincipleTension {
                principle_a: "Plan to Throw One Away".to_string(),
                principle_b: "Incremental Migration".to_string(),
                thinker_a: "Fred Brooks".to_string(),
                thinker_b: "Sam Newman".to_string(),
                when_to_pick_a: "System is small, low-risk, and you deeply understand what went wrong".to_string(),
                when_to_pick_b: "System is in production, has users, and business continuity matters".to_string(),
            },
        ],

        blind_spots: vec![
            BlindSpot {
                name: "Hidden Business Rules".to_string(),
                description: "The 'mess' often encodes years of edge cases".to_string(),
                check_question: "Do you know ALL the business rules encoded in the current system?".to_string(),
                severity: BlindSpotSeverity::Critical,
            },
            BlindSpot {
                name: "Rewrite Duration".to_string(),
                description: "Rewrites always take 2-3x longer than estimated".to_string(),
                check_question: "Can the business wait that long without new features?".to_string(),
                severity: BlindSpotSeverity::High,
            },
        ],

        anti_patterns: vec![
            AntiPattern {
                name: "Second System Effect".to_string(),
                description: "Over-engineering the replacement".to_string(),
                symptoms: vec![
                    "Adding features the old system didn't have".to_string(),
                    "'While we're at it' mentality".to_string(),
                    "Scope growing during rewrite".to_string(),
                ],
                cure: "Feature parity first. No new features until old system is fully replaced.".to_string(),
                source_thinker: "Fred Brooks".to_string(),
            },
        ],

        success_rate: 0.0,
        times_used: 0,
    }
}

fn build_vs_buy() -> DecisionTemplate {
    DecisionTemplate {
        id: "build-vs-buy".to_string(),
        name: "Build vs Buy".to_string(),
        description: "Should we build this ourselves or use an existing solution?".to_string(),
        domain: "software-architecture".to_string(),

        triggers: vec![TriggerPattern {
            keywords: vec![
                "build".to_string(),
                "buy".to_string(),
                "vendor".to_string(),
                "saas".to_string(),
                "library".to_string(),
                "framework".to_string(),
                "make".to_string(),
            ],
            phrases: vec![
                "build or buy".to_string(),
                "build vs buy".to_string(),
                "use a library".to_string(),
                "use saas".to_string(),
                "third party".to_string(),
                "roll our own".to_string(),
            ],
            min_confidence: 0.6,
        }],

        tree: DecisionTree {
            question: "Is this capability your CORE BUSINESS or supporting infrastructure?".to_string(),
            help_text: Some("Core = what you compete on. Supporting = enables core but isn't differentiated.".to_string()),
            options: vec![
                DecisionOption {
                    label: "CORE - competitive differentiator".to_string(),
                    description: "This is what makes us special".to_string(),
                    principles: vec!["Focus on Core".to_string()],
                    next: None,
                    recommendation: Some(
                        "BUILD IT. Naval Ravikant: 'If it's your competitive advantage, you must \
                        own it.' Outsourcing your moat is outsourcing your business. But: start \
                        with the simplest version that tests your hypothesis.".to_string()
                    ),
                },
                DecisionOption {
                    label: "SUPPORTING - enables but doesn't differentiate".to_string(),
                    description: "Needed but not unique to us".to_string(),
                    principles: vec!["Not Invented Here Antipattern".to_string()],
                    recommendation: None,
                    next: Some(Box::new(DecisionTree {
                        question: "Does a good solution exist that fits your needs?".to_string(),
                        help_text: None,
                        options: vec![
                            DecisionOption {
                                label: "Yes - mature solution exists".to_string(),
                                description: "Well-maintained, fits 80%+ of needs".to_string(),
                                principles: vec!["YAGNI".to_string()],
                                next: None,
                                recommendation: Some(
                                    "BUY/USE IT. Kent Beck: 'You Ain't Gonna Need It' applies to building \
                                    what already exists. Your time is better spent on what makes you unique. \
                                    Accept the 20% that doesn't fit perfectly.".to_string()
                                ),
                            },
                            DecisionOption {
                                label: "No - nothing fits well".to_string(),
                                description: "Existing solutions need heavy customization".to_string(),
                                principles: vec!["Worse is Better".to_string()],
                                next: None,
                                recommendation: Some(
                                    "BUILD A MINIMAL VERSION. If nothing fits, build the simplest thing \
                                    that works. Richard Gabriel: 'Worse is better' - a simple solution \
                                    that works beats a complex one that doesn't. Don't over-engineer.".to_string()
                                ),
                            },
                        ],
                    })),
                },
            ],
        },

        synergies: vec![],
        tensions: vec![],

        blind_spots: vec![
            BlindSpot {
                name: "Total Cost of Ownership".to_string(),
                description: "Build cost is obvious; maintenance cost is hidden".to_string(),
                check_question: "Have you calculated: ongoing maintenance, security updates, team training, documentation?".to_string(),
                severity: BlindSpotSeverity::High,
            },
            BlindSpot {
                name: "Vendor Lock-in".to_string(),
                description: "How hard is it to switch later?".to_string(),
                check_question: "What's the exit strategy if this vendor fails or raises prices?".to_string(),
                severity: BlindSpotSeverity::Medium,
            },
        ],

        anti_patterns: vec![
            AntiPattern {
                name: "Not Invented Here".to_string(),
                description: "Refusing to use external code".to_string(),
                symptoms: vec![
                    "Building logging frameworks".to_string(),
                    "Custom auth when OAuth exists".to_string(),
                    "'We can do it better'".to_string(),
                ],
                cure: "Ask: 'Is building this what we're paid to do?'".to_string(),
                source_thinker: "Industry Wisdom".to_string(),
            },
        ],

        success_rate: 0.0,
        times_used: 0,
    }
}

fn scale_team() -> DecisionTemplate {
    DecisionTemplate {
        id: "scale-team".to_string(),
        name: "Scaling the Team".to_string(),
        description: "Should we add more people to speed up delivery?".to_string(),
        domain: "management-theory".to_string(),

        triggers: vec![TriggerPattern {
            keywords: vec![
                "hire".to_string(),
                "team".to_string(),
                "people".to_string(),
                "scale".to_string(),
                "staff".to_string(),
                "developers".to_string(),
                "engineers".to_string(),
                "late".to_string(),
                "behind".to_string(),
                "speed".to_string(),
                "adding".to_string(),
                "headcount".to_string(),
            ],
            phrases: vec![
                "add more people".to_string(),
                "adding people".to_string(),
                "adding more".to_string(),
                "scale the team".to_string(),
                "hire developers".to_string(),
                "project behind".to_string(),
                "behind schedule".to_string(),
                "need more engineers".to_string(),
                "late project".to_string(),
                "speed up".to_string(),
                "grow the team".to_string(),
            ],
            min_confidence: 0.6,
        }],

        tree: DecisionTree {
            question: "Is the project already late or behind schedule?".to_string(),
            help_text: Some("This fundamentally changes whether adding people helps".to_string()),
            options: vec![
                DecisionOption {
                    label: "Yes - we're already behind".to_string(),
                    description: "Deadline missed or at risk".to_string(),
                    principles: vec!["Brooks's Law".to_string()],
                    next: None,
                    recommendation: Some(
                        "DO NOT ADD PEOPLE. Fred Brooks: 'Adding people to a late project makes it later.' \
                        New hires require training (stealing time from existing team), increase communication \
                        overhead (n*(n-1)/2 channels), and won't be productive for months. Instead: \
                        (1) Cut scope, (2) Extend deadline, (3) Improve process efficiency.".to_string()
                    ),
                },
                DecisionOption {
                    label: "No - planning for future capacity".to_string(),
                    description: "Project on track, thinking ahead".to_string(),
                    principles: vec!["Two Pizza Teams".to_string()],
                    recommendation: None,
                    next: Some(Box::new(DecisionTree {
                        question: "What's the current team size?".to_string(),
                        help_text: None,
                        options: vec![
                            DecisionOption {
                                label: "Small (2-5 people)".to_string(),
                                description: "High bandwidth communication".to_string(),
                                principles: vec!["Surgical Team".to_string()],
                                next: None,
                                recommendation: Some(
                                    "ADD CAREFULLY. Brooks recommends the 'surgical team' model - one \
                                    chief programmer supported by specialists. Add people who extend \
                                    capability (DevOps, QA, Design) not duplicate it.".to_string()
                                ),
                            },
                            DecisionOption {
                                label: "Medium (6-10 people)".to_string(),
                                description: "Communication getting harder".to_string(),
                                principles: vec!["Two Pizza Teams".to_string()],
                                next: None,
                                recommendation: Some(
                                    "SPLIT BEFORE SCALING. Bezos: 'If a team can't be fed by two pizzas, \
                                    it's too big.' Split into two focused teams with clear ownership \
                                    before adding more people. Otherwise: too many meetings, too much coordination.".to_string()
                                ),
                            },
                            DecisionOption {
                                label: "Large (10+ people)".to_string(),
                                description: "Already struggling with coordination".to_string(),
                                principles: vec!["Conceptual Integrity".to_string()],
                                next: None,
                                recommendation: Some(
                                    "RESTRUCTURE FIRST. Adding more people will make coordination worse. \
                                    Brooks: 'Conceptual integrity requires a small number of minds.' \
                                    Create smaller, autonomous teams with clear boundaries before scaling.".to_string()
                                ),
                            },
                        ],
                    })),
                },
            ],
        },

        synergies: vec![],
        tensions: vec![],

        blind_spots: vec![
            BlindSpot {
                name: "Onboarding Cost".to_string(),
                description: "New hires drain existing team for 3-6 months".to_string(),
                check_question: "Who will train new hires? Can they afford to lose that productivity?".to_string(),
                severity: BlindSpotSeverity::Critical,
            },
        ],

        anti_patterns: vec![],
        success_rate: 0.0,
        times_used: 0,
    }
}

fn technical_debt() -> DecisionTemplate {
    DecisionTemplate {
        id: "technical-debt".to_string(),
        name: "Technical Debt".to_string(),
        description: "How should we handle technical debt?".to_string(),
        domain: "software-architecture".to_string(),

        triggers: vec![TriggerPattern {
            keywords: vec![
                "debt".to_string(),
                "cleanup".to_string(),
                "refactor".to_string(),
                "quality".to_string(),
                "mess".to_string(),
            ],
            phrases: vec![
                "technical debt".to_string(),
                "pay down debt".to_string(),
                "code quality".to_string(),
                "cleanup sprint".to_string(),
            ],
            min_confidence: 0.6,
        }],

        tree: DecisionTree {
            question: "Is this debt BLOCKING new features or just ANNOYING?".to_string(),
            help_text: None,
            options: vec![
                DecisionOption {
                    label: "BLOCKING - can't add features until fixed".to_string(),
                    description: "Must refactor to proceed".to_string(),
                    principles: vec!["Boy Scout Rule".to_string()],
                    next: None,
                    recommendation: Some(
                        "FIX IT NOW. But: scope it tightly. Fix the minimum needed to unblock \
                        the feature, not 'while we're here.' Tag remaining debt for future.".to_string()
                    ),
                },
                DecisionOption {
                    label: "ANNOYING - slows us down but workable".to_string(),
                    description: "Would be nice to fix".to_string(),
                    principles: vec!["Boy Scout Rule".to_string()],
                    next: None,
                    recommendation: Some(
                        "INCREMENTAL. Martin's Boy Scout Rule: 'Leave code better than you found it.' \
                        Don't do 'cleanup sprints' - weave small improvements into every feature. \
                        10 minutes of cleanup per PR compounds over time.".to_string()
                    ),
                },
            ],
        },

        synergies: vec![],
        tensions: vec![],
        blind_spots: vec![],
        anti_patterns: vec![],
        success_rate: 0.0,
        times_used: 0,
    }
}

fn mvp_scope() -> DecisionTemplate {
    DecisionTemplate {
        id: "mvp-scope".to_string(),
        name: "MVP Scope".to_string(),
        description: "What should be in our MVP?".to_string(),
        domain: "entrepreneurship".to_string(),

        triggers: vec![TriggerPattern {
            keywords: vec![
                "mvp".to_string(),
                "scope".to_string(),
                "feature".to_string(),
                "launch".to_string(),
                "minimum".to_string(),
                "viable".to_string(),
            ],
            phrases: vec![
                "mvp scope".to_string(),
                "what to include".to_string(),
                "must have".to_string(),
                "nice to have".to_string(),
                "minimum viable".to_string(),
            ],
            min_confidence: 0.6,
        }],

        tree: DecisionTree {
            question: "What's the ONE thing you're trying to learn from this MVP?".to_string(),
            help_text: Some("MVPs test hypotheses, not build products".to_string()),
            options: vec![
                DecisionOption {
                    label: "Will people PAY for this?".to_string(),
                    description: "Testing business model".to_string(),
                    principles: vec!["80/20 Analysis".to_string()],
                    next: None,
                    recommendation: Some(
                        "PAYMENT FLOW ONLY. Tim Ferriss 80/20: 20% of features drive 80% of value. \
                        Build: landing page, payment, ONE core feature. No login, no settings, no nice-to-have. \
                        If they won't pay for a rough version, polish won't help.".to_string()
                    ),
                },
                DecisionOption {
                    label: "Can we BUILD this?".to_string(),
                    description: "Testing technical feasibility".to_string(),
                    principles: vec!["Make it Work, Make it Right, Make it Fast".to_string()],
                    next: None,
                    recommendation: Some(
                        "SPIKE THEN DECIDE. Build the hardest technical piece first. If it works, \
                        you know you can do it. If it doesn't, you saved months. Kent Beck: \
                        'Make it work' before 'make it right.'".to_string()
                    ),
                },
                DecisionOption {
                    label: "Will people USE this?".to_string(),
                    description: "Testing user engagement".to_string(),
                    principles: vec!["YAGNI".to_string()],
                    next: None,
                    recommendation: Some(
                        "CORE LOOP ONLY. Build one complete user journey. No edge cases, no \
                        error handling, no admin. Kent Beck's YAGNI: 'You Ain't Gonna Need It' \
                        until users prove they do.".to_string()
                    ),
                },
            ],
        },

        synergies: vec![],
        tensions: vec![],
        blind_spots: vec![],
        anti_patterns: vec![],
        success_rate: 0.0,
        times_used: 0,
    }
}

fn architecture_migration() -> DecisionTemplate {
    DecisionTemplate {
        id: "architecture-migration".to_string(),
        name: "Architecture Migration".to_string(),
        description: "How should we migrate to a new architecture?".to_string(),
        domain: "software-architecture".to_string(),

        triggers: vec![TriggerPattern {
            keywords: vec![
                "migration".to_string(),
                "migrate".to_string(),
                "modernize".to_string(),
                "upgrade".to_string(),
                "move".to_string(),
            ],
            phrases: vec![
                "migrate to".to_string(),
                "migration strategy".to_string(),
                "move to cloud".to_string(),
                "upgrade architecture".to_string(),
            ],
            min_confidence: 0.6,
        }],

        tree: DecisionTree {
            question: "Is this migration REQUIRED or DESIRED?".to_string(),
            help_text: Some("Required = can't continue without it. Desired = would be nice.".to_string()),
            options: vec![
                DecisionOption {
                    label: "REQUIRED - forced by external factors".to_string(),
                    description: "Vendor EOL, compliance, scaling limits".to_string(),
                    principles: vec!["Incremental Migration".to_string()],
                    next: None,
                    recommendation: Some(
                        "STRANGLER FIG + DEADLINE. You must migrate, but do it safely: \
                        (1) Build new system alongside old, (2) Migrate traffic gradually, \
                        (3) Have rollback plan. Sam Newman: 'Never big-bang a required migration.'".to_string()
                    ),
                },
                DecisionOption {
                    label: "DESIRED - we want to improve".to_string(),
                    description: "No external pressure".to_string(),
                    principles: vec!["No Silver Bullet".to_string()],
                    next: None,
                    recommendation: Some(
                        "QUESTION THE NEED. Fred Brooks: 'There is no silver bullet.' New architecture \
                        won't magically solve problems. Ask: (1) What specific problem does this solve? \
                        (2) Can we solve it without migration? (3) Is the pain worth the gain?".to_string()
                    ),
                },
            ],
        },

        synergies: vec![],
        tensions: vec![],
        blind_spots: vec![],
        anti_patterns: vec![],
        success_rate: 0.0,
        times_used: 0,
    }
}

fn database_choice() -> DecisionTemplate {
    DecisionTemplate {
        id: "database-choice".to_string(),
        name: "Database Choice".to_string(),
        description: "What database should we use?".to_string(),
        domain: "software-architecture".to_string(),

        triggers: vec![TriggerPattern {
            keywords: vec![
                "database".to_string(),
                "postgres".to_string(),
                "mongodb".to_string(),
                "mysql".to_string(),
                "redis".to_string(),
                "dynamodb".to_string(),
                "sql".to_string(),
                "nosql".to_string(),
            ],
            phrases: vec![
                "which database".to_string(),
                "database choice".to_string(),
                "sql vs nosql".to_string(),
                "relational or document".to_string(),
            ],
            min_confidence: 0.6,
        }],

        tree: DecisionTree {
            question: "What's your data shape?".to_string(),
            help_text: None,
            options: vec![
                DecisionOption {
                    label: "Structured with relationships".to_string(),
                    description: "Users have orders have items".to_string(),
                    principles: vec!["Simple Thing That Works".to_string()],
                    next: None,
                    recommendation: Some(
                        "POSTGRES. Don't overthink it. It handles: relational data, JSON documents, \
                        full-text search, time series. Start with Postgres. Migrate later if needed. \
                        Most 'NoSQL' choices are premature optimization.".to_string()
                    ),
                },
                DecisionOption {
                    label: "Documents without joins".to_string(),
                    description: "Self-contained records".to_string(),
                    principles: vec!["Simple Thing That Works".to_string()],
                    next: None,
                    recommendation: Some(
                        "STILL PROBABLY POSTGRES. It has JSONB. If you truly don't need transactions \
                        or ACID, MongoDB works. But most teams regret going NoSQL too early.".to_string()
                    ),
                },
                DecisionOption {
                    label: "Key-value or cache".to_string(),
                    description: "Simple lookups, high speed".to_string(),
                    principles: vec!["Right Tool for the Job".to_string()],
                    next: None,
                    recommendation: Some(
                        "REDIS for cache/session. But don't use it as primary DB. \
                        DynamoDB if AWS-native and need infinite scale.".to_string()
                    ),
                },
            ],
        },

        synergies: vec![],
        tensions: vec![],
        blind_spots: vec![],
        anti_patterns: vec![],
        success_rate: 0.0,
        times_used: 0,
    }
}

fn feature_prioritization() -> DecisionTemplate {
    DecisionTemplate {
        id: "feature-prioritization".to_string(),
        name: "Feature Prioritization".to_string(),
        description: "Which feature should we build next?".to_string(),
        domain: "product-management".to_string(),

        triggers: vec![TriggerPattern {
            keywords: vec![
                "prioritize".to_string(),
                "priority".to_string(),
                "roadmap".to_string(),
                "backlog".to_string(),
                "next".to_string(),
                "feature".to_string(),
                "first".to_string(),
                "order".to_string(),
                "sequence".to_string(),
                "rank".to_string(),
            ],
            phrases: vec![
                "which feature".to_string(),
                "what to build".to_string(),
                "build first".to_string(),
                "build next".to_string(),
                "feature priority".to_string(),
                "prioritize features".to_string(),
                "prioritization".to_string(),
                "which to do first".to_string(),
                "roadmap order".to_string(),
            ],
            min_confidence: 0.5,
        }],

        tree: DecisionTree {
            question: "What's the primary constraint you're optimizing for?".to_string(),
            help_text: Some("Every prioritization framework optimizes for something different".to_string()),
            options: vec![
                DecisionOption {
                    label: "REVENUE - maximize business value".to_string(),
                    description: "Need to generate income quickly".to_string(),
                    principles: vec!["80/20 Analysis".to_string(), "Opportunity Cost".to_string()],
                    recommendation: None,
                    next: Some(Box::new(DecisionTree {
                        question: "Do you have data on what users actually want?".to_string(),
                        help_text: None,
                        options: vec![
                            DecisionOption {
                                label: "Yes - user research exists".to_string(),
                                description: "Surveys, interviews, usage data available".to_string(),
                                principles: vec!["80/20 Analysis".to_string()],
                                next: None,
                                recommendation: Some(
                                    "WEIGHTED SCORING. Tim Ferriss 80/20: 20% of features drive 80% of value. \
                                    Score each feature: (Impact × Confidence) ÷ Effort. Build the highest \
                                    scorers first. Ignore pet features with low impact.".to_string()
                                ),
                            },
                            DecisionOption {
                                label: "No - we're guessing".to_string(),
                                description: "Limited data, going on intuition".to_string(),
                                principles: vec!["Falsifiability".to_string()],
                                next: None,
                                recommendation: Some(
                                    "VALIDATE FIRST. Karl Popper: 'A theory that can't be tested isn't useful.' \
                                    Don't build features on hunches. Instead: (1) Create a landing page for \
                                    top 3 ideas, (2) Measure clicks/signups, (3) Build what's validated. \
                                    Naval: 'Specific knowledge comes from experimentation, not planning.'".to_string()
                                ),
                            },
                        ],
                    })),
                },
                DecisionOption {
                    label: "RISK - reduce technical or business risk".to_string(),
                    description: "De-risk before investing more".to_string(),
                    principles: vec!["Antifragility".to_string(), "Optionality".to_string()],
                    next: None,
                    recommendation: Some(
                        "RISK-FIRST ORDERING. Taleb: 'What matters is the distribution of outcomes, \
                        not the average.' Build the riskiest features first - the ones that could \
                        fail catastrophically. If they fail, you want to know NOW, not after investing \
                        6 months. This preserves optionality.".to_string()
                    ),
                },
                DecisionOption {
                    label: "LEARNING - discover what works".to_string(),
                    description: "Early stage, need to learn fast".to_string(),
                    principles: vec!["Feedback Loops".to_string(), "Make it Work".to_string()],
                    next: None,
                    recommendation: Some(
                        "BUILD-MEASURE-LEARN CYCLE. Eric Ries: 'The unit of progress is validated learning.' \
                        Pick features that teach you something: (1) What's your riskiest assumption? \
                        (2) What's the smallest thing you can build to test it? (3) Build that. \
                        Karpathy: 'Don't be a hero' - start with the simplest version.".to_string()
                    ),
                },
                DecisionOption {
                    label: "DEPENDENCIES - technical sequencing".to_string(),
                    description: "Some features unlock others".to_string(),
                    principles: vec!["Incremental Migration".to_string()],
                    next: None,
                    recommendation: Some(
                        "DEPENDENCY GRAPH + CRITICAL PATH. Build what unblocks the most. \
                        If A enables B, C, and D, build A first even if it's lower value. \
                        Sam Newman: 'Optimize for flow, not individual task efficiency.' \
                        Use: (1) Map dependencies, (2) Find critical path, (3) Work backwards.".to_string()
                    ),
                },
            ],
        },

        synergies: vec![
            PrincipleSynergy {
                principles: vec!["80/20 Analysis".to_string(), "Opportunity Cost".to_string()],
                thinkers: vec!["Tim Ferriss".to_string(), "Naval Ravikant".to_string()],
                why: "Knowing what NOT to build is as valuable as knowing what to build".to_string(),
                combined_power: "Ruthlessly cut low-value features, focus on high-leverage work".to_string(),
            },
            PrincipleSynergy {
                principles: vec!["Antifragility".to_string(), "Optionality".to_string()],
                thinkers: vec!["Nassim Taleb".to_string()],
                why: "Early failures are cheap; late failures are expensive".to_string(),
                combined_power: "Fail fast on risky features, preserve resources for pivots".to_string(),
            },
        ],

        tensions: vec![
            PrincipleTension {
                principle_a: "80/20 Analysis (build high-value)".to_string(),
                principle_b: "Risk-First (build risky first)".to_string(),
                thinker_a: "Tim Ferriss".to_string(),
                thinker_b: "Nassim Taleb".to_string(),
                when_to_pick_a: "You have high confidence in what users want".to_string(),
                when_to_pick_b: "You're uncertain and need to validate assumptions".to_string(),
            },
        ],

        blind_spots: vec![
            BlindSpot {
                name: "Hidden Dependencies".to_string(),
                description: "Features often have dependencies you don't see until you build them".to_string(),
                check_question: "Have you mapped what each feature requires (APIs, infrastructure, data)?".to_string(),
                severity: BlindSpotSeverity::High,
            },
            BlindSpot {
                name: "Sunk Cost Fallacy".to_string(),
                description: "Prioritizing features you've already invested in, not best ones".to_string(),
                check_question: "If you were starting fresh today, would you still build this?".to_string(),
                severity: BlindSpotSeverity::Medium,
            },
            BlindSpot {
                name: "Loudest Voice Bias".to_string(),
                description: "Building what the loudest customer asks for".to_string(),
                check_question: "Is this request representative of your target market?".to_string(),
                severity: BlindSpotSeverity::High,
            },
        ],

        anti_patterns: vec![
            AntiPattern {
                name: "HIPPO (Highest Paid Person's Opinion)".to_string(),
                description: "Features prioritized by seniority, not evidence".to_string(),
                symptoms: vec![
                    "Priority changes when executives speak".to_string(),
                    "No one asks for data to justify features".to_string(),
                    "Roadmap reflects politics, not customer needs".to_string(),
                ],
                cure: "Require evidence for every feature. What problem? How many users? What impact?".to_string(),
                source_thinker: "Marty Cagan".to_string(),
            },
            AntiPattern {
                name: "Feature Factory".to_string(),
                description: "Measuring success by features shipped, not outcomes".to_string(),
                symptoms: vec![
                    "Team celebrates ship dates, not user metrics".to_string(),
                    "'How many features this sprint?' is the success metric".to_string(),
                    "No one revisits whether features worked".to_string(),
                ],
                cure: "Measure outcomes, not output. Did this feature move the needle?".to_string(),
                source_thinker: "John Cutler".to_string(),
            },
        ],

        success_rate: 0.0,
        times_used: 0,
    }
}

fn api_design() -> DecisionTemplate {
    DecisionTemplate {
        id: "api-design".to_string(),
        name: "API Design".to_string(),
        description: "What API style should we use?".to_string(),
        domain: "software-architecture".to_string(),

        triggers: vec![TriggerPattern {
            keywords: vec![
                "api".to_string(),
                "rest".to_string(),
                "graphql".to_string(),
                "grpc".to_string(),
                "endpoint".to_string(),
                "interface".to_string(),
            ],
            phrases: vec![
                "api design".to_string(),
                "rest or graphql".to_string(),
                "which api".to_string(),
                "api style".to_string(),
                "rest vs".to_string(),
                "graphql vs".to_string(),
            ],
            min_confidence: 0.6,
        }],

        tree: DecisionTree {
            question: "Who are the primary API consumers?".to_string(),
            help_text: Some("Different consumers have different needs".to_string()),
            options: vec![
                DecisionOption {
                    label: "Internal teams you control".to_string(),
                    description: "Your own frontend, mobile apps".to_string(),
                    principles: vec!["Simple Thing That Works".to_string()],
                    recommendation: None,
                    next: Some(Box::new(DecisionTree {
                        question: "How complex is your data graph?".to_string(),
                        help_text: None,
                        options: vec![
                            DecisionOption {
                                label: "Simple - few resources, simple relationships".to_string(),
                                description: "Users, posts, comments - straightforward".to_string(),
                                principles: vec!["YAGNI".to_string()],
                                next: None,
                                recommendation: Some(
                                    "REST. Simple, well-understood, great tooling. Don't add GraphQL \
                                    complexity for simple CRUD. Kent Beck: 'You Ain't Gonna Need It.'".to_string()
                                ),
                            },
                            DecisionOption {
                                label: "Complex - deep nesting, variable data needs".to_string(),
                                description: "Clients need different fields for different views".to_string(),
                                principles: vec!["Right Tool for the Job".to_string()],
                                next: None,
                                recommendation: Some(
                                    "GRAPHQL. Excels at: flexible queries, reducing over-fetching, \
                                    typed schema. But comes with complexity: caching harder, \
                                    N+1 queries, learning curve. Worth it for complex data needs.".to_string()
                                ),
                            },
                        ],
                    })),
                },
                DecisionOption {
                    label: "External developers / public API".to_string(),
                    description: "Third parties will integrate".to_string(),
                    principles: vec!["Worse is Better".to_string()],
                    next: None,
                    recommendation: Some(
                        "REST. Richard Gabriel: 'Worse is better' - simplicity wins adoption. \
                        REST is universally understood, works in any language, cacheable. \
                        GraphQL's learning curve hurts developer adoption for public APIs.".to_string()
                    ),
                },
                DecisionOption {
                    label: "Internal microservices".to_string(),
                    description: "Service-to-service communication".to_string(),
                    principles: vec!["Performance".to_string()],
                    next: None,
                    recommendation: Some(
                        "gRPC. Binary protocol, strongly typed, fast. Perfect for internal \
                        services where you control both ends. Use protocol buffers for \
                        schema evolution. REST for external, gRPC for internal.".to_string()
                    ),
                },
            ],
        },

        synergies: vec![],
        tensions: vec![
            PrincipleTension {
                principle_a: "GraphQL flexibility".to_string(),
                principle_b: "REST simplicity".to_string(),
                thinker_a: "Facebook Engineering".to_string(),
                thinker_b: "Roy Fielding".to_string(),
                when_to_pick_a: "Complex data needs, controlled consumers, type-safety valued".to_string(),
                when_to_pick_b: "Simple data, public API, maximum adoption needed".to_string(),
            },
        ],
        blind_spots: vec![
            BlindSpot {
                name: "Versioning Strategy".to_string(),
                description: "How will you evolve the API without breaking clients?".to_string(),
                check_question: "Do you have a plan for: URL versioning, header versioning, or additive changes only?".to_string(),
                severity: BlindSpotSeverity::High,
            },
        ],
        anti_patterns: vec![
            AntiPattern {
                name: "GraphQL for Simple CRUD".to_string(),
                description: "Using GraphQL when REST would suffice".to_string(),
                symptoms: vec![
                    "All queries fetch the same fields".to_string(),
                    "No nested relationships".to_string(),
                    "Added complexity for no benefit".to_string(),
                ],
                cure: "Start with REST. Add GraphQL only when REST becomes limiting.".to_string(),
                source_thinker: "Industry Wisdom".to_string(),
            },
        ],
        success_rate: 0.0,
        times_used: 0,
    }
}

fn testing_strategy() -> DecisionTemplate {
    DecisionTemplate {
        id: "testing-strategy".to_string(),
        name: "Testing Strategy".to_string(),
        description: "How should we approach testing?".to_string(),
        domain: "software-engineering".to_string(),

        triggers: vec![TriggerPattern {
            keywords: vec![
                "test".to_string(),
                "tests".to_string(),
                "testing".to_string(),
                "tdd".to_string(),
                "unit".to_string(),
                "integration".to_string(),
                "e2e".to_string(),
                "coverage".to_string(),
                "spec".to_string(),
                "specs".to_string(),
                "verify".to_string(),
                "validate".to_string(),
            ],
            phrases: vec![
                "testing strategy".to_string(),
                "how to test".to_string(),
                "how should we test".to_string(),
                "what to test".to_string(),
                "test coverage".to_string(),
                "unit vs integration".to_string(),
                "should we test".to_string(),
                "approach testing".to_string(),
                "test this".to_string(),
            ],
            min_confidence: 0.5,
        }],

        tree: DecisionTree {
            question: "What's the biggest risk in your codebase?".to_string(),
            help_text: Some("Test where failure is most costly".to_string()),
            options: vec![
                DecisionOption {
                    label: "Complex business logic".to_string(),
                    description: "Calculations, state machines, rules".to_string(),
                    principles: vec!["Make it Work".to_string()],
                    next: None,
                    recommendation: Some(
                        "UNIT TESTS for logic. Kent Beck: 'Test the things that might break.' \
                        Pure functions with business logic deserve heavy unit testing. \
                        Fast, isolated, deterministic. Aim for: (1) All edge cases, \
                        (2) Boundary conditions, (3) Error paths."
                            .to_string(),
                    ),
                },
                DecisionOption {
                    label: "Integration points".to_string(),
                    description: "APIs, databases, external services".to_string(),
                    principles: vec!["Design for Failure".to_string()],
                    next: None,
                    recommendation: Some(
                        "INTEGRATION TESTS for boundaries. Sam Newman: 'Design for failure.' \
                        Test that: (1) API contracts hold, (2) Database queries work, \
                        (3) External services handle failures gracefully. Use: contract tests, \
                        test containers, recorded responses."
                            .to_string(),
                    ),
                },
                DecisionOption {
                    label: "User flows".to_string(),
                    description: "Critical paths through the app".to_string(),
                    principles: vec!["80/20 Analysis".to_string()],
                    next: None,
                    recommendation: Some(
                        "E2E TESTS for critical paths ONLY. Tim Ferriss 80/20: test the 20% of \
                        paths that matter most. E2E tests are slow and flaky - use sparingly. \
                        Test: (1) Login/signup, (2) Core transaction (purchase, submit), \
                        (3) Critical integrations. Not every page needs E2E."
                            .to_string(),
                    ),
                },
                DecisionOption {
                    label: "Fast-moving code".to_string(),
                    description: "Frequent changes, prototyping".to_string(),
                    principles: vec!["Antifragility".to_string()],
                    next: None,
                    recommendation: Some(
                        "MINIMAL TESTS + OBSERVABILITY. Taleb: 'Antifragile systems benefit from \
                        volatility.' For rapidly changing code, heavy tests become maintenance \
                        burden. Instead: (1) Test critical paths only, (2) Invest in monitoring, \
                        (3) Fast rollback capability. Detect and recover > prevent all failures."
                            .to_string(),
                    ),
                },
            ],
        },

        synergies: vec![PrincipleSynergy {
            principles: vec!["Make it Work".to_string(), "Make it Right".to_string()],
            thinkers: vec!["Kent Beck".to_string()],
            why: "Tests enable fearless refactoring".to_string(),
            combined_power: "Write just enough tests to refactor confidently".to_string(),
        }],
        tensions: vec![PrincipleTension {
            principle_a: "High Coverage".to_string(),
            principle_b: "Fast Iteration".to_string(),
            thinker_a: "Traditional TDD".to_string(),
            thinker_b: "Startup Velocity".to_string(),
            when_to_pick_a: "Stable, critical system where bugs are expensive".to_string(),
            when_to_pick_b: "Rapid exploration where test maintenance slows learning".to_string(),
        }],
        blind_spots: vec![
            BlindSpot {
                name: "Testing the Wrong Level".to_string(),
                description: "Too many E2E tests, not enough unit tests".to_string(),
                check_question: "Is your test pyramid inverted (slow tests > fast tests)?"
                    .to_string(),
                severity: BlindSpotSeverity::High,
            },
            BlindSpot {
                name: "Testing Implementation Details".to_string(),
                description: "Tests break on refactoring even when behavior unchanged".to_string(),
                check_question: "Do your tests specify WHAT not HOW?".to_string(),
                severity: BlindSpotSeverity::Medium,
            },
        ],
        anti_patterns: vec![AntiPattern {
            name: "100% Coverage Obsession".to_string(),
            description: "Testing everything equally regardless of risk".to_string(),
            symptoms: vec![
                "Tests for getters/setters".to_string(),
                "Mocking everything".to_string(),
                "Tests more complex than code they test".to_string(),
            ],
            cure: "Test behavior at boundaries, not implementation details.".to_string(),
            source_thinker: "Kent Beck".to_string(),
        }],
        success_rate: 0.0,
        times_used: 0,
    }
}

fn performance_optimization() -> DecisionTemplate {
    DecisionTemplate {
        id: "performance-optimization".to_string(),
        name: "Performance Optimization".to_string(),
        description: "When and how should we optimize?".to_string(),
        domain: "software-engineering".to_string(),

        triggers: vec![TriggerPattern {
            keywords: vec![
                "performance".to_string(),
                "optimize".to_string(),
                "slow".to_string(),
                "fast".to_string(),
                "speed".to_string(),
                "latency".to_string(),
                "throughput".to_string(),
                "scalability".to_string(),
            ],
            phrases: vec![
                "make it faster".to_string(),
                "performance issue".to_string(),
                "too slow".to_string(),
                "optimize for".to_string(),
                "speed up".to_string(),
                "improve performance".to_string(),
            ],
            min_confidence: 0.6,
        }],

        tree: DecisionTree {
            question: "Is performance currently a REAL problem or ANTICIPATED?".to_string(),
            help_text: Some("'Premature optimization is the root of all evil' - Knuth".to_string()),
            options: vec![
                DecisionOption {
                    label: "REAL - users complaining, metrics prove it".to_string(),
                    description: "Measured latency, slow operations identified".to_string(),
                    principles: vec!["80/20 Analysis".to_string()],
                    recommendation: None,
                    next: Some(Box::new(DecisionTree {
                        question: "Have you profiled to identify the bottleneck?".to_string(),
                        help_text: None,
                        options: vec![
                            DecisionOption {
                                label: "Yes - I know exactly what's slow".to_string(),
                                description: "Profiler/APM pointed to specific code".to_string(),
                                principles: vec!["Make it Fast".to_string()],
                                next: None,
                                recommendation: Some(
                                    "OPTIMIZE THE BOTTLENECK. Kent Beck: 'Make it fast' - but only after \
                                    'make it work' and 'make it right.' 80/20 rule: 20% of code causes \
                                    80% of slowness. Fix that 20%. Common fixes: (1) Add caching, \
                                    (2) Batch operations, (3) Add indexes, (4) Reduce N+1 queries.".to_string()
                                ),
                            },
                            DecisionOption {
                                label: "No - it just feels slow".to_string(),
                                description: "No profiling data yet".to_string(),
                                principles: vec!["Measure First".to_string()],
                                next: None,
                                recommendation: Some(
                                    "PROFILE BEFORE OPTIMIZING. Donald Knuth: 'Premature optimization is \
                                    the root of all evil.' Your intuition about what's slow is usually wrong. \
                                    Steps: (1) Add APM/profiling, (2) Find actual bottleneck, (3) Measure \
                                    before AND after, (4) Optimize only the hot path.".to_string()
                                ),
                            },
                        ],
                    })),
                },
                DecisionOption {
                    label: "ANTICIPATED - might be slow at scale".to_string(),
                    description: "Works fine now but worried about future".to_string(),
                    principles: vec!["YAGNI".to_string(), "Simple Thing That Works".to_string()],
                    next: None,
                    recommendation: Some(
                        "DON'T OPTIMIZE YET. Kent Beck YAGNI: 'You Ain't Gonna Need It.' \
                        Karpathy: 'Don't be a hero' - solve today's problems. Premature optimization: \
                        (1) Adds complexity, (2) Makes code harder to change, (3) Often wrong about \
                        what will be slow. Instead: build simply, add monitoring, optimize when data \
                        shows a real problem. Most apps never hit the scale they fear.".to_string()
                    ),
                },
            ],
        },

        synergies: vec![
            PrincipleSynergy {
                principles: vec!["Make it Work".to_string(), "Make it Right".to_string(), "Make it Fast".to_string()],
                thinkers: vec!["Kent Beck".to_string()],
                why: "Order matters - optimize after correctness".to_string(),
                combined_power: "Working code you can safely optimize".to_string(),
            },
        ],
        tensions: vec![
            PrincipleTension {
                principle_a: "Optimize Early".to_string(),
                principle_b: "YAGNI".to_string(),
                thinker_a: "Systems Engineers".to_string(),
                thinker_b: "Kent Beck".to_string(),
                when_to_pick_a: "Architecture decisions that are hard to change later (data models)".to_string(),
                when_to_pick_b: "Code that can be refactored easily when need arises".to_string(),
            },
        ],
        blind_spots: vec![
            BlindSpot {
                name: "Wrong Bottleneck".to_string(),
                description: "Optimizing code that isn't the actual problem".to_string(),
                check_question: "Have you profiled in production with real load?".to_string(),
                severity: BlindSpotSeverity::Critical,
            },
            BlindSpot {
                name: "Optimization at Wrong Level".to_string(),
                description: "Micro-optimizing when architecture is the problem".to_string(),
                check_question: "Is this an algorithm problem or a code efficiency problem?".to_string(),
                severity: BlindSpotSeverity::High,
            },
        ],
        anti_patterns: vec![
            AntiPattern {
                name: "Premature Optimization".to_string(),
                description: "Optimizing before knowing what's slow".to_string(),
                symptoms: vec![
                    "Complex code 'for performance' without benchmarks".to_string(),
                    "Caching everything 'just in case'".to_string(),
                    "Micro-optimizations in non-hot paths".to_string(),
                ],
                cure: "Profile first. Optimize second. Measure the improvement.".to_string(),
                source_thinker: "Donald Knuth".to_string(),
            },
        ],
        success_rate: 0.0,
        times_used: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_templates_returns_all_12() {
        let templates = get_templates();
        assert_eq!(templates.len(), 12);

        // Verify all have unique IDs
        let ids: Vec<_> = templates.iter().map(|t| &t.id).collect();
        let mut unique_ids = ids.clone();
        unique_ids.sort();
        unique_ids.dedup();
        assert_eq!(ids.len(), unique_ids.len(), "Template IDs should be unique");
    }

    #[test]
    fn test_get_templates_have_required_fields() {
        for template in get_templates() {
            assert!(!template.id.is_empty(), "Template ID should not be empty");
            assert!(
                !template.name.is_empty(),
                "Template name should not be empty"
            );
            assert!(
                !template.domain.is_empty(),
                "Template domain should not be empty"
            );
            assert!(
                !template.triggers.is_empty(),
                "Template should have triggers"
            );
            assert!(
                !template.tree.question.is_empty(),
                "Template tree should have a question"
            );
            assert!(
                !template.tree.options.is_empty(),
                "Template tree should have options"
            );
        }
    }

    #[test]
    fn test_match_templates_microservices() {
        let matches = match_templates("Should we use microservices or stay with a monolith?");
        assert!(!matches.is_empty(), "Should match microservices template");
        assert_eq!(matches[0].0.id, "monolith-vs-microservices");
    }

    #[test]
    fn test_match_templates_rewrite() {
        let matches = match_templates("Should we rewrite this legacy system from scratch?");
        assert!(!matches.is_empty(), "Should match rewrite template");
        assert_eq!(matches[0].0.id, "rewrite-vs-refactor");
    }

    #[test]
    fn test_match_templates_build_vs_buy() {
        let matches = match_templates("Should we build our own auth system or use a third party?");
        assert!(!matches.is_empty(), "Should match build vs buy template");
        assert_eq!(matches[0].0.id, "build-vs-buy");
    }

    #[test]
    fn test_match_templates_late_project() {
        let matches =
            match_templates("We're behind schedule, should we add more people to the team?");
        assert!(!matches.is_empty(), "Should match scale team template");
        assert_eq!(matches[0].0.id, "scale-team");
    }

    #[test]
    fn test_match_templates_testing() {
        let matches = match_templates("How should we approach testing this new feature?");
        assert!(!matches.is_empty(), "Should match testing template");
        assert_eq!(matches[0].0.id, "testing-strategy");
    }

    #[test]
    fn test_match_templates_performance() {
        let matches = match_templates("The app is too slow, how should we optimize performance?");
        assert!(!matches.is_empty(), "Should match performance template");
        assert_eq!(matches[0].0.id, "performance-optimization");
    }

    #[test]
    fn test_match_templates_prioritization() {
        let matches = match_templates("Which feature should we prioritize and build first?");
        assert!(!matches.is_empty(), "Should match prioritization template");
        assert_eq!(matches[0].0.id, "feature-prioritization");
    }

    #[test]
    fn test_match_templates_no_match() {
        let matches = match_templates("What should I have for lunch?");
        assert!(
            matches.is_empty(),
            "Random question should not match any template"
        );
    }

    #[test]
    fn test_match_templates_case_insensitive() {
        let matches_lower = match_templates("should we use MICROSERVICES?");
        let matches_upper = match_templates("SHOULD WE USE microservices?");
        assert!(!matches_lower.is_empty());
        assert!(!matches_upper.is_empty());
        assert_eq!(matches_lower[0].0.id, matches_upper[0].0.id);
    }

    #[test]
    fn test_match_templates_sorted_by_score() {
        // A question with multiple keyword matches should score higher
        let matches = match_templates("microservices vs monolith architecture service decompose");
        assert!(!matches.is_empty());
        // First match should have highest score
        if matches.len() > 1 {
            assert!(
                matches[0].1 >= matches[1].1,
                "Results should be sorted by score descending"
            );
        }
    }

    #[test]
    fn test_blind_spot_severity_values() {
        // Verify BlindSpotSeverity enum works correctly
        assert_eq!(
            serde_json::to_string(&BlindSpotSeverity::Critical).unwrap(),
            "\"critical\""
        );
        assert_eq!(
            serde_json::to_string(&BlindSpotSeverity::High).unwrap(),
            "\"high\""
        );
        assert_eq!(
            serde_json::to_string(&BlindSpotSeverity::Medium).unwrap(),
            "\"medium\""
        );
        assert_eq!(
            serde_json::to_string(&BlindSpotSeverity::Low).unwrap(),
            "\"low\""
        );
    }

    #[test]
    fn test_decision_tree_depth() {
        // Verify nested decision trees work
        let template = monolith_vs_microservices();
        let first_option = &template.tree.options[0];
        assert!(
            first_option.next.is_some(),
            "First option should have nested tree"
        );

        let nested = first_option.next.as_ref().unwrap();
        assert!(!nested.question.is_empty());
        assert!(!nested.options.is_empty());
    }

    #[test]
    fn test_template_serialization() {
        // Verify templates can be serialized to JSON
        let templates = get_templates();
        let json = serde_json::to_string(&templates);
        assert!(json.is_ok(), "Templates should serialize to JSON");

        // And deserialize back
        let json_str = json.unwrap();
        let parsed: Result<Vec<DecisionTemplate>, _> = serde_json::from_str(&json_str);
        assert!(parsed.is_ok(), "Templates should deserialize from JSON");
        assert_eq!(parsed.unwrap().len(), 12);
    }
}
