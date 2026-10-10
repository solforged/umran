//! `cargo run --release -p umran-web --example forming` reports seed 21.
//! `--json` emits the same four frame views for native/WASM parity.
use serde_json::Value;
use std::time::Instant;
use umran_web::Bench;

fn main() {
    let json_only = std::env::args().any(|arg| arg == "--json");
    for size in ["small", "medium", "large", "vast"] {
        let bench = Bench::new(21, size).unwrap();
        let start = Instant::now();
        let json = bench.forming().unwrap();
        let elapsed = start.elapsed();
        if json_only {
            println!("{json}");
            continue;
        }
        let forming: Value = serde_json::from_str(&json).unwrap();
        let map: Value = serde_json::from_str(&bench.map().unwrap()).unwrap();
        let regions = map["regions"].as_array().unwrap();
        println!(
            "seed 21 {size}: forming {:.3} ms, {} regions",
            elapsed.as_secs_f64() * 1000.0,
            regions.len()
        );
        println!("frame  land share  land bodies");
        for (step, frame) in forming["frames"].as_array().unwrap().iter().enumerate() {
            let land = frame["land"].as_str().unwrap().as_bytes();
            let mut seen = vec![false; land.len()];
            let mut bodies = 0;
            for root in 0..land.len() {
                if seen[root] || land[root] != b'1' {
                    continue;
                }
                bodies += 1;
                let mut stack = vec![root];
                seen[root] = true;
                while let Some(r) = stack.pop() {
                    for neighbour in regions[r]["neighbours"].as_array().unwrap() {
                        let n = neighbour.as_u64().unwrap() as usize;
                        if land[n] == b'1' && !seen[n] {
                            seen[n] = true;
                            stack.push(n);
                        }
                    }
                }
            }
            let share = land.iter().filter(|&&c| c == b'1').count() as f64 / land.len() as f64;
            println!("{step:>5}  {share:>10.4}  {bodies:>11}");
        }
    }
}
