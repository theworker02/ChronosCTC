# Publishing Cronos-CTC

## GitHub Pages

Site sources live in [`site/`](../site/). On every push to `main` that touches
site assets, [`.github/workflows/pages.yml`](../.github/workflows/pages.yml)
builds and deploys to **https://mloon25.github.io/cronos-ctc/**.

Enable once in the GitHub UI:

1. **Settings → Pages → Build and deployment → Source: GitHub Actions**
2. Push to `main` (or run the **GitHub Pages** workflow manually)

## crates.io

All 19 workspace crates are publishable at version `0.6.0`. Workspace path
dependencies also declare `version = "0.6.0"` so dependents resolve from the
registry after upload.

### Secrets

| Secret | Where | Purpose |
|--------|-------|---------|
| `CARGO_REGISTRY_TOKEN` | GitHub Actions + local env | `cargo publish` |
| `GITHUB_TOKEN` | Automatic in Actions | Pages + Releases |

Create a crates.io token at https://crates.io/settings/tokens with publish
scope for new crates / updates.

### Local publish

```bash
export CARGO_REGISTRY_TOKEN=…   # crates.io API token
./scripts/publish-crates.sh
```

Dry-run (only crates whose predecessors already exist on crates.io will fully
simulate upload; others are deferred):

```bash
./scripts/publish-crates.sh --dry-run --allow-dirty
```

### CI publish

- Tag `v*` → [release.yml](../.github/workflows/release.yml) creates a GitHub Release
- Manual / on GitHub Release → [publish-crates.yml](../.github/workflows/publish-crates.yml)

Publish order is computed topologically (see `scripts/publish-crates.sh`).
