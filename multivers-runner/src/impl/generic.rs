use std::convert::Infallible;
use std::ffi::OsString;
use std::process::Command;

use anyhow::{Context, Result};

use crate::build::Build;

pub fn exec(build: Build, exe_filename: OsString) -> Result<Infallible> {
    let mut file = tempfile::Builder::new()
        .suffix(&exe_filename)
        .tempfile()
        .context("Failed to create a temporary file")?;
    // Execution with a temporary file will likely fail since it is common for `/tmp` to be mounted with noexec.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let metadata = file.as_file().metadata()?;
        let mut permissions = metadata.permissions();

        permissions.set_mode(0o700);
    }

    build.decompress_into(&mut file).with_context(|| {
        format!(
            "Failed to write the build to the temporary file `{}`",
            file.path().display()
        )
    })?;

    let path = file.into_temp_path();

    let exit_status = Command::new(&path)
        .args(std::env::args_os().skip(1))
        .status()
        .with_context(|| format!("Failed to execute temporary file `{}`", path.display()))?;

    std::process::exit(exit_status.code().unwrap_or_default());
}
