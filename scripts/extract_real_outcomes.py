#!/usr/bin/env python3
"""
Real Outcome Extraction Pipeline for Neural Bandit Training

Extracts recorded decision outcomes from 100minds.db and transforms them
into training data format. Supports hybrid loading with synthetic data.

Usage:
    # Extract real outcomes only
    python scripts/extract_real_outcomes.py --db data/100minds.db --output training_data/real_outcomes.jsonl

    # Hybrid mode: mix real (30%) with synthetic (70%)
    python scripts/extract_real_outcomes.py --db data/100minds.db --synthetic training_data/synthetic.jsonl --ratio 0.3 --output training_data/hybrid.jsonl

    # Check outcome availability
    python scripts/extract_real_outcomes.py --db data/100minds.db --check
"""

import argparse
import json
import random
import sqlite3
import sys
from dataclasses import dataclass, asdict
from datetime import datetime
from pathlib import Path
from typing import Dict, List, Optional, Tuple


@dataclass
class ContextFeatures:
    """Contextual features for neural network training."""
    stakeholder: str
    company_stage: str
    urgency: str
    domain_match: bool
    total_principles_selected: int
    is_for_position: bool


@dataclass
class TrainingExample:
    """A single training example for the neural bandit."""
    id: str
    question: str
    domain: str
    difficulty: int
    principle_id: str
    principle_name: str
    thinker_id: str
    position_rank: int
    confidence: float
    success: float
    reasoning: str
    context_features: ContextFeatures

    def to_dict(self) -> dict:
        d = asdict(self)
        d['context_features'] = asdict(self.context_features)
        return d


# Domain detection patterns
DOMAIN_PATTERNS = {
    'architecture': ['microservice', 'monolith', 'rewrite', 'refactor', 'architecture', 'decompose', 'service'],
    'scaling': ['scale', 'traffic', 'load', 'horizontal', 'vertical', 'performance', 'bottleneck'],
    'testing': ['test', 'tdd', 'coverage', 'unit', 'integration', 'flaky', 'regression'],
    'management': ['team', 'engineer', 'hire', 'process', 'agile', 'sprint', 'deadline'],
    'security': ['security', 'vulnerability', 'auth', 'encrypt', 'permission', 'access'],
    'database': ['database', 'sql', 'query', 'index', 'migration', 'schema'],
    'devops': ['deploy', 'ci/cd', 'pipeline', 'docker', 'kubernetes', 'container'],
    'tech-debt': ['debt', 'legacy', 'cleanup', 'modernize', 'upgrade'],
    'performance': ['slow', 'latency', 'throughput', 'optimize', 'cache'],
    'build-vs-buy': ['build', 'buy', 'vendor', 'saas', 'custom'],
}

# Stakeholder detection
STAKEHOLDER_KEYWORDS = {
    'CTO': ['cto', 'chief technology', 'technical leader'],
    'Engineering Manager': ['manager', 'em', 'team lead'],
    'Senior Engineer': ['senior', 'staff', 'principal'],
    'Tech Lead': ['tech lead', 'architect'],
    'Product Manager': ['product manager', 'pm', 'product'],
    'Founder': ['founder', 'ceo', 'startup'],
    'DevOps Engineer': ['devops', 'sre', 'platform'],
}

# Urgency detection
URGENCY_KEYWORDS = {
    'crisis': ['urgent', 'asap', 'emergency', 'production down', 'critical'],
    'implementation': ['implementing', 'building', 'working on'],
    'planning': ['planning', 'considering', 'thinking about'],
    'exploration': ['exploring', 'researching', 'evaluating'],
    'post-mortem': ['incident', 'post-mortem', 'root cause', 'failure'],
}

# Company stage detection
STAGE_KEYWORDS = {
    'early-startup': ['startup', 'mvp', 'bootstrap', 'seed'],
    'growth-stage': ['growing', 'growth', 'series a', 'scaling'],
    'scale-up': ['scale', 'series b', 'series c', 'expanding'],
    'mature-company': ['mature', 'established', 'enterprise'],
    'enterprise': ['enterprise', 'fortune', 'large'],
}


def detect_domain(question: str) -> str:
    """Detect the domain from question text."""
    question_lower = question.lower()
    scores = {}

    for domain, patterns in DOMAIN_PATTERNS.items():
        score = sum(1 for p in patterns if p in question_lower)
        if score > 0:
            scores[domain] = score

    if scores:
        return max(scores, key=scores.get)
    return 'architecture'  # Default


