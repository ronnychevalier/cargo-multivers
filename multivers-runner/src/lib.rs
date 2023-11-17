#![feature(stdarch_internal)]
#![no_main]

mod build;

use build::{Build, Executable};

/// Function called at program startup.
///
/// # Safety
///
/// - `argc` must never be negative.
/// - `argv` and `envp` must be null-terminated arrays of valid pointers to null-terminated strings.
/// - Each element of `argv` and `envp` must be valid for reads of bytes up to and including the null terminator.
#[no_mangle]
pub unsafe extern "C" fn main(argc: i32, argv: *const *const i8, envp: *const *const i8) {
    let result = run(argc, argv, envp);

    proc_exit::exit(result);
}

unsafe fn run(argc: i32, argv: *const *const i8, envp: *const *const i8) -> proc_exit::ExitResult {
    Build::find().exec(argc, argv, envp)?;

    Ok(())
}
