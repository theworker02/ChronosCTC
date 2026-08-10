<p align="center">
  <img src="https://raw.githubusercontent.com/theworker02/ChronosCTC/main/assets/logo.png" alt="Cronos-CTC logo" width="168"/>
</p>

<h1 align="center">Cronos-CTC</h1>

<p align="center">
  <strong>Chronal runtime for Deutsch-consistent closed timelike curves</strong><br/>
  A self-compiling spacetime engine — from fixed-point kernels to Novikov cosmologies.
</p>

<p align="center">
  <a href="https://theworker02.github.io/ChronosCTC/"><img alt="pages" src="https://img.shields.io/badge/GitHub%20Pages-live-2EE6D6?style=plastic"/></a>
  <a href="https://github.com/theworker02/ChronosCTC/actions/workflows/ci.yml"><img alt="ci" src="https://img.shields.io/github/actions/workflow/status/theworker02/ChronosCTC/ci.yml?branch=main&style=plastic&label=CI"/></a>
  <a href="https://crates.io/crates/ctc-cosmos"><img alt="crates.io" src="https://img.shields.io/crates/v/ctc-cosmos?style=plastic&color=149E96"/></a>
  <img alt="version" src="https://img.shields.io/badge/version-0.6.0-2EE6D6?style=plastic"/>
  <img alt="phase" src="https://img.shields.io/badge/phase-6%20Novikov-0B1220?style=plastic"/>
  <img alt="license" src="https://img.shields.io/badge/license-Apache%202.0-C9A227?style=plastic"/>
  <img alt="rust" src="https://img.shields.io/badge/rust-edition%202021-orange?style=plastic"/>
  <a href="SECURITY.md"><img alt="security" src="https://img.shields.io/badge/security-policy-149E96?style=plastic"/></a>
  <a href="PRIVACY.md"><img alt="privacy" src="https://img.shields.io/badge/privacy-policy-2EE6D6?style=plastic"/></a>
</p>

<p align="center">
  <a href="https://theworker02.github.io/ChronosCTC/">Website</a> ·
  <a href="https://github.com/theworker02/ChronosCTC/releases/tag/v0.6.0">v0.6.0 Release</a> ·
  <a href="docs/PUBLISHING.md">Publishing</a> ·
  <a href="CHANGELOG.md">Changelog</a>
</p>

---

## Overview

Cronos-CTC is an experimental **Rust workspace** that treats computation as a
problem on a spacetime manifold rather than a von Neumann instruction stream.

Retrocausal programs do not “step forward.” They converge to chronal states
\(\rho\) that satisfy the **Deutsch consistency condition**

\[
U(\rho)=\rho
\qquad\text{i.e.}\qquad
r(x)=F(x)-x=0
\]

where \(F\) is the CTC evolution map and \(r\) is the residual monitored by
`ctc-kernel`.

Across six phases the stack grows from a single Anderson fixed-point solver into
a **Novikov closed cosmos**: holographic boundary projection, Landauer
thermodynamics, Genesis law compilation \(\Lambda^\star\), and
horizon-persisted self-sustaining ticks.

> This is research / systems software inspired by CTC consistency literature.
> It is **not** a claim of physical time travel hardware.

## Features

- **Deutsch fixed-point kernel** — Anderson-accelerated multi-start basin search with unique / multi-weighted / paradox classification
- **Worldline DAG memory** — immutable spacetime fabric indexed by \((a,\tau)\)
- **Retrocausal DSL compiler** — cyclic dependency graphs → fixed-point equations
- **Hardware bridge** — FPGA / GPU / annealer preference routing with CPU fallback
- **Cross-epoch signalling** — Deutsch-gated teleportation across proper time
- **Multiversal ledger** — fork, score, and Proof-of-Consistency collapse
- **Holographic boundary** — AdS/CFT-style bulk→boundary compression (`ctc-holo`)
- **Landauer thermodynamics** — bit-erasure / prune energy accounting (`ctc-entropy`)
- **Genesis meta-compiler** — workload-driven rewrite of physical law parameters
- **Novikov runtime** — seal \(\Lambda^\star\) onto the live stack and sustain ticks (`ctc-cosmos`)

