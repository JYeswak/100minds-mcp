#!/usr/bin/env python3
"""
V4 Swarm Drift Probe: Multi-Agent Sync Under Adversarial Drift

Tests V4 SwarmPosterior against:
1. 25% drift rate (agents see different distributions)
2. Sync failure simulation (what if deltas don't arrive?)
3. Cold-start + drift combination (worst case)

Target: Validate +28% collective gains from shared posteriors

Usage:
    python scripts/probe_v4_swarm.py
"""

import math
import random
from dataclasses import dataclass, field
from typing import List, Dict, Tuple
from collections import defaultdict

# Constants
DOMAINS = ['architecture', 'testing', 'scaling', 'management', 'security', 'performance']
PRINCIPLES = [
    ('yagni', 'architecture'),
    ('kiss', 'architecture'),
    ('tdd', 'testing'),
    ('red-green-refactor', 'testing'),
    ('horizontal-scale', 'scaling'),
    ('brooks-law', 'management'),
    ('defense-in-depth', 'security'),
    ('measure-first', 'performance'),
]


@dataclass
class PosteriorState:
    """Beta distribution posterior for a principle-domain pair."""
    alpha: float = 1.0
    beta: float = 1.0
    observations: int = 0

    def update(self, success: bool):
        if success:
            self.alpha += 1.0
        else:
            self.beta += 1.0
        self.observations += 1

    def mean(self) -> float:
        return self.alpha / (self.alpha + self.beta)

    def variance(self) -> float:
        s = self.alpha + self.beta
        return (self.alpha * self.beta) / (s * s * (s + 1.0))

    def decay(self, factor: float = 0.95):
        """Apply forgetting factor."""
        effective = self.observations * factor
        total = self.alpha + self.beta
        ratio = self.alpha / total
        self.alpha = 1.0 + ratio * effective
        self.beta = 1.0 + (1.0 - ratio) * effective
        self.observations = int(effective)


@dataclass
class PosteriorDelta:
    """Delta for sharing between agents."""
    agent_id: str
    key: str
    alpha_delta: float
    beta_delta: float
    confidence: float


@dataclass
class Agent:
    """Simulated swarm agent with local posteriors."""
    agent_id: str
    posteriors: Dict[str, PosteriorState] = field(default_factory=dict)
    pending_deltas: List[PosteriorDelta] = field(default_factory=list)
    sync_interval: int = 10
    outcomes_since_sync: int = 0
    forgetting_factor: float = 0.95

    def record_outcome(self, principle: str, domain: str, success: bool):
        key = f"{principle}:{domain}"
        if key not in self.posteriors:
            self.posteriors[key] = PosteriorState()

        old_alpha = self.posteriors[key].alpha
        old_beta = self.posteriors[key].beta

        self.posteriors[key].update(success)
        self.outcomes_since_sync += 1

        # Create delta
        delta = PosteriorDelta(
            agent_id=self.agent_id,
            key=key,
            alpha_delta=self.posteriors[key].alpha - old_alpha,
            beta_delta=self.posteriors[key].beta - old_beta,
            confidence=1.0 / (1.0 + self.posteriors[key].variance()),
        )
        self.pending_deltas.append(delta)

    def needs_sync(self) -> bool:
        return self.outcomes_since_sync >= self.sync_interval

    def get_deltas(self) -> List[PosteriorDelta]:
        deltas = self.pending_deltas.copy()
        self.pending_deltas.clear()
        self.outcomes_since_sync = 0
        return deltas

    def apply_peer_deltas(self, deltas: List[PosteriorDelta]):
        for delta in deltas:
            if delta.agent_id == self.agent_id:
                continue

            if delta.key not in self.posteriors:
                self.posteriors[delta.key] = PosteriorState()

            posterior = self.posteriors[delta.key]
            local_conf = 1.0 / (1.0 + posterior.variance())
            peer_weight = delta.confidence / (local_conf + delta.confidence)

            posterior.alpha += delta.alpha_delta * peer_weight
            posterior.beta += delta.beta_delta * peer_weight

    def apply_decay(self):
        for posterior in self.posteriors.values():
            posterior.decay(self.forgetting_factor)

    def get_probability(self, principle: str, domain: str) -> float:
        key = f"{principle}:{domain}"
        if key in self.posteriors:
            return self.posteriors[key].mean()
        return 0.5

    def get_variance(self, principle: str, domain: str) -> float:
        key = f"{principle}:{domain}"
        if key in self.posteriors:
            return self.posteriors[key].variance()
        return 0.25  # High uncertainty


