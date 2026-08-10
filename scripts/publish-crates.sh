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

UA="chronos-ctc-agent (https://github.com/theworker02/ChronosCTC)"

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

crate_published() {
  local crate="$1" ver="$2"
  curl -fsSL -H "User-Agent: ${UA}" \
    "https://crates.io/api/v1/crates/${crate}/${ver}" >/dev/null 2>&1
}

# Sleep until a crates.io rate-limit deadline.
wait_for_reset() {
  local iso="$1"
  local now epoch_now epoch_reset delta
  now=$(date -u +%s)
  epoch_reset=$(date -u -d "$iso" +%s 2>/dev/null || python3 -c \
    "import sys,datetime; print(int(datetime.datetime.fromisoformat(sys.argv[1].replace('Z','+00:00')).timestamp()))" \
    "$iso")
  delta=$((epoch_reset - now))
  if (( delta < 0 )); then delta=5; fi
  delta=$((delta + 8))   # buffer past the exact reset
  echo "rate limited; waiting ${delta}s until ${iso}…" >&2
  sleep "$delta"
}

publish_one() {
  local crate="$1"
  local out attempt
  for attempt in 1 2 3 4 5 6; do
    if out="$(cargo publish -p "${crate}" "${DIRTY_FLAG[@]}" 2>&1)"; then
      printf '%s\n' "$out"
      return 0
    fi
    printf '%s\n' "$out"
    if printf '%s' "$out" | grep -qE "already exists|already been published|is already uploaded"; then
      echo "note: ${crate} already present on crates.io"
      return 0
    fi
    # crates.io new-crate rate limit
    if printf '%s' "$out" | grep -q "status 429 Too Many Requests"; then
      local reset
      reset=$(printf '%s' "$out" | grep -oE "after [A-Za-z]{3}, [0-9]{2} [A-Za-z]{3} [0-9]{4} [0-9:]{8} GMT" \
        | sed 's/after //' | head -1)
      if [[ -n "$reset" ]]; then
        wait_for_reset "$(date -u -d "$reset" +%Y-%m-%dT%H:%M:%SZ 2>/dev/null || echo "$reset")"
      else
        echo "rate limited (no timestamp); waiting 120s…" >&2
        sleep 120
      fi
      continue
    fi
    # transient index lag / other 5xx → backoff
    if printf '%s' "$out" | grep -qE "status 5[0-9]{2}|failed to lookup address|Connection refused|index"; then
      local delay=$((attempt * 30))
      echo "transient error; sleeping ${delay}s…" >&2
      sleep "$delay"
      continue
    fi
    echo "fatal publish error for ${crate}" >&2
    return 1
  done
  echo "exhausted retries for ${crate}" >&2
  return 1
}

for crate in "${CRATES[@]}"; do
  echo
  echo "── ${crate} ──────────────────────────────────────────"
  ver="$(cargo metadata --no-deps --format-version 1 \
    | python3 -c "import json,sys; d=json.load(sys.stdin); print(next(p['version'] for p in d['packages'] if p['name']=='${crate}'))")"
  if crate_published "${crate}" "${ver}"; then
    echo "skip: ${crate}@${ver} already published"
    continue
  fi
  publish_one "${crate}"
  # Soft pacing between successful publishes to stay under new-crate limits.
  # crates.io rate-limits brand-new crates (~1 publish per 10 minutes).
  echo "pacing 540s before next crate…" >&2
  sleep 540
done

echo
echo "==> done"
