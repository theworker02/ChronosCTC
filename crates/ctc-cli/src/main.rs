//! Cronos-CTC Phase 6 demonstration driver.
//!
//! Novikov closed cosmos:
//! 1. Genesis locks physical law vector Λ*
//! 2. Seal rewrites live solver / signal / mesh / holo / thermo / GC
//! 3. Holographic sustainment ticks with thermo↔GC coupling
//! 4. Horizon checkpoint persists the sealed universe

mod config;
mod phase2;
mod phase3;
mod phase4;
mod phase5;
mod phase6;

use phase2::ConvergenceObservationLoop;
use ctc_compiler::{lower, parse_module};

fn main() {
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║         Cronos-CTC Chronal Runtime v0.6.0 (Phase 6)      ║");
    println!("║  Cosmos · Horizon — Novikov Self-Sustaining Spacetime    ║");
    println!("╚══════════════════════════════════════════════════════════╝\n");

    let runtime = config::load_runtime_config();
    println!(
        "config: cosmos.ticks={}  holo.ratio={:.2}  genesis.meta_epochs={}\n",
        runtime.cosmos.sustain_ticks,
        runtime.holo.boundary_ratio,
        runtime.genesis.max_meta_epochs
    );

    phase6_novikov(&runtime);
    println!();
    phase5_smoke(&runtime);
    println!();
    phase4_smoke(&runtime);
    println!();
    phase2_smoke(&runtime);
}

fn phase6_novikov(runtime: &config::RuntimeConfig) {
    println!("── Phase-6: Novikov Closed Cosmos ─────────────────────────");
    println!("  Genesis Λ* → seal host → holographic ticks → horizon\n");

    let report = phase6::run_novikov_cosmos(runtime).expect("novikov cosmos");
    println!("  {}", report.message);
    println!("  sealed ε (signal) : {:.3e} → {:.3e}", report.seal.before_signal_eps, report.seal.after_signal_eps);
    println!("  sealed hop (mesh) : {}µs → {}µs", report.seal.before_hop_us, report.seal.after_hop_us);
    println!("  sealed boundary   : {:.3} → {:.3}", report.seal.before_boundary, report.seal.after_boundary);
    println!("  laws.ε            : {:.3e}", report.final_laws.deutsch_tolerance);
    println!("  laws.signal×      : {:.3}", report.final_laws.signal_speed);
    println!("  final residual    : {:.6e}", report.final_residual);
    println!("  zero-energy state : {}", report.zero_energy);
    println!("  sustainment ticks : {}", report.ticks.len());
    for t in &report.ticks {
        println!(
            "    tick {}: r={:.3e} S_EE={:.4} thermo={:.3e}J GC={} hops≈{}",
            t.tick,
            t.residual,
            t.holo_von_neumann,
            t.thermo_net_j,
            t.gc_nodes_pruned,
            t.teleports_simulated
        );
    }
    if let Some(id) = report.horizon_id {
        println!("  horizon checkpoint: #{id}");
    }
    if let Some(w) = report.multiverse_winner {
        println!("  multiverse winner : U{w}");
    }
}

fn phase5_smoke(runtime: &config::RuntimeConfig) {
    println!("── Phase-5 Smoke: Cosmological Bootstrap ──────────────────");
    let report = phase5::run_cosmological_lifecycle(runtime).expect("cosmos");
    println!(
        "  boundary={} S_EE={:.4} zero_E={} meta={}",
        report.holo_boundary_dim,
        report.holo_von_neumann,
        report.zero_energy,
        report.meta_epochs
    );
}

fn phase4_smoke(runtime: &config::RuntimeConfig) {
    println!("── Phase-4 Smoke: Multiversal Collapse ────────────────────");
    let report = phase4::run_multiverse_synthesis(runtime).expect("multiverse");
    println!(
        "  children={} winner=U{} pruned={}",
        report.bifurcation_children,
        report.winner,
        report.pruned.len()
    );
}

fn phase2_smoke(runtime: &config::RuntimeConfig) {
    println!("── Phase-2 Smoke: Bridge CTC ──────────────────────────────");
    let src = r#"
        chronal reg x @ τ0;
        chronal reg y @ τ1;
        retrocausal { y -> x };
        evolve x = 0.5 * x + 0.25 * y;
        evolve y = 0.25 * x + 0.5 * y + 0.5;
    "#;
    let compiled = lower(&parse_module("ctc", src).unwrap()).unwrap();
    let report = ConvergenceObservationLoop::from_runtime(runtime)
        .run(compiled)
        .unwrap();
    println!(
        "  device={:?} class={:?}",
        report.device_kind, report.solution.class
    );
}