def calculate_agent_drift(agents: List[Agent]) -> Dict[str, float]:
    """Calculate drift (variance) across agents for each key."""
    drift = {}

    # Collect all keys
    all_keys = set()
    for agent in agents:
        all_keys.update(agent.posteriors.keys())

    for key in all_keys:
        probs = [agent.posteriors.get(key, PosteriorState()).mean() for agent in agents]
        if len(probs) > 1:
            mean = sum(probs) / len(probs)
            variance = sum((p - mean) ** 2 for p in probs) / len(probs)
            drift[key] = math.sqrt(variance)

    return drift


def simulate_swarm(
    n_agents: int,
    n_outcomes: int,
    drift_rate: float,
    sync_enabled: bool,
    sync_failure_rate: float = 0.0,
) -> Dict:
    """
    Simulate multi-agent swarm with optional drift and sync.

    Args:
        n_agents: Number of agents
        n_outcomes: Total outcomes per agent
        drift_rate: Rate at which agents see different distributions
        sync_enabled: Whether to enable delta syncing
        sync_failure_rate: Rate at which sync messages are lost

    Returns:
        Metrics dict
    """
    random.seed(42)
    agents = [Agent(f"agent-{i}") for i in range(n_agents)]

    # Ground truth success rates
    ground_truth = {
        'yagni:architecture': 0.75,
        'kiss:architecture': 0.70,
        'tdd:testing': 0.80,
        'red-green-refactor:testing': 0.65,
        'horizontal-scale:scaling': 0.60,
        'brooks-law:management': 0.85,
        'defense-in-depth:security': 0.70,
        'measure-first:performance': 0.75,
    }

    # Track predictions vs outcomes
    predictions = []
    outcomes = []

    for _ in range(n_outcomes):
        for agent_idx, agent in enumerate(agents):
            # Pick a random principle
            principle, domain = random.choice(PRINCIPLES)
            key = f"{principle}:{domain}"
            base_rate = ground_truth.get(key, 0.5)

            # Apply drift: each agent sees slightly different rate
            if random.random() < drift_rate:
                # Drift: shift by up to ±20%
                drift_shift = random.uniform(-0.20, 0.20)
                # Different agents drift in different directions
                drift_shift *= (1 if agent_idx % 2 == 0 else -1)
                effective_rate = max(0.1, min(0.9, base_rate + drift_shift))
            else:
                effective_rate = base_rate

            # Sample outcome
            success = random.random() < effective_rate

            # Record prediction before updating
            pred = agent.get_probability(principle, domain)
            predictions.append(pred)
            outcomes.append(1.0 if success else 0.0)

            # Update agent
            agent.record_outcome(principle, domain, success)

        # Sync if enabled
        if sync_enabled:
            # Collect deltas from agents that need sync
            all_deltas = []
            for agent in agents:
                if agent.needs_sync():
                    deltas = agent.get_deltas()
                    # Apply sync failure rate
                    deltas = [d for d in deltas if random.random() > sync_failure_rate]
                    all_deltas.extend(deltas)

            # Distribute deltas
            for agent in agents:
                agent.apply_peer_deltas(all_deltas)
                agent.apply_decay()

    # Calculate metrics
    drift_map = calculate_agent_drift(agents)
    avg_drift = sum(drift_map.values()) / len(drift_map) if drift_map else 0

    # Accuracy: prediction > 0.5 matches outcome
    correct = sum(1 for p, o in zip(predictions, outcomes)
                  if (p >= 0.5) == (o >= 0.5))
    accuracy = correct / len(predictions) if predictions else 0

    # Brier score
    brier = sum((p - o) ** 2 for p, o in zip(predictions, outcomes)) / len(predictions)

    # Consensus: average probability across agents for each key
    consensus_probs = {}
    for key in ground_truth.keys():
        probs = [agent.posteriors.get(key, PosteriorState()).mean() for agent in agents]
        consensus_probs[key] = sum(probs) / len(probs)

    # MAE from ground truth
    mae = sum(abs(consensus_probs.get(k, 0.5) - v)
              for k, v in ground_truth.items()) / len(ground_truth)

    return {
        'accuracy': accuracy,
        'brier_score': brier,
        'avg_drift': avg_drift,
        'mae_from_truth': mae,
        'consensus_probs': consensus_probs,
        'drift_map': drift_map,
    }


