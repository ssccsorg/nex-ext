#!/usr/bin/env bash
set -euo pipefail
#
# nex-ext — Unified CI runner
#
# Mirrors the GitHub Actions gates in .github/workflows:
#   test.yml     Rust workspace (fmt, clippy, test)
#   ext.yml      Python runner harness (compileall, unittest)
#   deploy.yml   Gateway workers (npm ci, cf-typegen, typecheck)
#
# Usage:
#   ./run.sh               # Everything (default: rust + ext + gateway)
#   ./run.sh --rust        # Rust workspace checks only
#   ./run.sh --ext         # Python runner harness checks only
#   ./run.sh --gateway     # Gateway worker checks only
#   ./run.sh --help
#
# Container image builds (ext/ Dockerfiles) and worker deploys stay manual:
# they need Docker and Cloudflare credentials respectively.

cd "$(dirname "$0")"

# ── Rust workspace (mirrors .github/workflows/test.yml) ───────────────────

run_rust() {
    echo "--- fmt (check) ---"
    cargo fmt --check
    echo "--- clippy (workspace, warnings denied) ---"
    cargo clippy --workspace -- -D warnings
    echo "--- test (workspace) ---"
    cargo test --workspace
}

# ── Python runner harness (mirrors .github/workflows/ext.yml) ─────────────

run_ext() {
    echo "--- compileall ---"
    python3 -m compileall -q ext
    echo "--- unit tests ---"
    PYTHONPATH=ext python3 -m unittest discover -s ext/tests -t ext -v
}

# ── Gateway workers (mirrors .github/workflows/deploy.yml check job) ──────

gateway_worker() {
    local name="$1"
    echo "=== $name ==="
    (cd "gateway/$name" && npm ci --no-audit --no-fund)
    (cd "gateway/$name" && npm run cf-typegen)
    (cd "gateway/$name" && npm run typecheck)
}

run_gateway() {
    gateway_worker af-sync
    gateway_worker module-hub
}

# ── Dispatch ──────────────────────────────────────────────────────────────

case "${1:-}" in
    --rust)
        run_rust
        ;;
    --ext)
        run_ext
        ;;
    --gateway)
        run_gateway
        ;;
    --help|-h)
        echo "Usage: $0 [--rust|--ext|--gateway|--help]"
        exit 0
        ;;
    "")
        echo "=== Rust workspace ==="
        run_rust
        echo ""
        echo "=== Python runner harness (ext) ==="
        run_ext
        echo ""
        echo "=== Gateway workers ==="
        run_gateway
        ;;
    *)
        echo "Unknown: $1"
        echo "Usage: $0 [--rust|--ext|--gateway|--help]"
        exit 1
        ;;
esac
