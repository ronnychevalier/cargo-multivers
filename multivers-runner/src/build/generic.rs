use std::convert::Infallible;
use std::ffi::c_char;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Command;

use super::{Build, Executable};

impl Executable for Build<'_> {
    unsafe fn exec(
        self,
        _argc: i32,
        _argv: *const *const c_char,
        _envp: *const *const c_char,
    ) -> Result<Infallible, proc_exit::Exit> {
        let mut args = std::env::args_os();
        let exe_filename = args
            .next()
            .map(PathBuf::from)
            .and_then(|path| path.file_name().map(ToOwned::to_owned))
            .unwrap_or_default();

        let mut builder = tempfile::Builder::new();
        builder.suffix(&exe_filename);

        #[cfg(unix)]
        builder.permissions(std::fs::Permissions::from_mode(0o700));

        let mut file = builder.tempfile().map_err(|_| {
            proc_exit::Code::FAILURE.with_message("Failed to create a temporary file")
        })?;

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
