const CRATES = [
  ["ctc-kernel", "Deutsch fixed-point chronal solver"],
  ["ctc-dag", "Worldline spacetime memory fabric"],
  ["ctc-compiler", "Retrocausal DSL → fixed-point math"],
  ["ctc-pruner", "Paradox and residual branch pruner"],
  ["ctc-bridge", "FPGA / GPU / annealer offload HAL"],
  ["ctc-inspector", "τ-scrub debugger & residual telemetry"],
  ["ctc-gc", "Entropy-aware timeline garbage collector"],
  ["ctc-signal", "Cross-epoch binary teleportation"],
  ["ctc-oracle", "Pre-cognitive branch interception"],
  ["ctc-mesh", "Distributed temporal entanglement"],
  ["ctc-ledger", "Omniversal multi-timeline ledger"],
  ["ctc-agents", "Cross-temporal navigation agents"],
  ["ctc-collapse", "Proof-of-Consistency reality merger"],
  ["ctc-holo", "AdS/CFT holographic boundary projection"],
  ["ctc-entropy", "Landauer thermodynamic balancer"],
  ["ctc-genesis", "Self-referential physical-laws compiler"],
  ["ctc-horizon", "Event-horizon cosmos persistence"],
  ["ctc-cosmos", "Novikov closed-cosmos runtime"],
  ["ctc-cli", "End-to-end demonstration driver"],
];

const grid = document.getElementById("crate-grid");
if (grid) {
  for (const [name, blurb] of CRATES) {
    const a = document.createElement("a");
    a.className = "crate";
    a.href = `https://crates.io/crates/${name}`;
    a.innerHTML = `<strong>${name}</strong><span>${blurb}</span>`;
    grid.appendChild(a);
  }
}
