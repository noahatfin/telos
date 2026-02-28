#!/usr/bin/env bash
# Telos Validation Pipeline — Full experiment runner
#
# Builds telos, sets up the TaskBoard project in a temp directory,
# runs all 8 scenario stages, then generates context for all 7 experiments.
#
# Usage: ./run_all.sh [--keep-dir]
#   --keep-dir: Don't delete the temp project directory on exit
#
# Output:
#   - validation/measurements/exp_*_git_only.json
#   - validation/measurements/exp_*_telos_git.json
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
TELOS_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
KEEP_DIR=false

for arg in "$@"; do
    case "$arg" in
        --keep-dir) KEEP_DIR=true ;;
    esac
done

echo "============================================"
echo "  Telos Validation Pipeline"
echo "============================================"
echo ""

# --- Step 1: Build Telos ---
echo ">>> Step 1: Building telos..."
cd "$TELOS_ROOT"
cargo build --release 2>&1
TELOS_BIN="$TELOS_ROOT/target/release/telos-cli"

if [ ! -f "$TELOS_BIN" ]; then
    echo "ERROR: telos-cli binary not found at $TELOS_BIN"
    echo "Checking for alternative binary names..."
    ls -la "$TELOS_ROOT/target/release/" | grep telos || true
    exit 1
fi

echo "Telos binary: $TELOS_BIN"
echo ""

# --- Step 2: Set up temp project directory ---
echo ">>> Step 2: Setting up project directory..."
PROJECT_DIR=$(mktemp -d "${TMPDIR:-/tmp}/telos-validation-XXXXXX")
echo "Project directory: $PROJECT_DIR"

cleanup() {
    if [ "$KEEP_DIR" = false ]; then
        echo ""
        echo "Cleaning up: $PROJECT_DIR"
        rm -rf "$PROJECT_DIR"
    else
        echo ""
        echo "Keeping project directory: $PROJECT_DIR"
    fi
}
trap cleanup EXIT

# Copy TaskBoard source to temp directory
cp -r "$SCRIPT_DIR/taskboard/src" "$PROJECT_DIR/src"
cp "$SCRIPT_DIR/taskboard/Cargo.toml" "$PROJECT_DIR/Cargo.toml"
cp "$SCRIPT_DIR/taskboard/Cargo.lock" "$PROJECT_DIR/Cargo.lock"
# Copy .gitignore if it exists
if [ -f "$SCRIPT_DIR/taskboard/.gitignore" ]; then
    cp "$SCRIPT_DIR/taskboard/.gitignore" "$PROJECT_DIR/.gitignore"
fi

# Set up git config for the temp repo
export GIT_AUTHOR_NAME="Validation Agent"
export GIT_AUTHOR_EMAIL="agent@taskboard.dev"
export GIT_COMMITTER_NAME="Validation Agent"
export GIT_COMMITTER_EMAIL="agent@taskboard.dev"
export TELOS_AUTHOR_NAME="Agent"
export TELOS_AUTHOR_EMAIL="agent@taskboard.dev"
export TELOS_BIN="$TELOS_BIN"

echo ""

# --- Step 3: Run all scenario stages ---
echo ">>> Step 3: Running scenario stages..."
echo ""

SCENARIOS=(
    "01_bootstrap.sh"
    "02_auth.sh"
    "03_tasks.sh"
    "04_boards.sh"
    "05_regression.sh"
    "06_refactor.sh"
    "07_status_regression.sh"
    "08_security_regression.sh"
)

for scenario in "${SCENARIOS[@]}"; do
    echo "--- Running $scenario ---"
    bash "$SCRIPT_DIR/scenarios/$scenario" "$PROJECT_DIR"
    echo ""
done

echo "All scenarios complete."
echo ""

# --- Step 4: Verify compilation ---
echo ">>> Step 4: Verifying TaskBoard still compiles..."
cd "$PROJECT_DIR"
# Only check that the Rust source compiles (skip tests since some are expected to fail)
cargo check 2>&1 || {
    echo "WARNING: TaskBoard compilation check failed."
    echo "This may be expected if scenario bugs broke the code intentionally."
    echo "Continuing with experiment generation..."
}
echo ""

# --- Step 5: Show git history ---
echo ">>> Step 5: Git history after all scenarios:"
cd "$PROJECT_DIR"
git log --oneline
echo ""
echo "Telos log:"
$TELOS_BIN log 2>/dev/null || echo "(telos log not available)"
echo ""

# --- Step 6: Generate experiment contexts ---
echo ">>> Step 6: Generating experiment contexts..."
echo ""

# We need to generate contexts at specific git states for each experiment.
# Experiments A-D use the state after their respective scenario stages.
# Experiments E-G use the state after stages 7 and 8.

# Save current HEAD (after all scenarios) for experiments E-G
FINAL_HEAD=$(git rev-parse HEAD)

