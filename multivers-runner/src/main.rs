#![feature(stdsimd)]
use std::path::PathBuf;

mod build;
mod r#impl;

use build::Build;
use r#impl::exec;

fn main() {
    let result = run();

    proc_exit::exit(result);
}

fn run() -> proc_exit::ExitResult {
    let build = Build::find().ok_or_else(|| {
        proc_exit::Code::FAILURE
            .with_message("Failed to find a build satisfying the host CPU features")
    })?;

    let exe_filename = std::env::args_os()
        .next()
        .map(PathBuf::from)
        .and_then(|path| path.file_name().map(ToOwned::to_owned))
        .unwrap_or_default();

    exec(build, exe_filename)?;

    Ok(())
}
