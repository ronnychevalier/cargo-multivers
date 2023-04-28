#![feature(stdsimd)]
#![no_main]

mod build;

use build::{Build, Executable};

#[no_mangle]
pub unsafe fn main(argc: i32, argv: *const *const i8, envp: *const *const i8) {
    let result = run(argc, argv, envp);

    proc_exit::exit(result);
}

unsafe fn run(argc: i32, argv: *const *const i8, envp: *const *const i8) -> proc_exit::ExitResult {
    Build::find().exec(argc, argv, envp)?;

    Ok(())
}
