//! Cronos-CTC Phase 5 demonstration driver.
//!
//! Cosmological lifecycle:
//! 1. Boundary Encoding — project bulk DAG onto holographic screen
//! 2. Thermodynamic Equilibrium — Landauer work of prune/converge
//! 3. Reality Compilation — Genesis locks physical law vector Λ*
//! 4. Self-Sustaining Execution — closed retrocausal cosmos

mod config;
mod phase2;
mod phase3;
mod phase4;
mod phase5;

use phase2::ConvergenceObservationLoop;
use ctc_compiler::{lower, parse_module};

fn main() {
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║         Cronos-CTC Chronal Runtime v0.5.0 (Phase 5)      ║");
    println!("║  Holo · Entropy · Genesis — Self-Compiling Spacetime     ║");
    println!("╚══════════════════════════════════════════════════════════╝\n");

    let runtime = config::load_runtime_config();
    println!(
        "config: holo.ratio={:.2}  T={}K  genesis.meta_epochs={}\n",
        runtime.holo.boundary_ratio,
        runtime.entropy.temperature_k,
        runtime.genesis.max_meta_epochs
    );

    phase5_cosmos(&runtime);
    println!();
    phase4_smoke(&runtime);
    println!();
    phase2_smoke(&runtime);
}

fn phase5_cosmos(runtime: &config::RuntimeConfig) {
    println!("── Phase-5: Cosmological Bootstrap ────────────────────────");
    println!("  bulk→boundary→Landauer→Genesis Λ* fixed point\n");

    let report = phase5::run_cosmological_lifecycle(runtime).expect("cosmos");
    println!("  {}", report.message);
    println!("  boundary dim      : {}", report.holo_boundary_dim);
    println!("  von Neumann S_EE  : {:.6}", report.holo_von_neumann);
    println!("  Ryu–Takayanagi    : {:.6}", report.rt_entropy);
    println!("  thermo net work   : {:.6e} J", report.thermo_net_j);
    println!("  zero-energy state : {}", report.zero_energy);
    println!("  laws.ε            : {:.3e}", report.laws_deutsch);
    println!("  laws.signal×      : {:.3}", report.laws_signal);
    println!("  laws.boundary     : {:.3}", report.laws_boundary);
    println!(
        "  meta-epochs       : {} (converged={})",
        report.meta_epochs, report.meta_converged
    );
    if let Some(w) = report.multiverse_winner {
        println!("  multiverse winner : U{w}");
    }
}

fn phase4_smoke(runtime: &config::RuntimeConfig) {
    println!("── Phase-4 Smoke: Multiversal Collapse ────────────────────");
    let report = phase4::run_multiverse_synthesis(runtime).expect("multiverse");
    println!(
        "  children={} winner=U{} primary={:?} pruned={}",
        report.bifurcation_children,
        report.winner,
        report.primary_state,
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
