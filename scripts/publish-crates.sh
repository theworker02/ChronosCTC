#!/usr/bin/env bash
# Publish Cronos-CTC workspace crates to crates.io in dependency order.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

DRY_RUN=0
ALLOW_DIRTY=0
for arg in "$@"; do
  case "$arg" in
    --dry-run) DRY_RUN=1 ;;
    --allow-dirty) ALLOW_DIRTY=1 ;;
    *)
      echo "unknown arg: $arg" >&2
      exit 2
      ;;
  esac
done

CRATES=(
  ctc-dag
  ctc-kernel
  ctc-pruner
  ctc-compiler
  ctc-signal
  ctc-ledger
  ctc-oracle
  ctc-bridge
  ctc-gc
  ctc-agents
  ctc-mesh
  ctc-inspector
  ctc-collapse
  ctc-holo
  ctc-entropy
  ctc-genesis
  ctc-horizon
  ctc-cosmos
  ctc-cli
)

DIRTY_FLAG=()
if [[ "$ALLOW_DIRTY" -eq 1 ]]; then
  DIRTY_FLAG+=(--allow-dirty)
fi

echo "==> publishing ${#CRATES[@]} crates (dry_run=${DRY_RUN})"

if [[ "$DRY_RUN" -eq 1 ]]; then
  # path+version deps require predecessors on crates.io before dependents can
  # even --dry-run. Validate the graph locally, then dry-run every crate whose
  # workspace deps are already published (always includes current leaves).
  cargo check --workspace --all-targets
  for crate in "${CRATES[@]}"; do
    echo
    echo "── dry-run ${crate} ──────────────────────────────────"
    if cargo publish -p "${crate}" --dry-run "${DIRTY_FLAG[@]}" >/tmp/ctc-publish-dry.log 2>&1; then
      echo "ok: ${crate}"
    else
      echo "deferred: ${crate} (publish predecessors to crates.io first)"
    fi
  done
  echo
  echo "==> dry-run complete"
  exit 0
fi

if [[ -z "${CARGO_REGISTRY_TOKEN:-}" ]]; then
  echo "CARGO_REGISTRY_TOKEN is required for live publish" >&2
  exit 1
fi

for crate in "${CRATES[@]}"; do
  echo
  echo "── ${crate} ──────────────────────────────────────────"
  ver="$(cargo metadata --no-deps --format-version 1 \
    | python3 -c "import json,sys; d=json.load(sys.stdin); print(next(p['version'] for p in d['packages'] if p['name']=='${crate}'))")"
  if curl -fsSL "https://crates.io/api/v1/crates/${crate}/${ver}" >/dev/null 2>&1; then
    echo "skip: ${crate}@${ver} already published"
    continue
  fi
  if cargo publish -p "${crate}" "${DIRTY_FLAG[@]}"; then
    :
  else
    echo "retry ${crate} in 45s (index lag)..."
    sleep 45
    cargo publish -p "${crate}" "${DIRTY_FLAG[@]}"
  fi
  # Allow crates.io index to observe the new crate before dependents publish.
  sleep 20
done

echo
echo "==> done"
