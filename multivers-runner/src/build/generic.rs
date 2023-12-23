use std::convert::Infallible;
use std::path::PathBuf;
use std::process::Command;

use super::{Build, Executable};

impl Executable for Build<'_> {
    unsafe fn exec(
        self,
        _argc: i32,
        _argv: *const *const i8,
        _envp: *const *const i8,
    ) -> Result<Infallible, proc_exit::Exit> {
        let mut args = std::env::args_os();
        let exe_filename = args
            .next()
            .map(PathBuf::from)
            .and_then(|path| path.file_name().map(ToOwned::to_owned))
            .unwrap_or_default();

        let mut file = tempfile::Builder::new()
            .suffix(&exe_filename)
            .tempfile()
            .map_err(|_| {
                proc_exit::Code::FAILURE.with_message("Failed to create a temporary file")
            })?;
        // Execution with a temporary file will likely fail since it is common for `/tmp` to be mounted with noexec.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            let metadata = file.as_file().metadata()?;
            let mut permissions = metadata.permissions();

            permissions.set_mode(0o700);
        }

        self.extract_into(&mut file).map_err(|_| {
            proc_exit::Code::FAILURE.with_message(format!(
                "Failed to write the build to the temporary file `{}`",
                file.path().display()
            ))
        })?;

        let path = file.into_temp_path();

        let exit_status = Command::new(&path).args(args).status().map_err(|_| {
            proc_exit::Code::FAILURE.with_message(format!(
                "Failed to execute temporary file `{}`",
                path.display()
            ))
        })?;

        proc_exit::Code::from_status(exit_status).process_exit()
    }
}
