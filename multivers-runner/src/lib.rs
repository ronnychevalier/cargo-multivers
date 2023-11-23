//! # `multivers-runner`
//!
//! This crate can be used to create a portable binary that embeds multiple versions of an executable each using a different CPU feature set.
//!
//! Take a look at [`cargo multivers`][cargo-multivers], it does all the work for you: build the multiple versions and build the final binary that embeds them.
//!
//! ## How Does it Work?
//!
//! The build script parses a JSON description file (see an example below) that contains a set of paths to executables with their dependency on CPU features
//! from the environment variable `MULTIVERS_BUILDS_DESCRIPTION_PATH`.
//! Then, it generates a Rust file that contains a compressed source binary and compressed binary patches to regenerate the other binaries from the source.
//!  
//! ```json
//! {
//!   "builds": [
//!     {
//!       "path": "/path/to/binary-with-additional-cpu-features",
//!       "features": [
//!         "aes",
//!         "avx",
//!         "avx2",
//!         "sse",
//!         "sse2",
//!         "sse3",
//!         "sse4.1",
//!         "sse4.2",
//!         "ssse3",
//!       ]
//!     },
//!     {
//!       "path": "/path/to/binary-source",
//!       "features": [
//!         "sse",
//!         "sse2"
//!       ]
//!     }
//!   ]
//! }
//! ```
//!
//! At runtime, the function [main] uncompresses and executes the version that matches the CPU features of the host.
//! On Linux, it uses `memfd_create` and `fexecve` to do an in-memory execution.
//! On Windows, however, it writes the version in a temporary file and executes it.
//!
//! # `cargo multivers`
//!
//! This library is used by [`cargo multivers`][cargo-multivers] to build the final binary that embeds the multiple versions.
//!
//! [cargo-multivers]: https://crates.io/crates/cargo-multivers
#![feature(stdarch_internal)]
#![cfg_attr(not(test), no_main)]
#![cfg_attr(test, allow(dead_code))]

mod build;

use build::{Build, Executable};

/// Function called at program startup.
///
/// When [main] is executed, it uncompresses and executes the version that matches the CPU features
/// of the host.
///
/// # Example
///
/// ```no_run
/// #![no_main]
///
/// pub use multivers_runner::main;
/// ```
///
/// # Safety
///
/// - `argc` must never be negative.
/// - `argv` and `envp` must be null-terminated arrays of valid pointers to null-terminated strings.
/// - Each element of `argv` and `envp` must be valid for reads of bytes up to and including the null terminator.
#[no_mangle]
#[cfg(not(test))]
pub unsafe extern "C" fn main(argc: i32, argv: *const *const i8, envp: *const *const i8) {
    let result = run(argc, argv, envp);

    proc_exit::exit(result);
}

unsafe fn run(argc: i32, argv: *const *const i8, envp: *const *const i8) -> proc_exit::ExitResult {
    Build::find().exec(argc, argv, envp)?;

    Ok(())
}
