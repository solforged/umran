//! cargo run --release -p umran-sim --example names_band
#[path = "../tests/support/naming.rs"]
mod naming;

fn main() {
    naming::band(40, 200, true);
}
