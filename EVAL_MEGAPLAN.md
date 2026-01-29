# 100minds: 1M Question Evaluation & Optimization Framework

## Vision: Surprise Ourselves with How Good This Gets

We're building the most scientifically rigorous evaluation system possible, using 2026 SOTA techniques from bandit optimization, generative AI, and neural retrieval.

**Target:** Run 1M+ synthetic questions → optimize until metrics shock us

---

## Phase 1: Synthetic Question Generation (100k-1M)

### 1.1 LLM-Generated Question Factory

Use Claude Haiku to generate diverse questions across ALL decision domains:

```rust
// Question templates per domain
const DOMAIN_TEMPLATES: &[(&str, &[&str])] = &[
    ("architecture", &[
        "Should we {action} our {system_type}?",
        "Is it time to {migrate_verb} from {tech_old} to {tech_new}?",
        "Our {component} is {problem}. What should we do?",
        "Should we adopt {pattern} for our {use_case}?",
    ]),
    ("scaling", &[
        "We need to handle {multiplier}x traffic. How?",
        "Should we scale {horizontal_or_vertical}?",
        "Is {scaling_pattern} right for our {workload}?",
    ]),
    // ... 20+ domains
];
```

**Diversity Axes:**
- Domain (architecture, scaling, testing, management, etc.)
- Complexity (1-5)
- Stakeholder (CTO, IC engineer, PM, founder)
- Company stage (startup, growth, enterprise)
- Urgency (exploration vs crisis mode)

**Target:** 100k questions covering parameter space exhaustively

### 1.2 Hard Negative Mining

Generate adversarial questions designed to trick the system:

```rust
pub fn generate_hard_negatives(principle: &Principle) -> Vec<String> {
    // Questions that SEEM related but shouldn't cite this principle
    let anti_questions = vec![
        format!("I love {} but should I use it everywhere?", principle.name),
        format!("What are the downsides of {}?", principle.name),
        // Questions where this principle is WRONG
    ];
    anti_questions
}
```

---

## Phase 2: BanditLP-Style Optimization

### 2.1 Neural Thompson Sampling

From ACM Web 2026: Neural TS with large-scale LP for constrained selection.

```rust
pub struct NeuralThompsonSampler {
    // Neural network produces embedding per principle
    embeddings: HashMap<String, Vec<f32>>,  // 256-dim

    // Per-principle posterior: N(μ, σ²)
    posterior_mean: HashMap<String, f64>,
    posterior_var: HashMap<String, f64>,

    // Contextual: per-(principle, domain) posteriors
    contextual_arms: HashMap<(String, String), BetaPosterior>,
}

impl NeuralThompsonSampler {
    pub fn sample(&self, question_embedding: &[f32], domain: &str) -> Vec<(String, f64)> {
        // 1. Compute similarity scores using neural embeddings
        // 2. Sample from posterior for each candidate
        // 3. Return ranked list
    }

    pub fn update(&mut self, principle_id: &str, domain: &str, reward: f64) {
        // Bayesian update of posterior parameters
    }
}
```

### 2.2 Feel-Good Thompson Sampling (FG-TS)

More aggressive exploration for undersampled arms:

```rust
pub fn feel_good_sample(alpha: f64, beta: f64, sample_count: u64) -> f64 {
    let base_sample = beta_sample(alpha, beta);

    // FG-TS: Add bonus for low-sample arms
    if sample_count < 100 {
        let exploration_bonus = 2.0 * (1000.0_f64.ln() / (sample_count as f64 + 1.0)).sqrt();
        base_sample + exploration_bonus
    } else {
        base_sample
    }
}
```

### 2.3 Constraint-Aware LP (Multi-Stakeholder)

Optimize for multiple objectives simultaneously:

