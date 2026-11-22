#!/usr/bin/env bash
# Shared checks for Git hooks, contributors, and local pull-request review.
set -euo pipefail

root="$(git rev-parse --show-toplevel)"
cd "$root"
mode="${1:-full}"
if [[ "$mode" != fast && "$mode" != full ]]; then
  echo "Usage: bash scripts/verify.sh [fast|full]" >&2
  exit 2
fi

cargo fmt --all -- --check
cargo check --locked --workspace

if [[ "$mode" == fast ]]; then
  echo "Span fast checks passed."
  exit 0
fi

cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo test --locked --workspace
cargo test --locked --workspace --all-features
cargo test --locked -p span-core --no-default-features
cargo build --locked --release -p span

python="${PYTHON:-python3}"
"$python" -m unittest discover -s scripts -p 'test_capture_hook.py'

# Do not inherit real database paths or external inference endpoints during checks.
export SPAN_EMBEDDER=hash
unset SPAN_RERANK_URL SPAN_RERANK_TOKEN SPAN_RERANK_MODEL SPAN_VECTOR_INDEX
temporary="$(mktemp -d "${TMPDIR:-/tmp}/span-verify.XXXXXXXX")"
trap 'rm -f "$temporary/memory.db" "$temporary/memory.db-shm" "$temporary/memory.db-wal" "$temporary/memory.db-journal"; rmdir "$temporary"' EXIT
export SPAN_DB="$temporary/memory.db"

target="${CARGO_TARGET_DIR:-$root/target}"
binary="$target/release/span"
"$binary" paths --json
"$binary" setup all --dry-run
"$python" scripts/smoke_mcp.py "$binary"
echo "Span full verification passed."
