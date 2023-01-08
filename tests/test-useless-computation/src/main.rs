use std::time::Instant;

use sha3::{Digest, Sha3_256};

fn main() {
    let start = Instant::now();
    let result: u64 = (0..3000000)
        .into_iter()
        .filter_map(|i| {
            let string = format!("useless computation {i}");
            let digest = Sha3_256::digest(string.as_bytes());
            let value = digest.get(20)?;
            Some(u64::from(*value))
        })
        .sum();

    let elapsed = start.elapsed().as_secs_f64();
    println!("useless result = {result}; in {elapsed}s");
}