```rust
pub struct MultiObjectiveOptimizer {
    objectives: Vec<Objective>,
    constraints: Vec<Constraint>,
}

pub enum Objective {
    Relevance { weight: f64 },
    Diversity { weight: f64, min_thinkers: usize },
    Coverage { weight: f64, required_domains: Vec<String> },
    Novelty { weight: f64 },  // Don't always cite same principles
}

pub enum Constraint {
    MaxPrinciplesPerResponse(usize),
    MinConfidence(f64),
    NoAntiPrinciples,
    RequireBothSides,  // FOR and AGAINST positions
}
```

---

## Phase 3: LLM-as-Judge Evaluation (Scalable)

### 3.1 Multi-Criteria Rubric

```rust
pub struct JudgeRubric {
    criteria: Vec<JudgeCriterion>,
}

pub struct JudgeCriterion {
    name: String,
    weight: f64,
    prompt_template: String,
    scale: RatingScale,
}

const RUBRIC: &[JudgeCriterion] = &[
    JudgeCriterion {
        name: "Relevance",
        weight: 0.30,
        prompt: "Do the cited principles directly address the question asked?",
        scale: Scale1to5,
    },
    JudgeCriterion {
        name: "Completeness",
        weight: 0.20,
        prompt: "Are important considerations missing? Rate 5 if comprehensive.",
        scale: Scale1to5,
    },
    JudgeCriterion {
        name: "Actionability",
        weight: 0.25,
        prompt: "Can the user immediately act on this advice?",
        scale: Scale1to5,
    },
    JudgeCriterion {
        name: "Balance",
        weight: 0.15,
        prompt: "Are both FOR and AGAINST positions genuinely opposed?",
        scale: Scale1to5,
    },
    JudgeCriterion {
        name: "Authority",
        weight: 0.10,
        prompt: "Are the cited thinkers credible for this domain?",
        scale: Scale1to5,
    },
];
```

### 3.2 Pairwise Preference Collection

More reliable than absolute scoring:

```rust
pub async fn pairwise_judge(
    question: &str,
    response_a: &CounselResponse,
    response_b: &CounselResponse,
    judge_model: &str,
) -> PairwiseResult {
    let prompt = format!(r#"
Question: {question}

Response A:
{response_a:?}

Response B:
{response_b:?}

Which response better helps the user make their decision?
Consider: relevance, actionability, balance, authority.

Output JSON: {{"winner": "A" | "B" | "tie", "confidence": 0.0-1.0, "reasoning": "..."}}
"#);

    // Call Haiku for cheap, fast judging
    call_llm(judge_model, &prompt).await
}
```

### 3.3 Synthetic Annotation Pipeline

Generate ground truth at scale using LLM consensus:

```rust
pub async fn annotate_scenario(question: &str) -> GroundTruth {
    // Use 3 different LLM judges
    let judges = ["claude-3-haiku", "gpt-4o-mini", "gemini-flash"];

    let mut all_principles: Vec<String> = vec![];
    let mut all_anti: Vec<String> = vec![];

    for judge in judges {
        let annotation = call_llm(judge, &format!(r#"
For this decision question, list:
1. Principles that SHOULD be cited (from software engineering wisdom)
2. Principles that would be WRONG to cite

Question: {question}

Output JSON: {{"expected": [...], "anti": [...]}}
"#)).await;

        all_principles.extend(annotation.expected);
        all_anti.extend(annotation.anti);
    }

    // Majority voting for consensus
    GroundTruth {
        expected: majority_vote(&all_principles, 2),  // 2+ judges agree
        anti: majority_vote(&all_anti, 2),
    }
}
```

---

## Phase 4: Bayesian Hyperparameter Optimization

### 4.1 Parameter Space

