# Cronos-CTC v0.6.0 — Novikov Closed Cosmos

<p align="center">
  <img src="../../assets/logo.svg" alt="Cronos-CTC" width="120"/>
</p>

**Release date:** 2026-08-10  
**Codename:** Novikov  
**License:** Apache-2.0

## Highlights

- Sealed physical laws \(\Lambda^\star\) now rewrite the **live** solver, signal Deutsch gate, mesh hop latency, holographic ratio, and thermo/GC stack (`ctc-cosmos`).
- Sustainment ticks couple holographic boundary solves to Landauer accounting and entropy GC pressure.
- `ctc-horizon` persists sealed cosmos state across process epochs.
- Shared brand mark shipped under `assets/` and every crate’s `assets/logo.svg`.

## Install / run

```bash
git checkout v0.6.0
cargo test --workspace
cargo run -p ctc-cli
```

## Artifacts

| File | Description |
|------|-------------|
| `RELEASE_NOTES.md` | This document |
| `SHA256SUMS` | Checksums for release tree + built binary (when present) |
| `cronos-ctc` | Demo binary (built during release packaging) |

## Verification snapshot

```text
cargo test --workspace   # green
cargo run -p ctc-cli     # Phase-6 Novikov demo → zero_E=true
```

## Crates (19)

`ctc-kernel` · `ctc-dag` · `ctc-compiler` · `ctc-pruner` · `ctc-bridge` · `ctc-inspector` · `ctc-gc` · `ctc-signal` · `ctc-oracle` · `ctc-mesh` · `ctc-ledger` · `ctc-agents` · `ctc-collapse` · `ctc-holo` · `ctc-entropy` · `ctc-genesis` · `ctc-horizon` · `ctc-cosmos` · `ctc-cli`