def probe_drift_without_sync():
    """Test 1: 25% drift without sync (baseline)."""
    print("\n" + "="*60)
    print("PROBE 1: 25% Drift WITHOUT Sync (Baseline)")
    print("="*60)

    metrics = simulate_swarm(
        n_agents=3,
        n_outcomes=500,
        drift_rate=0.25,
        sync_enabled=False,
    )

    print(f"\n  Accuracy:       {metrics['accuracy']:.1%}")
    print(f"  Brier Score:    {metrics['brier_score']:.3f}")
    print(f"  Avg Drift:      {metrics['avg_drift']*100:.1f}pp")
    print(f"  MAE from Truth: {metrics['mae_from_truth']:.3f}")

    return metrics


def probe_drift_with_sync():
    """Test 2: 25% drift WITH sync."""
    print("\n" + "="*60)
    print("PROBE 2: 25% Drift WITH Sync (V4)")
    print("="*60)

    metrics = simulate_swarm(
        n_agents=3,
        n_outcomes=500,
        drift_rate=0.25,
        sync_enabled=True,
    )

    print(f"\n  Accuracy:       {metrics['accuracy']:.1%}")
    print(f"  Brier Score:    {metrics['brier_score']:.3f}")
    print(f"  Avg Drift:      {metrics['avg_drift']*100:.1f}pp")
    print(f"  MAE from Truth: {metrics['mae_from_truth']:.3f}")

    return metrics


def probe_sync_failure():
    """Test 3: Sync with 30% message loss."""
    print("\n" + "="*60)
    print("PROBE 3: Sync with 30% Message Loss")
    print("="*60)

    metrics = simulate_swarm(
        n_agents=3,
        n_outcomes=500,
        drift_rate=0.25,
        sync_enabled=True,
        sync_failure_rate=0.30,
    )

    print(f"\n  Accuracy:       {metrics['accuracy']:.1%}")
    print(f"  Brier Score:    {metrics['brier_score']:.3f}")
    print(f"  Avg Drift:      {metrics['avg_drift']*100:.1f}pp")
    print(f"  MAE from Truth: {metrics['mae_from_truth']:.3f}")

    return metrics


def probe_high_drift_stress():
    """Test 4: Extreme 50% drift (stress test)."""
    print("\n" + "="*60)
    print("PROBE 4: Extreme 50% Drift (Stress Test)")
    print("="*60)

    metrics_no_sync = simulate_swarm(
        n_agents=5,
        n_outcomes=1000,
        drift_rate=0.50,
        sync_enabled=False,
    )

    metrics_with_sync = simulate_swarm(
        n_agents=5,
        n_outcomes=1000,
        drift_rate=0.50,
        sync_enabled=True,
    )

    print(f"\n  Without Sync:")
    print(f"    Accuracy:    {metrics_no_sync['accuracy']:.1%}")
    print(f"    Drift:       {metrics_no_sync['avg_drift']*100:.1f}pp")

    print(f"\n  With Sync:")
    print(f"    Accuracy:    {metrics_with_sync['accuracy']:.1%}")
    print(f"    Drift:       {metrics_with_sync['avg_drift']*100:.1f}pp")

    improvement = metrics_with_sync['accuracy'] - metrics_no_sync['accuracy']
    drift_reduction = metrics_no_sync['avg_drift'] - metrics_with_sync['avg_drift']

    print(f"\n  Sync Improvement: {improvement*100:+.1f}pp accuracy")
    print(f"  Drift Reduction:  {drift_reduction*100:.1f}pp")

    return metrics_no_sync, metrics_with_sync


