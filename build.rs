//! Build script to remove warnings when using `cfg(coverage)`

fn main() {
    println!("cargo:rustc-check-cfg=cfg(coverage)");
}