## Quick start

### Requirements

- Rust **1.75+** (edition 2021 workspace; newer stable recommended)
- Cargo, Git

### Clone & run

```bash
git clone https://github.com/theworker02/ChronosCTC.git
cd ChronosCTC
cargo test --workspace
cargo run -p ctc-cli
```

The CLI boots the Phase-6 Novikov lifecycle, then smokes Phase 5 / 4 / 2.

### Install the binary

```bash
cargo install --path crates/ctc-cli
cronos-ctc
```

### Configuration

Runtime knobs live in [`configs/runtime.toml`](configs/runtime.toml):

| Section | Controls |
|---------|----------|
| `[solver]` | Anderson depth, tolerance ε, restarts, domain |
| `[bridge]` | CTC / classical device preference & cost model |
| `[signal]` / `[mesh]` | Deutsch gate & hop latency |
| `[holo]` / `[entropy]` / `[genesis]` | Boundary ratio, Landauer T, meta-epochs |
| `[cosmos]` | Sustainment ticks, horizon checkpoints, zero-energy gate |

## Architecture

```text
ChronosCTC/
├── crates/
│   ├── ctc-kernel/      # Deutsch consistency solver & nonlinear runtime
│   ├── ctc-dag/         # Topological worldline memory fabric
│   ├── ctc-compiler/    # Cyclic graphs → fixed-point equations
│   ├── ctc-pruner/      # Paradox / residual branch pruner
│   ├── ctc-bridge/      # FPGA / GPU / annealer offload HAL
│   ├── ctc-inspector/   # τ-scrub debugger & residual telemetry
│   ├── ctc-gc/          # Entropy-aware timeline garbage collector
│   ├── ctc-signal/      # Cross-epoch binary teleportation
│   ├── ctc-oracle/      # Pre-cognitive branch interception
│   ├── ctc-mesh/        # Distributed temporal entanglement network
│   ├── ctc-ledger/      # Omniversal multi-timeline ledger
│   ├── ctc-agents/      # Cross-temporal navigation agents
│   ├── ctc-collapse/    # Proof-of-Consistency reality merger
│   ├── ctc-holo/        # AdS/CFT holographic boundary projection
│   ├── ctc-entropy/     # Landauer thermodynamic work extractor
│   ├── ctc-genesis/     # Self-referential physical-laws compiler
│   ├── ctc-horizon/     # Event-horizon persistence for sealed cosmos
│   ├── ctc-cosmos/      # Novikov closed-cosmos seal & tick runtime
│   └── ctc-cli/         # End-to-end demonstration driver (bin: cronos-ctc)
├── site/                # GitHub Pages source
├── configs/             # runtime.toml
├── scripts/             # publish / push helpers
└── releases/            # versioned release notes + checksums
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

### Cosmological lifecycle (Phase 6)

1. **Genesis** locks physical laws \(\Lambda^\star = G(W(\Lambda^\star))\)
2. **Seal** rewrites live solver ε, signal Deutsch gate, mesh hop latency, holo ratio, thermo/GC
3. **Tick** runs holographic boundary solves with Landauer ↔ GC coupling
4. **Horizon** checkpoints the sealed universe for process resurrection

```text
bulk DAG ──► ctc-holo boundary ──► ctc-entropy Landauer
                                         │
                                         ▼
                              ctc-genesis law fixed point Λ*
                                         │
                                         ▼
                         ctc-cosmos seal → sustain → ctc-horizon
