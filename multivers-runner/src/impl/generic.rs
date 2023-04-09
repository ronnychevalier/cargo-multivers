use std::convert::Infallible;
use std::ffi::OsString;
use std::process::Command;

use crate::build::Build;

pub fn exec(build: Build, exe_filename: OsString) -> Result<Infallible, proc_exit::Exit> {
    let mut file = tempfile::Builder::new()
        .suffix(&exe_filename)
        .tempfile()
        .map_err(|_| proc_exit::Code::FAILURE.with_message("Failed to create a temporary file"))?;
    // Execution with a temporary file will likely fail since it is common for `/tmp` to be mounted with noexec.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let metadata = file.as_file().metadata()?;
        let mut permissions = metadata.permissions();

        permissions.set_mode(0o700);
    }

    build.decompress_into(&mut file).map_err(|_| {
        proc_exit::Code::FAILURE.with_message(format!(
            "Failed to write the build to the temporary file `{}`",
            file.path().display()
        ))
    })?;

    let path = file.into_temp_path();

    let exit_status = Command::new(&path)
        .args(std::env::args_os().skip(1))
        .status()
        .map_err(|_| {
            proc_exit::Code::FAILURE.with_message(format!(
                "Failed to execute temporary file `{}`",
                path.display()
            ))
        })?;

    proc_exit::Code::from_status(exit_status).process_exit()
}
