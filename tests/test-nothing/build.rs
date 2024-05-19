fn main() {
    let target = std::env::var("TARGET").unwrap_or_default();
    if target.contains("windows") {
        println!("cargo:rustc-link-arg-bins=/NODEFAULTLIB");
        println!("cargo:rustc-link-arg-bins=/EMITPOGOPHASEINFO");
        println!("cargo:rustc-link-arg-bins=/DEBUG:NONE");
        println!("cargo:rustc-link-arg-bins=/MERGE:.rdata=.text");
        println!("cargo:rustc-link-arg-bins=/MERGE:.pdata=.text");
    } else if target.contains("linux") {
        println!("cargo:rustc-link-arg-bins=-static");
        println!("cargo:rustc-link-arg-bins=-nostdlib");
        if std::env::var("CARGO_CFG_COVERAGE").is_ok() {
            println!("cargo:rustc-link-arg-bins=-lc");
        }
        println!("cargo:rustc-link-arg-bins=-Wl,-n,-N,--no-dynamic-linker,--build-id=none");
    }
}