def detect_stakeholder(context_json: dict) -> str:
    """Detect stakeholder from context."""
    if not context_json:
        return 'Tech Lead'

    context_str = json.dumps(context_json).lower()

    for stakeholder, keywords in STAKEHOLDER_KEYWORDS.items():
        if any(kw in context_str for kw in keywords):
            return stakeholder

    return 'Senior Engineer'


def detect_urgency(question: str, context_json: dict) -> str:
    """Detect urgency level."""
    text = question.lower()
    if context_json:
        text += ' ' + json.dumps(context_json).lower()

    for urgency, keywords in URGENCY_KEYWORDS.items():
        if any(kw in text for kw in keywords):
            return urgency

    return 'planning'


def detect_company_stage(context_json: dict) -> str:
    """Detect company stage from context."""
    if not context_json:
        return 'growth-stage'

    context_str = json.dumps(context_json).lower()

    for stage, keywords in STAGE_KEYWORDS.items():
        if any(kw in context_str for kw in keywords):
            return stage

    return 'growth-stage'


def estimate_difficulty(question: str, counsel_json: dict) -> int:
    """Estimate difficulty from question complexity and counsel response."""
    # Base difficulty from question length and complexity
    word_count = len(question.split())

    if word_count < 20:
        base = 1
    elif word_count < 40:
        base = 2
    elif word_count < 60:
        base = 3
    elif word_count < 100:
        base = 4
    else:
        base = 5

    # Adjust based on counsel complexity
    if counsel_json:
        for_count = len(counsel_json.get('for', []))
        against_count = len(counsel_json.get('against', []))

        if for_count + against_count >= 8:
            base = min(5, base + 1)

    return base


def extract_principles_from_counsel(counsel_json: dict) -> List[Tuple[str, str, str, float, bool]]:
    """
    Extract principles from counsel JSON.
    Returns list of (principle_id, principle_name, thinker_id, confidence, is_for).
    """
    principles = []

    for position in ['for', 'against']:
        is_for = position == 'for'
        entries = counsel_json.get(position, [])

        for idx, entry in enumerate(entries):
            # Handle different counsel response formats
            if isinstance(entry, dict):
                principle_id = entry.get('id', entry.get('principle_id', f'p-{idx}'))
                principle_name = entry.get('principle', entry.get('name', 'Unknown'))
                thinker = entry.get('thinker', entry.get('source', 'Unknown'))
                confidence = entry.get('confidence', entry.get('score', 0.5))
            else:
                continue

            # Normalize thinker to ID format
            thinker_id = thinker.lower().replace(' ', '-').replace("'", '')

            principles.append((principle_id, principle_name, thinker_id, confidence, is_for))

    return principles


def decision_to_examples(
    decision_id: str,
    question: str,
    context_json: dict,
    counsel_json: dict,
    outcome_success: bool,
    outcome_notes: str
) -> List[TrainingExample]:
    """
    Convert a single decision with outcome to training examples.

    Each principle mentioned in the counsel becomes a training example.
    Success label propagates from the decision outcome.
    """
    examples = []

    # Detect context features
    domain = detect_domain(question)
    stakeholder = detect_stakeholder(context_json)
    urgency = detect_urgency(question, context_json)
    company_stage = detect_company_stage(context_json)
    difficulty = estimate_difficulty(question, counsel_json)

    # Extract principles
    principles = extract_principles_from_counsel(counsel_json)

    if not principles:
        return examples

    total_principles = len(principles)

    for rank, (principle_id, principle_name, thinker_id, confidence, is_for) in enumerate(principles):
        # Domain match check
        principle_lower = principle_name.lower()
        domain_match = any(
            pattern in principle_lower
            for pattern in DOMAIN_PATTERNS.get(domain, [])
        )

        # Success label logic:
        # - If outcome was success and principle was FOR: success
        # - If outcome was success and principle was AGAINST: partial success (0.3)
        # - If outcome was failure and principle was FOR: failure
        # - If outcome was failure and principle was AGAINST: partial success (0.6)
        if outcome_success:
            success = 1.0 if is_for else 0.3
        else:
            success = 0.0 if is_for else 0.6

        # Build reasoning
        position_str = "FOR" if is_for else "AGAINST"
        outcome_str = "succeeded" if outcome_success else "failed"
        reasoning = f"Decision {outcome_str}. Principle was in {position_str} position."
        if outcome_notes:
            reasoning += f" Notes: {outcome_notes[:100]}"

        context_features = ContextFeatures(
            stakeholder=stakeholder,
            company_stage=company_stage,
            urgency=urgency,
            domain_match=domain_match,
            total_principles_selected=total_principles,
            is_for_position=is_for,
        )

        example = TrainingExample(
            id=f"{decision_id}-{principle_id}",
            question=question,
            domain=domain,
            difficulty=difficulty,
            principle_id=principle_id,
            principle_name=principle_name,
            thinker_id=thinker_id,
            position_rank=rank,
            confidence=confidence,
            success=success,
            reasoning=reasoning,
            context_features=context_features,
        )

        examples.append(example)

    return examples


