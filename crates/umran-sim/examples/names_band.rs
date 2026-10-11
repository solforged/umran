//! cargo run --release -p umran-sim --example names_band
#[path = "../tests/support/naming.rs"]
mod naming;

use umran_sim::MapSize;

fn main() {
    naming::band(40, 200, MapSize::Medium, true);
    naming::band(8, 200, MapSize::Vast, true);
}
