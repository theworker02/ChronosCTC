# Contributing to Cronos-CTC

Thanks for your interest in improving the chronal stack.

## Ground rules

- Be kind — see [`CODE_OF_CONDUCT.md`](CODE_OF_CONDUCT.md)
- Prefer small, reviewable PRs
- Math before code: document the fixed-point / consistency intent when changing solvers
- Do not add von Neumann sequential execution as the primary model

## Development setup

```bash
git clone https://github.com/theworker02/ChronosCTC.git
cd ChronosCTC
cargo test --workspace
cargo run -p ctc-cli
```

## Branch & commit style

- Feature branches: `cursor/<short-description>-d5a7` (agents) or `feat/<topic>`
- Commits: imperative mood, focused (`fix: …`, `feat: …`, `docs: …`)
- Keep `Cargo.lock` committed (binary workspace)

## Before you open a PR

```bash
cargo fmt
cargo test --workspace
cargo build -p ctc-cli --release
```

- Update crate `README.md` / root docs if you change public APIs
- Add or adjust unit tests near the math you touch
- Never commit secrets (`CARGO_REGISTRY_TOKEN`, PATs)

## Security reports

See [`SECURITY.md`](SECURITY.md) — do not file public issues for vulnerabilities.

## Publishing

Maintainers publish with [`scripts/publish-crates.sh`](scripts/publish-crates.sh). Contributors should not publish crates unless asked.