def extract_from_database(db_path: str) -> List[TrainingExample]:
    """Extract training examples from decisions with recorded outcomes."""
    conn = sqlite3.connect(db_path)
    conn.row_factory = sqlite3.Row

    cursor = conn.execute("""
        SELECT
            id,
            question,
            context_json,
            counsel_json,
            outcome_success,
            outcome_notes
        FROM decisions
        WHERE outcome_success IS NOT NULL
        ORDER BY created_at DESC
    """)

    all_examples = []
    decisions_processed = 0

    for row in cursor:
        try:
            context_json = json.loads(row['context_json']) if row['context_json'] else {}
            counsel_json = json.loads(row['counsel_json']) if row['counsel_json'] else {}

            examples = decision_to_examples(
                decision_id=row['id'],
                question=row['question'],
                context_json=context_json,
                counsel_json=counsel_json,
                outcome_success=bool(row['outcome_success']),
                outcome_notes=row['outcome_notes'] or '',
            )

            all_examples.extend(examples)
            decisions_processed += 1

        except (json.JSONDecodeError, KeyError) as e:
            print(f"Warning: Skipping decision {row['id']}: {e}", file=sys.stderr)
            continue

    conn.close()

    print(f"Extracted {len(all_examples)} examples from {decisions_processed} decisions")
    return all_examples


def load_synthetic_data(path: str) -> List[dict]:
    """Load synthetic training data from JSONL file."""
    examples = []
    with open(path, 'r') as f:
        for line in f:
            examples.append(json.loads(line.strip()))
    return examples


def create_hybrid_dataset(
    real_examples: List[TrainingExample],
    synthetic_path: str,
    real_ratio: float,
    total_size: int = 10000,
) -> List[dict]:
    """
    Create hybrid dataset mixing real and synthetic data.

    Args:
        real_examples: Extracted real outcome examples
        synthetic_path: Path to synthetic JSONL
        real_ratio: Ratio of real data (0.0-1.0)
        total_size: Target total dataset size

    Returns:
        Mixed dataset as list of dicts
    """
    synthetic_data = load_synthetic_data(synthetic_path)

    # Calculate counts
    target_real = int(total_size * real_ratio)
    target_synthetic = total_size - target_real

    # Handle cold-start: if not enough real data, adjust ratio
    actual_real = min(target_real, len(real_examples))
    actual_synthetic = total_size - actual_real

    if actual_real < target_real:
        print(f"Warning: Only {len(real_examples)} real examples available, "
              f"using {actual_real} (requested {target_real})")

    # Sample data
    if actual_real > 0:
        if len(real_examples) <= actual_real:
            sampled_real = [e.to_dict() for e in real_examples]
        else:
            sampled_real = [e.to_dict() for e in random.sample(real_examples, actual_real)]
    else:
        sampled_real = []

    sampled_synthetic = random.sample(synthetic_data, min(actual_synthetic, len(synthetic_data)))

    # Combine and shuffle
    combined = sampled_real + sampled_synthetic
    random.shuffle(combined)

    print(f"Hybrid dataset: {len(sampled_real)} real + {len(sampled_synthetic)} synthetic = {len(combined)} total")
    return combined


