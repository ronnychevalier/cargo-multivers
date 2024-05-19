#![doc = "README.md"]
#![feature(stdarch_internal)]
#![allow(internal_features)]
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
    let result = unsafe { run(argc, argv, envp) };

    proc_exit::exit(result);
}

unsafe fn run(argc: i32, argv: *const *const i8, envp: *const *const i8) -> proc_exit::ExitResult {
    let build = Build::find().unwrap_or_default();

    unsafe { build.exec(argc, argv, envp) }?;

    Ok(())
}