```rust
pub struct HyperparameterSpace {
    // Scoring weights
    fts_weight: Range<f64>,           // 0.0 - 2.0
    semantic_weight: Range<f64>,       // 0.0 - 1.0
    rrf_k: Range<u32>,                 // 10 - 100

    // Domain boost weights
    domain_boost_arch: Range<f64>,     // 0.0 - 50.0
    domain_boost_testing: Range<f64>,
    domain_boost_scaling: Range<f64>,

    // Thompson Sampling
    prior_alpha: Range<f64>,           // 0.1 - 10.0
    prior_beta: Range<f64>,
    ucb_exploration_c: Range<f64>,     // 0.1 - 5.0

    // Temporal decay
    decay_lambda: Range<f64>,          // 0.8 - 0.99
}
```

### 4.2 Bayesian Optimization Loop

```rust
pub async fn bayesian_optimize(
    space: &HyperparameterSpace,
    eval_function: impl Fn(&Hyperparameters) -> f64,
    n_iterations: usize,
) -> Hyperparameters {
    let mut gp = GaussianProcess::new();
    let mut best_params = Hyperparameters::default();
    let mut best_score = 0.0;

    for i in 0..n_iterations {
        // Sample next point using Expected Improvement
        let candidate = gp.suggest_next(space);

        // Evaluate on 1000-question sample
        let score = eval_function(&candidate);

        // Update GP posterior
        gp.observe(candidate, score);

        if score > best_score {
            best_score = score;
            best_params = candidate;
            println!("Iteration {}: New best! Score = {:.4}", i, score);
        }
    }

    best_params
}
```

---

## Phase 5: Adversarial Robustness Testing

### 5.1 Question Perturbations

```rust
pub fn generate_perturbations(question: &str) -> Vec<String> {
    vec![
        // Typos
        add_typos(question),
        // Synonyms
        replace_with_synonyms(question),
        // Paraphrase
        paraphrase_llm(question),
        // Negation
        negate_question(question),
        // Add noise
        add_irrelevant_context(question),
        // Simplify
        simplify_to_keywords(question),
    ]
}
```

### 5.2 Distribution Shift Testing

```rust
pub fn test_distribution_shift(engine: &CounselEngine) -> ShiftResults {
    // Test on questions from different eras
    let eras = vec![
        ("2015_questions", load_2015_style_questions()),
        ("2020_questions", load_2020_style_questions()),
        ("2025_questions", load_2025_style_questions()),
    ];

    // Test on different company stages
    let stages = vec![
        ("startup", load_startup_questions()),
        ("growth", load_growth_questions()),
        ("enterprise", load_enterprise_questions()),
    ];

    // Measure performance degradation
    measure_across_distributions(&engine, eras, stages)
}
```

---

## Phase 6: Continuous Improvement Loop

### 6.1 Automated Discovery

```rust
pub struct ImprovementDiscovery {
    // Find principles that are NEVER selected
    orphan_threshold: u64,  // <10 samples

    // Find principles that are ALWAYS wrong
    failure_threshold: f64,  // success rate < 20%

    // Find missing coverage
    domain_coverage_min: f64,  // Every domain needs >80% coverage
}

pub async fn discover_improvements(
    results: &EvalResults,
) -> Vec<Improvement> {
    let mut improvements = vec![];

    // 1. Orphan principles - need better keywords?
    for orphan in find_orphans(&results) {
        improvements.push(Improvement::AddKeywords(orphan));
    }

    // 2. Always-wrong principles - remove or restrict?
    for failure in find_consistent_failures(&results) {
        improvements.push(Improvement::AddAntiPattern(failure));
    }

    // 3. Missing coverage - need new thinkers?
    for gap in find_coverage_gaps(&results) {
        improvements.push(Improvement::SuggestNewThinker(gap));
    }

    // 4. Scoring imbalances - adjust weights?
    for imbalance in find_scoring_imbalances(&results) {
        improvements.push(Improvement::AdjustWeight(imbalance));
    }

    improvements
}
```

### 6.2 Automated Fix Application