def check_outcome_availability(db_path: str) -> dict:
    """Check outcome data availability in database."""
    conn = sqlite3.connect(db_path)

    stats = {}

    # Check if table exists
    table_exists = conn.execute(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='decisions'"
    ).fetchone()[0]

    if not table_exists:
        stats['total_decisions'] = 0
        stats['with_outcomes'] = 0
        stats['success_count'] = 0
        stats['success_rate'] = None
        stats['recent_outcomes'] = 0
        stats['schema_initialized'] = False
        conn.close()
        return stats

    stats['schema_initialized'] = True

    # Total decisions
    stats['total_decisions'] = conn.execute(
        "SELECT COUNT(*) FROM decisions"
    ).fetchone()[0]

    # With outcomes
    stats['with_outcomes'] = conn.execute(
        "SELECT COUNT(*) FROM decisions WHERE outcome_success IS NOT NULL"
    ).fetchone()[0]

    # Success rate
    if stats['with_outcomes'] > 0:
        stats['success_count'] = conn.execute(
            "SELECT COUNT(*) FROM decisions WHERE outcome_success = 1"
        ).fetchone()[0]
        stats['success_rate'] = stats['success_count'] / stats['with_outcomes']
    else:
        stats['success_count'] = 0
        stats['success_rate'] = None

    # Recent outcomes (last 30 days)
    stats['recent_outcomes'] = conn.execute("""
        SELECT COUNT(*) FROM decisions
        WHERE outcome_success IS NOT NULL
        AND outcome_recorded_at >= datetime('now', '-30 days')
    """).fetchone()[0]

    conn.close()
    return stats


def main():
    parser = argparse.ArgumentParser(
        description="Extract real outcomes for neural bandit training"
    )
    parser.add_argument(
        '--db', type=str, default='data/100minds.db',
        help='Path to 100minds database'
    )
    parser.add_argument(
        '--output', type=str,
        help='Output JSONL file path'
    )
    parser.add_argument(
        '--synthetic', type=str,
        help='Path to synthetic data for hybrid mode'
    )
    parser.add_argument(
        '--ratio', type=float, default=0.3,
        help='Ratio of real data in hybrid mode (default: 0.3)'
    )
    parser.add_argument(
        '--total', type=int, default=10000,
        help='Total examples in hybrid mode (default: 10000)'
    )
    parser.add_argument(
        '--check', action='store_true',
        help='Check outcome data availability'
    )
    parser.add_argument(
        '--seed', type=int, default=42,
        help='Random seed for reproducibility'
    )

    args = parser.parse_args()
    random.seed(args.seed)

    # Check database exists
    if not Path(args.db).exists():
        print(f"Error: Database not found: {args.db}", file=sys.stderr)
        sys.exit(1)

    # Check mode
    if args.check:
        stats = check_outcome_availability(args.db)
        print("\n=== 100minds Outcome Data Availability ===")
        print(f"Total decisions:    {stats['total_decisions']:,}")
        print(f"With outcomes:      {stats['with_outcomes']:,}")
        if stats['success_rate'] is not None:
            print(f"Success rate:       {stats['success_rate']:.1%}")
        print(f"Recent (30d):       {stats['recent_outcomes']:,}")

        if not stats.get('schema_initialized', True):
            print("\n[NOT INITIALIZED] Database schema not created yet.")
            print("Run a counsel query first: 100minds counsel 'Should we...?'")
        elif stats['with_outcomes'] == 0:
            print("\n[COLD START] No recorded outcomes yet.")
            print("Record outcomes using: 100minds record-outcome <decision-id> --success")
        elif stats['with_outcomes'] < 100:
            print(f"\n[LOW DATA] Only {stats['with_outcomes']} outcomes recorded.")
            print("Recommend hybrid training with synthetic data.")
        else:
            print(f"\n[READY] {stats['with_outcomes']} outcomes available for training.")

        return

    # Require output path
    if not args.output:
        print("Error: --output required", file=sys.stderr)
        sys.exit(1)

    # Extract real outcomes
    real_examples = extract_from_database(args.db)

    # Hybrid or pure real mode
    if args.synthetic:
        if not Path(args.synthetic).exists():
            print(f"Error: Synthetic data not found: {args.synthetic}", file=sys.stderr)
            sys.exit(1)

        dataset = create_hybrid_dataset(
            real_examples,
            args.synthetic,
            args.ratio,
            args.total,
        )
    else:
        dataset = [e.to_dict() for e in real_examples]

    # Write output
    output_path = Path(args.output)
    output_path.parent.mkdir(parents=True, exist_ok=True)

    with open(output_path, 'w') as f:
        for example in dataset:
            f.write(json.dumps(example) + '\n')

    print(f"Wrote {len(dataset)} examples to {args.output}")

    # Summary statistics
    if dataset:
        success_count = sum(1 for e in dataset if e['success'] > 0.5)
        print(f"Success rate: {success_count/len(dataset):.1%}")


if __name__ == '__main__':
    main()
