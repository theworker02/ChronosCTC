<p align="center">
  <img src="assets/logo.svg" alt="Cronos-CTC logo" width="160" height="160"/>
</p>

<h1 align="center">Cronos-CTC</h1>

<p align="center">
  <strong>Chronal runtime for Deutsch-consistent closed timelike curves</strong><br/>
  A self-compiling spacetime engine — from fixed-point kernels to Novikov cosmologies.
</p>

<p align="center">
  <a href="https://mloon25.github.io/cronos-ctc/"><img alt="pages" src="https://img.shields.io/badge/GitHub%20Pages-live-2EE6D6?style=for-the-badge"/></a>
  <a href="https://github.com/Mloon25/cronos-ctc/actions/workflows/ci.yml"><img alt="ci" src="https://img.shields.io/github/actions/workflow/status/Mloon25/cronos-ctc/ci.yml?branch=main&style=for-the-badge&label=CI"/></a>
  <a href="https://crates.io/crates/ctc-cosmos"><img alt="crates.io" src="https://img.shields.io/crates/v/ctc-cosmos?style=for-the-badge&color=149E96"/></a>
  <img alt="version" src="https://img.shields.io/badge/version-0.6.0-2EE6D6?style=for-the-badge"/>
  <img alt="phase" src="https://img.shields.io/badge/phase-6%20Novikov-0B1220?style=for-the-badge"/>
  <img alt="license" src="https://img.shields.io/badge/license-Apache%202.0-C9A227?style=for-the-badge"/>
  <img alt="rust" src="https://img.shields.io/badge/rust-edition%202021-orange?style=for-the-badge"/>
</p>

---

## What it is

Cronos-CTC replaces the instruction pointer with a **nonlinear Deutsch fixed-point solver**.
Retrocausal programs do not step forward in time — they converge to states \(\rho\) satisfying

\[
U(\rho)=\rho
\]

Across six phases the runtime grows from a single CTC kernel into a **Novikov closed cosmos**:
holographic boundary projection, Landauer thermodynamics, Genesis law compilation, and
horizon-persisted self-sustaining ticks.

## Quick start

```bash
# Requires a recent Rust toolchain (1.75+ recommended)
cargo test --workspace
cargo run -p ctc-cli
```

Configuration lives in [`configs/runtime.toml`](configs/runtime.toml).

## Architecture

```text
cronos-ctc/
├── ctc-kernel/      # Deutsch consistency solver & nonlinear runtime
├── ctc-dag/         # Topological worldline memory fabric
├── ctc-compiler/    # Cyclic graphs → fixed-point equations
├── ctc-pruner/      # Paradox / residual branch pruner
├── ctc-bridge/      # FPGA / GPU / annealer offload HAL
├── ctc-inspector/   # τ-scrub debugger & residual telemetry
├── ctc-gc/          # Entropy-aware timeline garbage collector
├── ctc-signal/      # Cross-epoch binary teleportation
├── ctc-oracle/      # Pre-cognitive branch interception
├── ctc-mesh/        # Distributed temporal entanglement network
├── ctc-ledger/      # Omniversal multi-timeline ledger
├── ctc-agents/      # Cross-temporal navigation agents
├── ctc-collapse/    # Proof-of-Consistency reality merger
├── ctc-holo/        # AdS/CFT holographic boundary projection
├── ctc-entropy/     # Landauer thermodynamic work extractor
├── ctc-genesis/     # Self-referential physical-laws compiler
├── ctc-horizon/     # Event-horizon persistence for sealed cosmos
├── ctc-cosmos/      # Novikov closed-cosmos seal & tick runtime
└── ctc-cli/         # End-to-end demonstration driver
```

### Phase map

| Phase | Theme | Crates |
|------:|-------|--------|
| 1 | Fixed-point CTC kernel | `ctc-kernel`, `ctc-dag`, `ctc-compiler`, `ctc-pruner` |
| 2 | Bridge · Inspector · GC | `ctc-bridge`, `ctc-inspector`, `ctc-gc` |
| 3 | Retrocausal teleportation | `ctc-signal`, `ctc-oracle`, `ctc-mesh` |
| 4 | Multiversal consensus | `ctc-ledger`, `ctc-agents`, `ctc-collapse` |
| 5 | Holography · Thermo · Genesis | `ctc-holo`, `ctc-entropy`, `ctc-genesis` |
| 6 | Novikov closed cosmos | `ctc-cosmos`, `ctc-horizon` |

## Cosmological lifecycle (Phase 6)

1. **Genesis** locks physical laws \(\Lambda^\star\)
2. **Seal** rewrites live solver ε, signal Deutsch gate, mesh hop latency, holo ratio, thermo/GC
3. **Tick** runs holographic boundary solves with Landauer ↔ GC coupling
4. **Horizon** checkpoints the sealed universe for process resurrection

## Website

The project site is built from [`site/`](site/) and deployed by GitHub Actions to

**https://mloon25.github.io/cronos-ctc/**

```bash
# local preview
python3 -m http.server --directory site 8080
```

## Publishing

Crates publish via [`scripts/publish-crates.sh`](scripts/publish-crates.sh) (also `.github/workflows/publish-crates.yml`).

Required secrets:
- `CARGO_REGISTRY_TOKEN` — crates.io API token
- GitHub Actions `GITHUB_TOKEN` — pages/release (automatic)

```bash
# dry-run locally
./scripts/publish-crates.sh --dry-run --allow-dirty
```

## Brand assets

| Asset | Path |
|-------|------|
| Vector logo | [`assets/logo.svg`](assets/logo.svg) |
| Raster logo | [`assets/logo.png`](assets/logo.png) |
| Package icon copy | `crates/<crate>/assets/logo.svg` |

Every workspace crate ships the same mark under `assets/` for docs, crates.io metadata, and UI chrome.

## Release

See [`CHANGELOG.md`](CHANGELOG.md) and [`releases/v0.6.0/`](releases/v0.6.0/) for the Phase 6 release notes and artifacts.

```bash
git checkout main
git tag -l 'v*'
cargo run -p ctc-cli
```

## License

Licensed under the [Apache License, Version 2.0](LICENSE).
