# Changelog

All notable changes to Cronos-CTC are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.6.0] — 2026-08-10

### Added — Phase 6: Novikov Closed Cosmos

- **`ctc-cosmos`** — law sealing (`RuntimePatch` / `HostPhysics`) and sustainment tick loop
- **`ctc-horizon`** — event-horizon checkpoints for sealed \(\Lambda^\star\), energy ledger, and boundary
- Holographic `project_dag` / `boundary_solve` in `ctc-holo`
- Landauer ↔ GC coupling with real ledger cell counts in `ctc-entropy`
- CLI Phase-6 Novikov lifecycle demo
- Project README, brand logo (`assets/logo.svg` / `.png`), per-crate package icons

### Changed

- Workspace version bumped to **0.6.0**
- Runtime config gains `[cosmos]` sustainment / horizon knobs

## [0.5.0] — 2026-08-10

### Added — Phase 5: Holographic Boundary & Self-Compiling Spacetime

- **`ctc-holo`** — AdS/CFT-style boundary projection and entanglement spectrum
- **`ctc-entropy`** — Landauer thermodynamic balancer
- **`ctc-genesis`** — self-referential physical-laws meta-compiler \(\Lambda^\star = G(W(\Lambda^\star))\)

## [0.4.0] — 2026-08-10

### Added — Phase 4: Multiversal Consensus

- **`ctc-ledger`**, **`ctc-agents`**, **`ctc-collapse`** — omniversal forks, chronal agents, Proof-of-Consistency merger

## [0.3.0] — 2026-08-10

### Added — Phase 3: Retrocausal Teleportation

- **`ctc-signal`**, **`ctc-oracle`**, **`ctc-mesh`** — Deutsch-gated cross-epoch transport

## [0.2.0] — 2026-08-10

### Added — Phase 2: Bridge · Inspector · GC

- **`ctc-bridge`**, **`ctc-inspector`**, **`ctc-gc`** — hardware offload, τ-scrub telemetry, entropy GC

## [0.1.0] — 2026-08-10

### Added — Phase 1: Bootstrap

- **`ctc-kernel`**, **`ctc-dag`**, **`ctc-compiler`**, **`ctc-pruner`**, **`ctc-cli`**
- Anderson accelerated Deutsch fixed-point solver and worldline fabric

[0.6.0]: releases/v0.6.0/RELEASE_NOTES.md
[0.5.0]: CHANGELOG.md#050--2026-08-10
[0.4.0]: CHANGELOG.md#040--2026-08-10
[0.3.0]: CHANGELOG.md#030--2026-08-10
[0.2.0]: CHANGELOG.md#020--2026-08-10
[0.1.0]: CHANGELOG.md#010--2026-08-10
