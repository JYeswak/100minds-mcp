#!/usr/bin/env python3
"""
Neural Bandit Training Script for 100minds

Trains a neural posterior network to replace Beta distributions for principle selection.
Architecture: MLP encoder with attention on context features.

Usage:
    python scripts/train_neural_bandit.py --data training_data/neural_training.jsonl
    python scripts/train_neural_bandit.py --data training_data/neural_training.jsonl --epochs 20 --export model.onnx
"""

import argparse
import json
import os
from pathlib import Path
from typing import Dict, List, Tuple
import numpy as np

# Check for torch availability
try:
    import torch
    import torch.nn as nn
    import torch.nn.functional as F
    from torch.utils.data import Dataset, DataLoader
    TORCH_AVAILABLE = True
except ImportError:
    TORCH_AVAILABLE = False
    print("Warning: PyTorch not installed. Install with: pip install torch")


class NeuralBanditDataset(Dataset):
    """Dataset for neural bandit training examples."""

    def __init__(self, jsonl_path: str, max_examples: int = None):
        self.examples = []
        self.domain_vocab = {}
        self.stakeholder_vocab = {}
        self.stage_vocab = {}
        self.urgency_vocab = {}
        self.principle_vocab = {}
        self.thinker_vocab = {}

        # Load examples
        with open(jsonl_path, 'r') as f:
            for i, line in enumerate(f):
                if max_examples and i >= max_examples:
                    break
                ex = json.loads(line.strip())
                self.examples.append(ex)

                # Build vocabularies
                self._add_to_vocab(self.domain_vocab, ex['domain'])
                ctx = ex['context_features']
                self._add_to_vocab(self.stakeholder_vocab, ctx['stakeholder'])
                self._add_to_vocab(self.stage_vocab, ctx['company_stage'])
                self._add_to_vocab(self.urgency_vocab, ctx['urgency'])
                self._add_to_vocab(self.principle_vocab, ex['principle_id'])
                self._add_to_vocab(self.thinker_vocab, ex['thinker_id'])

        print(f"Loaded {len(self.examples)} examples")
        print(f"Vocabularies: domains={len(self.domain_vocab)}, principles={len(self.principle_vocab)}, thinkers={len(self.thinker_vocab)}")

    def _add_to_vocab(self, vocab: Dict[str, int], item: str):
        if item not in vocab:
            vocab[item] = len(vocab)

    def __len__(self):
        return len(self.examples)

    def __getitem__(self, idx) -> Tuple[torch.Tensor, torch.Tensor, float]:
        ex = self.examples[idx]
        ctx = ex['context_features']

        # Build feature vector
        features = []

        # Domain one-hot
        domain_vec = [0.0] * len(self.domain_vocab)
        domain_vec[self.domain_vocab.get(ex['domain'], 0)] = 1.0
        features.extend(domain_vec)

        # Stakeholder one-hot
        stakeholder_vec = [0.0] * len(self.stakeholder_vocab)
        stakeholder_vec[self.stakeholder_vocab.get(ctx['stakeholder'], 0)] = 1.0
        features.extend(stakeholder_vec)

        # Company stage one-hot
        stage_vec = [0.0] * len(self.stage_vocab)
        stage_vec[self.stage_vocab.get(ctx['company_stage'], 0)] = 1.0
        features.extend(stage_vec)

        # Urgency one-hot
        urgency_vec = [0.0] * len(self.urgency_vocab)
        urgency_vec[self.urgency_vocab.get(ctx['urgency'], 0)] = 1.0
        features.extend(urgency_vec)

        # Scalar features (normalized)
        features.append(ex['difficulty'] / 5.0)
        features.append(ex['position_rank'] / 10.0)
        features.append(ex['confidence'])
        features.append(1.0 if ctx['domain_match'] else 0.0)
        features.append(ctx['total_principles_selected'] / 10.0)
        features.append(1.0 if ctx['is_for_position'] else 0.0)

        # Principle embedding index
        principle_idx = self.principle_vocab.get(ex['principle_id'], 0)

        # Thinker embedding index
        thinker_idx = self.thinker_vocab.get(ex['thinker_id'], 0)

        context_tensor = torch.tensor(features, dtype=torch.float32)
        arm_tensor = torch.tensor([principle_idx, thinker_idx], dtype=torch.long)
        label = ex['success']

        return context_tensor, arm_tensor, label

    @property
    def context_dim(self) -> int:
        """Dimension of context feature vector."""
        sample = self[0]
        return sample[0].shape[0]

    @property
    def num_principles(self) -> int:
        return len(self.principle_vocab)

    @property
    def num_thinkers(self) -> int:
        return len(self.thinker_vocab)