```rust
pub async fn apply_improvements(
    improvements: &[Improvement],
    engine: &mut CounselEngine,
) -> ApplyResults {
    for improvement in improvements {
        match improvement {
            Improvement::AddKeywords(principle_id, new_keywords) => {
                update_principle_keywords(principle_id, new_keywords);
            }
            Improvement::AdjustWeight(param, delta) => {
                adjust_scoring_weight(param, delta);
            }
            Improvement::AddAntiPattern(principle_id, pattern) => {
                add_anti_pattern_rule(principle_id, pattern);
            }
            // ...
        }
    }

    // Re-evaluate after fixes
    let new_results = run_benchmark(engine).await;

    ApplyResults {
        improvements_applied: improvements.len(),
        metrics_before: old_results.aggregate,
        metrics_after: new_results.aggregate,
    }
}
```

---

## Implementation Phases

### Day 1: Question Generation
- [ ] Build domain-specific template system
- [ ] Generate 10k questions using Claude
- [ ] Validate coverage across parameter space
- [ ] Generate hard negatives for each principle

### Day 2: Neural Thompson Sampling
- [ ] Implement Feel-Good TS with exploration bonus
- [ ] Add multi-objective scoring (relevance + diversity + coverage)
- [ ] Build contextual arm tracking per domain
- [ ] Implement constraint-aware selection

### Day 3: LLM Judge Pipeline
- [ ] Implement multi-criteria rubric evaluation
- [ ] Build pairwise comparison system
- [ ] Create synthetic annotation consensus pipeline
- [ ] Add cost-tracking for API calls

### Day 4: Bayesian Optimization
- [ ] Define hyperparameter space
- [ ] Implement GP-based optimization loop
- [ ] Run 100 iterations on 1k-question sample
- [ ] Apply best parameters

### Day 5: Adversarial Testing
- [ ] Generate question perturbations
- [ ] Test distribution shift robustness
- [ ] Find and fix failure modes
- [ ] Document edge cases

### Day 6: Continuous Loop
- [ ] Build automated improvement discovery
- [ ] Implement auto-fix application
- [ ] Run full 100k evaluation
- [ ] Generate comprehensive report

---

## Success Metrics

| Metric | Current | Phase 1 Target | Final Target |
|--------|---------|----------------|--------------|
| P@1 | 8% | 25% | 50%+ |
| P@3 | 15% | 40% | 70%+ |
| Recall | 20% | 50% | 80%+ |
| NDCG | ~0.15 | 0.40 | 0.75+ |
| Anti-principle rate | Unknown | <10% | <2% |
| Coverage (orphan %) | ~50% | <30% | <10% |

---

## CLI Commands

```bash
# Generate synthetic questions
100minds --eval generate-questions --count 100000 --output eval/synthetic/

# Run full benchmark
100minds --eval benchmark --questions eval/synthetic/*.json

# Run LLM judge evaluation
100minds --eval judge --sample 1000 --model haiku

# Run Bayesian optimization
100minds --eval optimize --iterations 100 --sample-size 1000

# Find improvements
100minds --eval discover-improvements --threshold 0.05

# Apply improvements automatically
100minds --eval apply-improvements --dry-run

# Full automated loop
100minds --eval auto-improve --iterations 10 --questions 10000
```

---

## Sources

- [BanditLP: Large-Scale Stochastic Optimization for Personalized Recommendations](https://arxiv.org/html/2601.15552) (ACM Web 2026)
- [Feel-Good Thompson Sampling for Contextual Bandits](https://arxiv.org/html/2507.15290)
- [DeepEval: The LLM Evaluation Framework](https://github.com/confident-ai/deepeval)
- [Scalable and Interpretable Contextual Bandits](https://link.springer.com/chapter/10.1007/978-3-032-03769-5_39)
- [LLM as a Judge: 2026 Guide](https://labelyourdata.com/articles/llm-as-a-judge)
- [Generative Recommendation: A Survey](https://www.techrxiv.org/doi/full/10.36227/techrxiv.176523089.94266134/v2)
