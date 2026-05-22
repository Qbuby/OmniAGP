#!/bin/bash
# OmniAGP M7 Smoke Test Runner
# Prerequisites: Rust toolchain, LLM service (Ollama/vLLM), optionally Godot 4
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
OUTPUT_DIR="${SMOKE_OUTPUT_DIR:-$REPO_ROOT/output/smoke-$(date +%Y%m%d-%H%M%S)}"

echo "=== OmniAGP M7 Smoke Test ==="
echo "Output: $OUTPUT_DIR"
echo ""

# Check prerequisites
echo "[1/5] Checking prerequisites..."
command -v cargo >/dev/null 2>&1 || { echo "ERROR: cargo not found"; exit 1; }

if [ -z "${LLM_BASE_URL:-}" ]; then
    export LLM_BASE_URL="http://localhost:11434/v1"
    echo "  LLM_BASE_URL defaulting to $LLM_BASE_URL"
fi

if [ -z "${LLM_MODEL:-}" ]; then
    export LLM_MODEL="qwen2.5-coder-7b"
    echo "  LLM_MODEL defaulting to $LLM_MODEL"
fi

# Verify LLM is reachable
if curl -sf "$LLM_BASE_URL/models" >/dev/null 2>&1; then
    echo "  LLM service: reachable"
else
    echo "  WARNING: LLM service not reachable at $LLM_BASE_URL"
    echo "  The test will fail at code generation stage."
fi

# Check Godot
if command -v godot >/dev/null 2>&1; then
    GODOT_VERSION=$(godot --version 2>/dev/null | head -1)
    echo "  Godot: $GODOT_VERSION"
else
    echo "  Godot: not found (headless QA and export will use stubs)"
fi

echo ""

# Build
echo "[2/5] Building smoke test binary..."
cd "$REPO_ROOT"
cargo build --release -p omni-smoke-test 2>&1 | tail -5
echo "  Build complete."
echo ""

# Run
echo "[3/5] Running end-to-end smoke test..."
export SMOKE_OUTPUT_DIR="$OUTPUT_DIR"
export RUST_LOG="${RUST_LOG:-info}"

START_TIME=$(date +%s)
cargo run --release -p omni-smoke-test 2>&1 | tee "$OUTPUT_DIR/smoke-test.log"
EXIT_CODE=${PIPESTATUS[0]}
END_TIME=$(date +%s)
ELAPSED=$((END_TIME - START_TIME))

echo ""
echo "[4/5] Results:"
echo "  Duration: ${ELAPSED}s"
echo "  Exit code: $EXIT_CODE"

if [ -f "$OUTPUT_DIR/smoke-test-report.json" ]; then
    echo "  Report: $OUTPUT_DIR/smoke-test-report.json"
    echo ""
    echo "  Summary:"
    python3 -c "
import json, sys
with open('$OUTPUT_DIR/smoke-test-report.json') as f:
    r = json.load(f)
print(f'    Success: {r[\"success\"]}')
print(f'    Stages:')
for s in r['stages']:
    status = 'PASS' if s['success'] else 'FAIL'
    print(f'      [{status}] {s[\"name\"]} ({s[\"duration_ms\"]}ms)')
if r.get('metrics'):
    m = r['metrics']
    print(f'    Metrics:')
    print(f'      Total time: {m[\"total_duration_ms\"]}ms')
    print(f'      LLM tokens: {m[\"llm_tokens_used\"]}')
    print(f'      Asset gen: {m[\"asset_generation_ms\"]}ms')
    print(f'      Code gen: {m[\"code_generation_ms\"]}ms')
" 2>/dev/null || echo "  (install python3 for formatted summary)"
fi

echo ""
echo "[5/5] Artifacts:"
echo "  $OUTPUT_DIR/"
ls -la "$OUTPUT_DIR/" 2>/dev/null || true

if [ $EXIT_CODE -eq 0 ]; then
    echo ""
    echo "=== SMOKE TEST PASSED ==="
else
    echo ""
    echo "=== SMOKE TEST FAILED (exit $EXIT_CODE) ==="
fi

exit $EXIT_CODE