# --- Experiment A: Cross-Session Memory (needs state after stage 2) ---
echo "--- Generating Experiment A context (after stage 2: auth)..."
# Find the commit from stage 2 (second git commit = "Add auth error handling and RBAC")
# After stages 1-2, there should be 3 commits total (bootstrap + 2 auth commits)
STAGE2_COMMIT=$(git log --oneline | tail -n +$(($(git log --oneline | wc -l) - 2)) | head -1 | awk '{print $1}')
git checkout "$STAGE2_COMMIT" 2>/dev/null || git checkout HEAD~$(( $(git log --oneline | wc -l) - 3 )) 2>/dev/null
python3 "$SCRIPT_DIR/experiments/memory_test.py" "$PROJECT_DIR" "$TELOS_BIN" || echo "  (Experiment A context generation had issues)"
git checkout "$FINAL_HEAD" 2>/dev/null

# --- Experiment B: Debugging (needs state after stage 5: regression) ---
echo ""
echo "--- Generating Experiment B context (after stage 5: regression)..."
# Find the regression commit (token expiry change)
# After stages 1-5, find the commit that changed token expiry
STAGE5_COMMIT=$(git log --oneline --all --grep="Increase token expiry" | head -1 | awk '{print $1}')
if [ -n "$STAGE5_COMMIT" ]; then
    git checkout "$STAGE5_COMMIT" 2>/dev/null
else
    echo "  Warning: Could not find stage 5 commit, using HEAD"
fi
python3 "$SCRIPT_DIR/experiments/debugging_test.py" "$PROJECT_DIR" "$TELOS_BIN" || echo "  (Experiment B context generation had issues)"
git checkout "$FINAL_HEAD" 2>/dev/null

# --- Experiment C: Code Review (needs state after stage 5: regression) ---
echo ""
echo "--- Generating Experiment C context (after stage 5: regression)..."
if [ -n "$STAGE5_COMMIT" ]; then
    git checkout "$STAGE5_COMMIT" 2>/dev/null
fi
python3 "$SCRIPT_DIR/experiments/review_test.py" "$PROJECT_DIR" "$TELOS_BIN" || echo "  (Experiment C context generation had issues)"
git checkout "$FINAL_HEAD" 2>/dev/null

# --- Experiment D: Refactoring (needs state after stage 6) ---
echo ""
echo "--- Generating Experiment D context (after stage 6: refactor setup)..."
STAGE6_COMMIT=$(git log --oneline --all --grep="Rename.*tasks.*items" | head -1 | awk '{print $1}')
# If no commit found, stage 6 only records an intent (no code change)
# Use the state right after stage 5's regression for the code, but with stage 6 intent
git checkout "$FINAL_HEAD" 2>/dev/null
python3 "$SCRIPT_DIR/experiments/refactor_test.py" "$PROJECT_DIR" "$TELOS_BIN" || echo "  (Experiment D context generation had issues)"

# --- Experiments E, F, G: All use state after stage 8 (already at FINAL_HEAD) ---
echo ""
echo "--- Generating Experiment E context (status regression from stage 7)..."
# Need to be at the commit where stage 7's regression was introduced
STAGE7_REGRESSION=$(git log --oneline --all --grep="Allow flexible task status" | head -1 | awk '{print $1}')
if [ -n "$STAGE7_REGRESSION" ]; then
    git checkout "$STAGE7_REGRESSION" 2>/dev/null
fi
python3 "$SCRIPT_DIR/experiments/status_test.py" "$PROJECT_DIR" "$TELOS_BIN" || echo "  (Experiment E context generation had issues)"
git checkout "$FINAL_HEAD" 2>/dev/null

echo ""
echo "--- Generating Experiment F context (error leak from stage 8)..."
# F and G need to be at FINAL_HEAD since they look backwards in history
python3 "$SCRIPT_DIR/experiments/leak_test.py" "$PROJECT_DIR" "$TELOS_BIN" || echo "  (Experiment F context generation had issues)"

echo ""
echo "--- Generating Experiment G context (escalation from stage 8)..."
python3 "$SCRIPT_DIR/experiments/escalation_test.py" "$PROJECT_DIR" "$TELOS_BIN" || echo "  (Experiment G context generation had issues)"

echo ""

# --- Step 7: Summary ---
echo "============================================"
echo "  Pipeline Complete"
echo "============================================"
echo ""
echo "Generated context files:"
ls -la "$SCRIPT_DIR/measurements/"exp_*.json 2>/dev/null || echo "  (no context files found)"
echo ""
echo "Experiments ready for evaluation:"
echo "  A: Cross-Session Memory       (exp_a_*.json)"
echo "  B: Debugging with Intent      (exp_b_*.json)"
echo "  C: Constraint Guardian Review  (exp_c_*.json)"
echo "  D: Impact-Guided Refactoring  (exp_d_*.json)"
echo "  E: Status Transition Integrity (exp_e_*.json)"
echo "  F: Error Information Leak      (exp_f_*.json)"
echo "  G: Permission Escalation       (exp_g_*.json)"
echo ""
echo "Next step: Feed context JSONs to an LLM and score responses"
echo "  with the evaluate_*() functions in each experiment script."
if [ "$KEEP_DIR" = true ]; then
    echo ""
    echo "Project directory preserved at: $PROJECT_DIR"
fi
