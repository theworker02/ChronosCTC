#!/usr/bin/env bash
# Push main + v0.6.0 to GitHub, then publish all crates to crates.io.
# Requires:
#   export GH_TOKEN=...                 # classic PAT with repo + workflow
#   export CARGO_REGISTRY_TOKEN=...     # crates.io API token
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

REMOTE_URL="${REMOTE_URL:-https://github.com/theworker02/ChronosCTC.git}"

if [[ -z "${GH_TOKEN:-}" ]]; then
  echo "GH_TOKEN is required (GitHub PAT with repo + workflow scopes)" >&2
  exit 1
fi
if [[ -z "${CARGO_REGISTRY_TOKEN:-}" ]]; then
  echo "CARGO_REGISTRY_TOKEN is required (crates.io API token)" >&2
  exit 1
fi

AUTH_URL="https://x-access-token:${GH_TOKEN}@github.com/theworker02/ChronosCTC.git"
git remote remove origin 2>/dev/null || true
git remote add origin "$REMOTE_URL"
git remote set-url origin "$AUTH_URL"

echo "==> pushing main"
git push -u origin main

echo "==> pushing tag v0.6.0"
git push origin v0.6.0

# Restore non-token remote URL so the token is not persisted in .git/config
git remote set-url origin "$REMOTE_URL"

echo "==> configuring gh"
printf '%s\n' "$GH_TOKEN" | gh auth login --with-token
gh auth setup-git

echo "==> enabling GitHub Pages (Actions source)"
gh api -X PUT "repos/theworker02/ChronosCTC/pages" \
  -f build_type=workflow \
  -f source='{"branch":"main","path":"/"}' 2>/dev/null \
  || gh api -X POST "repos/theworker02/ChronosCTC/pages" \
       -f build_type=workflow \
       -f source='{"branch":"main","path":"/"}' 2>/dev/null \
  || echo "warn: enable Pages manually at Settings → Pages → GitHub Actions"

echo "==> set Actions secret CARGO_REGISTRY_TOKEN"
gh secret set CARGO_REGISTRY_TOKEN --repo theworker02/ChronosCTC --body "$CARGO_REGISTRY_TOKEN"

echo "==> trigger Pages + publish workflows"
gh workflow run pages.yml --repo theworker02/ChronosCTC || true
gh workflow run publish-crates.yml --repo theworker02/ChronosCTC -f dry_run=false || true

echo "==> local crates.io publish (authoritative)"
./scripts/publish-crates.sh

echo "==> done"
echo "Site:   https://theworker02.github.io/ChronosCTC/"
echo "Repo:   https://github.com/theworker02/ChronosCTC"
echo "Crates: https://crates.io/search?q=ctc-"