class NeuralPosteriorV2(nn.Module):
    """
    Neural posterior network for principle selection (V2 - SOTA 2026).

    Improvements over V1:
    - Larger embeddings (128 vs 64) for better principle representation
    - Layer normalization for training stability
    - Higher dropout (0.2) for regularization
    - Residual connections in combiner
    - Label smoothing support

    Output: P(success | context, principle)
    """

    def __init__(
        self,
        context_dim: int,
        num_principles: int,
        num_thinkers: int,
        embed_dim: int = 128,  # Increased from 64
        hidden_dim: int = 256,  # Increased from 128
        dropout: float = 0.2,  # Increased from 0.1
    ):
        super().__init__()

        self.context_dim = context_dim
        self.num_principles = num_principles
        self.num_thinkers = num_thinkers

        # Embeddings for principles and thinkers (larger)
        self.principle_embed = nn.Embedding(num_principles, embed_dim)
        self.thinker_embed = nn.Embedding(num_thinkers, embed_dim)

        # Context encoder with LayerNorm
        self.context_encoder = nn.Sequential(
            nn.Linear(context_dim, hidden_dim),
            nn.LayerNorm(hidden_dim),
            nn.ReLU(),
            nn.Dropout(dropout),
            nn.Linear(hidden_dim, hidden_dim),
            nn.LayerNorm(hidden_dim),
            nn.ReLU(),
            nn.Dropout(dropout),
        )

        # Attention over context (simple self-attention)
        self.context_attention = nn.MultiheadAttention(
            embed_dim=hidden_dim,
            num_heads=8,  # Increased from 4
            dropout=dropout,
            batch_first=True,
        )
        self.attn_norm = nn.LayerNorm(hidden_dim)

        # Combine context + arm embeddings with residual
        combined_dim = hidden_dim + 2 * embed_dim
        self.combiner = nn.Sequential(
            nn.Linear(combined_dim, hidden_dim),
            nn.LayerNorm(hidden_dim),
            nn.ReLU(),
            nn.Dropout(dropout),
            nn.Linear(hidden_dim, hidden_dim // 2),
            nn.LayerNorm(hidden_dim // 2),
            nn.ReLU(),
        )

        # Output head: predicts P(success) and uncertainty
        self.success_head = nn.Linear(hidden_dim // 2, 1)
        self.uncertainty_head = nn.Linear(hidden_dim // 2, 1)

    def forward(
        self,
        context: torch.Tensor,
        arm_indices: torch.Tensor,
    ) -> Tuple[torch.Tensor, torch.Tensor]:
        """Forward pass with layer normalization."""
        # Encode context
        ctx_encoded = self.context_encoder(context)  # (batch, hidden)

        # Self-attention on context with residual + norm
        ctx_seq = ctx_encoded.unsqueeze(1)  # (batch, 1, hidden)
        ctx_attended, _ = self.context_attention(ctx_seq, ctx_seq, ctx_seq)
        ctx_attended = self.attn_norm(ctx_attended.squeeze(1) + ctx_encoded)  # residual

        # Get arm embeddings
        principle_emb = self.principle_embed(arm_indices[:, 0])
        thinker_emb = self.thinker_embed(arm_indices[:, 1])

        # Combine
        combined = torch.cat([ctx_attended, principle_emb, thinker_emb], dim=1)
        hidden = self.combiner(combined)

        # Outputs
        success_logit = self.success_head(hidden)
        success_prob = torch.sigmoid(success_logit)

        uncertainty_raw = self.uncertainty_head(hidden)
        uncertainty = F.softplus(uncertainty_raw)

        return success_prob, uncertainty


# Alias for backward compatibility
class NeuralPosterior(NeuralPosteriorV2):
    """Alias for NeuralPosteriorV2 (backward compatible)."""
    pass


class NeuralPosteriorV1(nn.Module):
    """
    Original neural posterior network (V1 - for comparison).
    """

    def __init__(
        self,
        context_dim: int,
        num_principles: int,
        num_thinkers: int,
        embed_dim: int = 64,
        hidden_dim: int = 128,
        dropout: float = 0.1,
    ):
        super().__init__()

        self.context_dim = context_dim
        self.num_principles = num_principles
        self.num_thinkers = num_thinkers

        # Embeddings for principles and thinkers
        self.principle_embed = nn.Embedding(num_principles, embed_dim)
        self.thinker_embed = nn.Embedding(num_thinkers, embed_dim)

        # Context encoder (MLP)
        self.context_encoder = nn.Sequential(
            nn.Linear(context_dim, hidden_dim),
            nn.ReLU(),
            nn.Dropout(dropout),
            nn.Linear(hidden_dim, hidden_dim),
            nn.ReLU(),
            nn.Dropout(dropout),
        )

        # Attention over context (simple self-attention)
        self.context_attention = nn.MultiheadAttention(
            embed_dim=hidden_dim,
            num_heads=4,
            dropout=dropout,
            batch_first=True,
        )

        # Combine context + arm embeddings
        combined_dim = hidden_dim + 2 * embed_dim
        self.combiner = nn.Sequential(
            nn.Linear(combined_dim, hidden_dim),
            nn.ReLU(),
            nn.Dropout(dropout),
            nn.Linear(hidden_dim, hidden_dim // 2),
            nn.ReLU(),
        )

        # Output head: predicts P(success) and uncertainty
        self.success_head = nn.Linear(hidden_dim // 2, 1)
        self.uncertainty_head = nn.Linear(hidden_dim // 2, 1)

    def forward(
        self,
        context: torch.Tensor,
        arm_indices: torch.Tensor,
    ) -> Tuple[torch.Tensor, torch.Tensor]:
        """
        Forward pass.

        Args:
            context: (batch, context_dim) context features
            arm_indices: (batch, 2) [principle_idx, thinker_idx]

        Returns:
            success_prob: (batch, 1) predicted P(success)
            uncertainty: (batch, 1) epistemic uncertainty estimate
        """
        batch_size = context.shape[0]

        # Encode context
        ctx_encoded = self.context_encoder(context)  # (batch, hidden)

        # Self-attention on context (reshape for attention)
        ctx_seq = ctx_encoded.unsqueeze(1)  # (batch, 1, hidden)
        ctx_attended, _ = self.context_attention(ctx_seq, ctx_seq, ctx_seq)
        ctx_attended = ctx_attended.squeeze(1)  # (batch, hidden)

        # Get arm embeddings
        principle_emb = self.principle_embed(arm_indices[:, 0])  # (batch, embed)
        thinker_emb = self.thinker_embed(arm_indices[:, 1])  # (batch, embed)

        # Combine
        combined = torch.cat([ctx_attended, principle_emb, thinker_emb], dim=1)
        hidden = self.combiner(combined)

        # Outputs
        success_logit = self.success_head(hidden)
        success_prob = torch.sigmoid(success_logit)

        uncertainty_raw = self.uncertainty_head(hidden)
        uncertainty = F.softplus(uncertainty_raw)  # Ensure positive

        return success_prob, uncertainty


def train_epoch(
    model: nn.Module,
    dataloader: DataLoader,
    optimizer: torch.optim.Optimizer,
    device: torch.device,
    label_noise: float = 0.0,  # Adversarial label noise (0.0-0.15)
    label_smoothing: float = 0.0,  # Label smoothing (0.0-0.1)
) -> Tuple[float, float]:
    """Train for one epoch with adversarial augmentation.

    Args:
        label_noise: Probability of flipping labels (adversarial robustness)
        label_smoothing: Smooth labels from {0,1} to {smooth, 1-smooth}
    """
    model.train()
    total_loss = 0.0
    correct = 0
    total = 0

    for context, arm_indices, labels in dataloader:
        context = context.to(device)
        arm_indices = arm_indices.to(device)
        labels = torch.tensor(labels, dtype=torch.float32).to(device).unsqueeze(1)

        # Apply adversarial label noise (flip random labels)
        if label_noise > 0:
            noise_mask = torch.rand_like(labels) < label_noise
            labels = torch.where(noise_mask, 1.0 - labels, labels)

        # Apply label smoothing (soft labels)
        if label_smoothing > 0:
            labels = labels * (1.0 - label_smoothing) + 0.5 * label_smoothing

        optimizer.zero_grad()

        success_prob, uncertainty = model(context, arm_indices)

        # Binary cross-entropy loss
        bce_loss = F.binary_cross_entropy(success_prob, labels)

        # Uncertainty regularization: higher uncertainty for wrong predictions
        pred_error = (success_prob - labels).abs()
        uncertainty_loss = (uncertainty - pred_error).pow(2).mean()

        loss = bce_loss + 0.1 * uncertainty_loss

        loss.backward()
        optimizer.step()

        total_loss += loss.item() * context.shape[0]

        # Accuracy (use original labels without smoothing for metrics)
        preds = (success_prob > 0.5).float()
        original_labels = (labels > 0.5).float()  # Threshold smoothed labels
        correct += (preds == original_labels).sum().item()
        total += labels.shape[0]

    avg_loss = total_loss / total
    accuracy = correct / total
    return avg_loss, accuracy


def run_validation(
    model: nn.Module,
    dataloader: DataLoader,
    device: torch.device,
) -> Tuple[float, float]:
    """Run validation on held-out set."""
    model.eval()
    total_loss = 0.0
    correct = 0
    total = 0

    with torch.no_grad():
        for context, arm_indices, labels in dataloader:
            context = context.to(device)
            arm_indices = arm_indices.to(device)
            labels = torch.tensor(labels, dtype=torch.float32).to(device).unsqueeze(1)

            success_prob, _ = model(context, arm_indices)

            loss = F.binary_cross_entropy(success_prob, labels)
            total_loss += loss.item() * context.shape[0]

            preds = (success_prob > 0.5).float()
            correct += (preds == labels).sum().item()
            total += labels.shape[0]

    avg_loss = total_loss / total
    accuracy = correct / total
    return avg_loss, accuracy


def export_onnx(
    model: nn.Module,
    context_dim: int,
    output_path: str,
):
    """Export model to ONNX format for Rust integration."""
    model.eval()
    model.cpu()  # Export from CPU for compatibility

    # Create dummy inputs matching model dimensions
    dummy_context = torch.randn(1, context_dim)
    dummy_arm = torch.tensor([[0, 0]], dtype=torch.long)

    # Use legacy tracer-based export (more reliable)
    torch.onnx.export(
        model,
        (dummy_context, dummy_arm),
        output_path,
        input_names=['context', 'arm_indices'],
        output_names=['success_prob', 'uncertainty'],
        dynamic_axes={
            'context': {0: 'batch_size'},
            'arm_indices': {0: 'batch_size'},
            'success_prob': {0: 'batch_size'},
            'uncertainty': {0: 'batch_size'},
        },
        opset_version=14,
        dynamo=False,  # Use legacy tracer, not dynamo
    )
    print(f"Exported ONNX model to: {output_path}")


def save_vocab(dataset: NeuralBanditDataset, output_path: str):
    """Save vocabulary mappings for inference."""
    vocab = {
        'domain': dataset.domain_vocab,
        'stakeholder': dataset.stakeholder_vocab,
        'stage': dataset.stage_vocab,
        'urgency': dataset.urgency_vocab,
        'principle': dataset.principle_vocab,
        'thinker': dataset.thinker_vocab,
    }
    with open(output_path, 'w') as f:
        json.dump(vocab, f, indent=2)
    print(f"Saved vocabulary to: {output_path}")


def main():
    parser = argparse.ArgumentParser(description='Train Neural Bandit for 100minds (V2 SOTA)')
    parser.add_argument('--data', type=str, required=True, help='Path to training JSONL file')
    parser.add_argument('--epochs', type=int, default=20, help='Number of training epochs')
    parser.add_argument('--batch-size', type=int, default=128, help='Batch size')
    parser.add_argument('--lr', type=float, default=5e-4, help='Learning rate')
    parser.add_argument('--embed-dim', type=int, default=128, help='Embedding dimension (V2: 128)')
    parser.add_argument('--hidden-dim', type=int, default=256, help='Hidden layer dimension (V2: 256)')
    parser.add_argument('--max-examples', type=int, default=None, help='Max examples to load')
    parser.add_argument('--val-split', type=float, default=0.15, help='Validation split ratio')
    parser.add_argument('--export', type=str, default=None, help='Export ONNX model to path')
    parser.add_argument('--save-vocab', type=str, default=None, help='Save vocabulary mappings')
    parser.add_argument('--checkpoint', type=str, default='neural_bandit.pt', help='Checkpoint path')
    # V2 SOTA improvements
    parser.add_argument('--label-noise', type=float, default=0.1, help='Adversarial label noise (0.0-0.15)')
    parser.add_argument('--label-smoothing', type=float, default=0.05, help='Label smoothing (0.0-0.1)')
    parser.add_argument('--early-stop', type=int, default=5, help='Early stopping patience (0=disabled)')
    parser.add_argument('--v1', action='store_true', help='Use V1 architecture (for comparison)')
    args = parser.parse_args()

    if not TORCH_AVAILABLE:
        print("Error: PyTorch is required. Install with: pip install torch")
        return 1

    # Load data
    print(f"Loading data from {args.data}...")
    dataset = NeuralBanditDataset(args.data, args.max_examples)

    # Split into train/val
    val_size = int(len(dataset) * args.val_split)
    train_size = len(dataset) - val_size
    train_dataset, val_dataset = torch.utils.data.random_split(
        dataset, [train_size, val_size]
    )

    train_loader = DataLoader(train_dataset, batch_size=args.batch_size, shuffle=True)
    val_loader = DataLoader(val_dataset, batch_size=args.batch_size)

    print(f"Train: {len(train_dataset)}, Val: {len(val_dataset)}")

    # Create model
    device = torch.device('cuda' if torch.cuda.is_available() else 'mps' if torch.backends.mps.is_available() else 'cpu')
    print(f"Using device: {device}")

    # Choose model architecture
    if args.v1:
        print("Using V1 architecture (original)")
        model = NeuralPosteriorV1(
            context_dim=dataset.context_dim,
            num_principles=dataset.num_principles,
            num_thinkers=dataset.num_thinkers,
            embed_dim=64,
            hidden_dim=128,
        ).to(device)
    else:
        print("Using V2 architecture (SOTA 2026)")
        model = NeuralPosteriorV2(
            context_dim=dataset.context_dim,
            num_principles=dataset.num_principles,
            num_thinkers=dataset.num_thinkers,
            embed_dim=args.embed_dim,
            hidden_dim=args.hidden_dim,
        ).to(device)

    print(f"Model parameters: {sum(p.numel() for p in model.parameters()):,}")

    # Optimizer
    optimizer = torch.optim.AdamW(model.parameters(), lr=args.lr, weight_decay=0.01)
    scheduler = torch.optim.lr_scheduler.CosineAnnealingLR(optimizer, T_max=args.epochs)

    # Training loop with early stopping
    best_val_acc = 0.0
    patience_counter = 0

    print("\n" + "="*60)
    print("Training Neural Posterior Network (V2 SOTA)")
    print(f"  Label noise: {args.label_noise:.1%}")
    print(f"  Label smoothing: {args.label_smoothing:.1%}")
    print(f"  Early stopping: {args.early_stop} epochs")
    print("="*60)

    for epoch in range(args.epochs):
        train_loss, train_acc = train_epoch(
            model, train_loader, optimizer, device,
            label_noise=args.label_noise,
            label_smoothing=args.label_smoothing,
        )
        val_loss, val_acc = run_validation(model, val_loader, device)
        scheduler.step()

        print(f"Epoch {epoch+1:2d}/{args.epochs}: "
              f"Train Loss={train_loss:.4f} Acc={train_acc:.3f} | "
              f"Val Loss={val_loss:.4f} Acc={val_acc:.3f}")

        if val_acc > best_val_acc:
            best_val_acc = val_acc
            patience_counter = 0
            torch.save({
                'epoch': epoch,
                'model_state_dict': model.state_dict(),
                'optimizer_state_dict': optimizer.state_dict(),
                'val_acc': val_acc,
                'context_dim': dataset.context_dim,
                'num_principles': dataset.num_principles,
                'num_thinkers': dataset.num_thinkers,
                'architecture': 'v1' if args.v1 else 'v2',
            }, args.checkpoint)
            print(f"  -> Saved checkpoint (best val acc: {val_acc:.3f})")
        else:
            patience_counter += 1
            if args.early_stop > 0 and patience_counter >= args.early_stop:
                print(f"  -> Early stopping at epoch {epoch+1} (no improvement for {args.early_stop} epochs)")
                break

    print("\n" + "="*60)
    print(f"Training complete. Best validation accuracy: {best_val_acc:.3f}")
    if best_val_acc >= 0.70:
        print("🎯 TARGET REACHED: Val accuracy >= 70%!")
    elif best_val_acc >= 0.65:
        print("📈 Good progress: Val accuracy >= 65%")
    print("="*60)

    # Export ONNX if requested
    if args.export:
        # Load best checkpoint
        checkpoint = torch.load(args.checkpoint, weights_only=False)
        model.load_state_dict(checkpoint['model_state_dict'])
        export_onnx(model, dataset.context_dim, args.export)

    # Save vocabulary if requested
    if args.save_vocab:
        save_vocab(dataset, args.save_vocab)

    return 0


if __name__ == '__main__':
    exit(main())
