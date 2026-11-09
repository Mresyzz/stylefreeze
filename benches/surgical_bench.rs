use std::time::Instant;
use stylefreeze::engine::SurgicalEngine;
use stylefreeze::lang::SupportedLang;

fn generate_sample_ts(lines: usize) -> (String, String) {
    let mut orig = String::new();
    let mut cand = String::new();

    orig.push_str("import { Service } from './service';\n\n");
    cand.push_str("import { Service } from './service';\n\n");

    for i in 0..lines {
        orig.push_str(&format!(
            "export function handler_{}(id: number): number {{\n    // Internal operation {}\n    const factor = {};\n    return id * factor;\n}}\n\n",
            i, i, i + 1
        ));

        // Candidate reformats to 2 spaces, deletes comments, and modifies the 50th function
        let factor_val = if i == lines / 2 { i + 999 } else { i + 1 };
        cand.push_str(&format!(
            "export function handler_{}(id: number): number {{\n  const factor = {};\n  return id * factor;\n}}\n\n",
            i, factor_val
        ));
    }

    (orig, cand)
}

fn main() {
    println!("=== StyleFreeze Micro-Benchmark ===");

    for &size in &[50, 200, 500] {
        let (orig, cand) = generate_sample_ts(size);
        let engine = SurgicalEngine::new(SupportedLang::TypeScript);

        // Warm up
        let _ = engine.reconcile(&orig, &cand);

        let start = Instant::now();
        let iters = 20;
        for _ in 0..iters {
            let _ = engine.reconcile(&orig, &cand);
        }
        let elapsed = start.elapsed();
        let avg_ms = (elapsed.as_secs_f64() * 1000.0) / iters as f64;

        let total_lines = size * 6;
        println!(
            "Scale: {:>5} lines of TS | Avg reconciliation latency: {:>6.2} ms",
            total_lines, avg_ms
        );
    }
}