```

## Crate index (crates.io)

| Crate | Role |
|-------|------|
| [`ctc-kernel`](https://crates.io/crates/ctc-kernel) | Anderson / Deutsch fixed-point solver |
| [`ctc-dag`](https://crates.io/crates/ctc-dag) | Worldline spacetime DAG |
| [`ctc-compiler`](https://crates.io/crates/ctc-compiler) | Retrocausal DSL lowering |
| [`ctc-pruner`](https://crates.io/crates/ctc-pruner) | Paradox branch pruner |
| [`ctc-bridge`](https://crates.io/crates/ctc-bridge) | Device offload HAL |
| [`ctc-inspector`](https://crates.io/crates/ctc-inspector) | Residual / τ debugger |
| [`ctc-gc`](https://crates.io/crates/ctc-gc) | Timeline garbage collector |
| [`ctc-signal`](https://crates.io/crates/ctc-signal) | Cross-epoch packets |
| [`ctc-oracle`](https://crates.io/crates/ctc-oracle) | Injection-point oracle |
| [`ctc-mesh`](https://crates.io/crates/ctc-mesh) | Distributed entanglement mesh |
| [`ctc-ledger`](https://crates.io/crates/ctc-ledger) | Omniversal fork ledger |
| [`ctc-agents`](https://crates.io/crates/ctc-agents) | Chronal agent fleet |
| [`ctc-collapse`](https://crates.io/crates/ctc-collapse) | PoC reality synthesis |
| [`ctc-holo`](https://crates.io/crates/ctc-holo) | Holographic projector |
| [`ctc-entropy`](https://crates.io/crates/ctc-entropy) | Landauer thermo balancer |
| [`ctc-genesis`](https://crates.io/crates/ctc-genesis) | Laws meta-compiler |
| [`ctc-horizon`](https://crates.io/crates/ctc-horizon) | Cosmos checkpoints |
| [`ctc-cosmos`](https://crates.io/crates/ctc-cosmos) | Sealed sustainment runtime |
| [`ctc-cli`](https://crates.io/crates/ctc-cli) | Demo binary `cronos-ctc` |

```bash
# Example dependency
[dependencies]
ctc-cosmos = "0.6"
ctc-kernel = "0.6"
```

## Development

```bash
cargo fmt
cargo clippy --workspace --all-targets
cargo test --workspace
cargo build -p ctc-cli --release
./scripts/publish-crates.sh --dry-run --allow-dirty
```

Branch convention for agents: `cursor/<descriptor>-d5a7`.

## Website & CI

| Surface | URL |
|---------|-----|
| GitHub Pages | https://theworker02.github.io/ChronosCTC/ |
| Actions | https://github.com/theworker02/ChronosCTC/actions |
| Releases | https://github.com/theworker02/ChronosCTC/releases |

Local site preview:

```bash
python3 -m http.server --directory site 8080
```

## Brand assets

| Asset | Path |
|-------|------|
| Raster logo (README / social) | [`assets/logo.png`](assets/logo.png) |
| Vector logo | [`assets/logo.svg`](assets/logo.svg) |
| Per-crate copy | `crates/<crate>/assets/logo.svg` |

## Governance & policies

| Document | Purpose |
|----------|---------|
| [`SECURITY.md`](SECURITY.md) | Vulnerability reporting |
| [`PRIVACY.md`](PRIVACY.md) | Privacy policy |
| [`CODE_OF_CONDUCT.md`](CODE_OF_CONDUCT.md) | Community standards |
| [`CONTRIBUTING.md`](CONTRIBUTING.md) | How to contribute |
| [`SUPPORT.md`](SUPPORT.md) | Where to get help |
| [`LICENSE`](LICENSE) | Apache-2.0 |
| [`NOTICE`](NOTICE) | Attribution |

## Publishing

See [`docs/PUBLISHING.md`](docs/PUBLISHING.md).

```bash
export CARGO_REGISTRY_TOKEN=…   # https://crates.io/settings/tokens
./scripts/publish-crates.sh
```

## License

Licensed under the [Apache License, Version 2.0](LICENSE).

```
Copyright 2026 Cronos-CTC Systems Architecture
```