def main():
    print("\n" + "╔" + "═"*58 + "╗")
    print("║  🌊 V4 SWARM DRIFT PROBE                                 ║")
    print("║     Testing SwarmPosterior under adversarial drift       ║")
    print("╚" + "═"*58 + "╝")

    # Run probes
    baseline = probe_drift_without_sync()
    with_sync = probe_drift_with_sync()
    with_loss = probe_sync_failure()
    stress_no, stress_yes = probe_high_drift_stress()

    # Summary
    print("\n" + "="*60)
    print("SWARM PROBE SUMMARY")
    print("="*60)

    print("\n  Configuration      | Accuracy | Drift  | Brier")
    print("  -------------------|----------|--------|------")
    print(f"  No Sync (25%)      | {baseline['accuracy']:.1%}    | {baseline['avg_drift']*100:.1f}pp  | {baseline['brier_score']:.3f}")
    print(f"  With Sync (25%)    | {with_sync['accuracy']:.1%}    | {with_sync['avg_drift']*100:.1f}pp  | {with_sync['brier_score']:.3f}")
    print(f"  Sync + 30% Loss    | {with_loss['accuracy']:.1%}    | {with_loss['avg_drift']*100:.1f}pp  | {with_loss['brier_score']:.3f}")
    print(f"  Stress No Sync     | {stress_no['accuracy']:.1%}    | {stress_no['avg_drift']*100:.1f}pp  | {stress_no['brier_score']:.3f}")
    print(f"  Stress With Sync   | {stress_yes['accuracy']:.1%}    | {stress_yes['avg_drift']*100:.1f}pp  | {stress_yes['brier_score']:.3f}")

    # Calculate improvements
    sync_acc_gain = with_sync['accuracy'] - baseline['accuracy']
    sync_drift_reduction = baseline['avg_drift'] - with_sync['avg_drift']
    stress_acc_gain = stress_yes['accuracy'] - stress_no['accuracy']

    print(f"\n  KEY FINDINGS:")
    print(f"    Sync accuracy gain (25% drift):  {sync_acc_gain*100:+.1f}pp")
    print(f"    Sync drift reduction:            {sync_drift_reduction*100:.1f}pp → {with_sync['avg_drift']*100:.1f}pp")
    print(f"    Stress test gain (50% drift):    {stress_acc_gain*100:+.1f}pp")

    # Validate targets
    print("\n  TARGET VALIDATION:")

    targets = [
        ("Sync gain ≥3%", sync_acc_gain >= 0.03),
        ("Drift reduction ≥50%", sync_drift_reduction / baseline['avg_drift'] >= 0.5 if baseline['avg_drift'] > 0 else True),
        ("Stress resilience ≥5%", stress_acc_gain >= 0.05),
        ("Loss tolerance <2% drop", with_sync['accuracy'] - with_loss['accuracy'] < 0.02),
    ]

    all_pass = True
    for name, passed in targets:
        status = "✅ PASS" if passed else "❌ FAIL"
        print(f"    {status}: {name}")
        all_pass = all_pass and passed

    print(f"\n  Overall: {'✅ V4 VALIDATED' if all_pass else '⚠️  NEEDS V5 FIXES'}")

    # V5 recommendations
    if not all_pass:
        print("\n" + "="*60)
        print("V5 RECOMMENDATIONS")
        print("="*60)
        print("""
  1. ADD DRIFT DETECTION
     Trigger: variance > 2pp across agents
     Action: Force immediate resync + increase sync frequency

  2. ADAPTIVE SYNC INTERVAL
     Low drift: sync every 20 outcomes
     High drift: sync every 5 outcomes

  3. REDUNDANT DELTA BROADCAST
     Send each delta 2x to overcome 30% loss
""")

    return all_pass


if __name__ == '__main__':
    import sys
    sys.exit(0 if main() else 1)
