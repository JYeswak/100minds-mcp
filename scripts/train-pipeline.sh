#!/usr/bin/env bash
#
# Neural Bandit Training Pipeline
# Full pipeline: generate synthetic → extract real → train → export ONNX
#
# Usage:
#   ./scripts/train-pipeline.sh              # Full pipeline
#   ./scripts/train-pipeline.sh --check      # Check data availability
#   ./scripts/train-pipeline.sh --synthetic  # Synthetic only (cold-start)
#   ./scripts/train-pipeline.sh --hybrid     # Hybrid (30% real + 70% synthetic)
#

set -euo pipefail

PROJECT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
MODELS_DIR="$PROJECT_DIR/models"
DATA_DIR="$PROJECT_DIR/data"
TRAINING_DIR="$PROJECT_DIR/training_data"
DB_PATH="$DATA_DIR/100minds.db"

# Ensure directories exist
mkdir -p "$MODELS_DIR" "$TRAINING_DIR"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log_info() { echo -e "${BLUE}[INFO]${NC} $1"; }
log_success() { echo -e "${GREEN}[OK]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

# Activate venv if it exists
if [[ -d "$PROJECT_DIR/.venv" ]]; then
    source "$PROJECT_DIR/.venv/bin/activate"
fi

check_mode() {
    log_info "Checking outcome data availability..."
    python3 "$PROJECT_DIR/scripts/extract_real_outcomes.py" --db "$DB_PATH" --check
}

generate_synthetic() {
    log_info "Generating synthetic training data..."

    # Use the Rust training data generator
    if command -v cargo &> /dev/null && [[ -f "$PROJECT_DIR/Cargo.toml" ]]; then
        cd "$PROJECT_DIR"
        cargo run --release -- --benchmark synthetic \
            --count 12000 \
            --output "$TRAINING_DIR/synthetic.jsonl" 2>/dev/null || {
            log_warn "Rust generator failed, falling back to Python..."
            python3 "$PROJECT_DIR/scripts/train_neural_bandit.py" \
                --generate-only \
                --output "$TRAINING_DIR/synthetic.jsonl" \
                --count 12000 2>/dev/null || {
                log_error "Failed to generate synthetic data"
                return 1
            }
        }
    else
        log_warn "Cargo not available, using Python generator..."
        python3 "$PROJECT_DIR/scripts/train_neural_bandit.py" \
            --generate-only \
            --output "$TRAINING_DIR/synthetic.jsonl" \
            --count 12000
    fi

    log_success "Generated synthetic data: $TRAINING_DIR/synthetic.jsonl"
}

extract_real() {
    log_info "Extracting real outcome data..."
    python3 "$PROJECT_DIR/scripts/extract_real_outcomes.py" \
        --db "$DB_PATH" \
        --output "$TRAINING_DIR/real_outcomes.jsonl"

    # Check if any data was extracted
    if [[ -f "$TRAINING_DIR/real_outcomes.jsonl" ]]; then
        local count=$(wc -l < "$TRAINING_DIR/real_outcomes.jsonl")
        log_success "Extracted $count real examples"
        return 0
    else
        log_warn "No real outcomes available"
        return 1
    fi
}

create_hybrid() {
    local ratio="${1:-0.3}"
    log_info "Creating hybrid dataset (${ratio}0% real)..."

    python3 "$PROJECT_DIR/scripts/extract_real_outcomes.py" \
        --db "$DB_PATH" \
        --synthetic "$TRAINING_DIR/synthetic.jsonl" \
        --ratio "$ratio" \
        --total 10000 \
        --output "$TRAINING_DIR/hybrid.jsonl"

    log_success "Created hybrid dataset: $TRAINING_DIR/hybrid.jsonl"
}

train_model() {
    local data_file="$1"
    local model_name="${2:-neural_bandit}"

    log_info "Training neural posterior model..."
    log_info "  Input: $data_file"
    log_info "  Output: $MODELS_DIR/${model_name}.onnx"

    python3 "$PROJECT_DIR/scripts/train_neural_bandit.py" \
        --data "$data_file" \
        --epochs 30 \
        --batch-size 64 \
        --label-noise 0.15 \
        --label-smoothing 0.1 \
        --early-stop 5 \
        --export "$MODELS_DIR/${model_name}.onnx"

    log_success "Model trained and exported: $MODELS_DIR/${model_name}.onnx"
}

full_pipeline() {
    local mode="${1:-auto}"

    echo ""
    echo "╔═══════════════════════════════════════════════════════════════╗"
    echo "║  🧠 Neural Bandit Training Pipeline                           ║"
    echo "║     100minds Principle Selection Optimizer                    ║"
    echo "╚═══════════════════════════════════════════════════════════════╝"
    echo ""

    # Step 1: Check data availability
    log_info "Step 1/4: Checking data availability..."
    local real_count=0
    if [[ -f "$DB_PATH" ]]; then
        real_count=$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM decisions WHERE outcome_success IS NOT NULL" 2>/dev/null || echo "0")
    fi
    log_info "  Real outcomes available: $real_count"

    # Step 2: Generate synthetic data
    log_info "Step 2/4: Generating synthetic data..."
    generate_synthetic

    # Step 3: Determine training data source
    log_info "Step 3/4: Preparing training data..."
    local training_data=""

    case "$mode" in
        synthetic)
            training_data="$TRAINING_DIR/synthetic.jsonl"
            log_info "  Mode: Synthetic only"
            ;;
        hybrid)
            if [[ "$real_count" -gt 0 ]]; then
                create_hybrid 0.3
                training_data="$TRAINING_DIR/hybrid.jsonl"
                log_info "  Mode: Hybrid (30% real)"
            else
                log_warn "  No real data available, falling back to synthetic"
                training_data="$TRAINING_DIR/synthetic.jsonl"
            fi
            ;;
        auto|*)
            if [[ "$real_count" -ge 100 ]]; then
                create_hybrid 0.5
                training_data="$TRAINING_DIR/hybrid.jsonl"
                log_info "  Mode: Auto-selected hybrid (50% real)"
            elif [[ "$real_count" -gt 0 ]]; then
                create_hybrid 0.3
                training_data="$TRAINING_DIR/hybrid.jsonl"
                log_info "  Mode: Auto-selected hybrid (30% real)"
            else
                training_data="$TRAINING_DIR/synthetic.jsonl"
                log_info "  Mode: Auto-selected synthetic (cold-start)"
            fi
            ;;
    esac

    # Step 4: Train model
    log_info "Step 4/4: Training neural posterior..."
    train_model "$training_data"

    echo ""
    echo "════════════════════════════════════════════════════════════════"
    log_success "Pipeline complete!"
    echo "  Model: $MODELS_DIR/neural_bandit.onnx"
    echo "  Vocab: $MODELS_DIR/neural_bandit_vocab_v2.json"
    echo ""
    echo "Next steps:"
    echo "  1. Record outcomes: 100minds --outcome <decision-id> --success"
    echo "  2. Re-train periodically to incorporate new outcomes"
    echo "  3. Target: 85%+ validation accuracy with sufficient real data"
    echo "════════════════════════════════════════════════════════════════"
}

# Main
case "${1:-}" in
    --check)
        check_mode
        ;;
    --synthetic)
        full_pipeline synthetic
        ;;
    --hybrid)
        full_pipeline hybrid
        ;;
    --help|-h)
        echo "Usage: $0 [option]"
        echo ""
        echo "Options:"
        echo "  (none)       Auto-select mode based on available data"
        echo "  --check      Check outcome data availability"
        echo "  --synthetic  Force synthetic-only training (cold-start)"
        echo "  --hybrid     Force hybrid mode (30% real + 70% synthetic)"
        echo "  --help       Show this help"
        ;;
    *)
        full_pipeline auto
        ;;
esac
