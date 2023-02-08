#![feature(stdsimd)]

use std::io::Write;
use std::process::Command;

use anyhow::{Context, Result};

use bincode::config;

mod build;

use build::Build;

const BUILDS: &[u8] = include_bytes!(env!("CARGO_MULTIVERS_BUILDS_PATH"));

fn main() -> Result<()> {
    let supported_features: Vec<&str> = std_detect::detect::features()
        .filter_map(|(feature, supported)| supported.then_some(feature))
        .collect();

    let (builds, _): (Vec<Build>, _) = bincode::decode_from_slice(BUILDS, config::standard())
        .context("Failed to decode the builds")?;

    let build = builds
        .into_iter()
        .find_map(|build| {
            build
                .required_cpu_features()
                .iter()
                .all(|feature| supported_features.contains(&feature.as_str()))
                .then_some(build)
        })
        .ok_or_else(|| {
            anyhow::anyhow!("Failed to find a build satisfying the current CPU's features")
        })?;
    let build = build.decompress().context("Failed to decompress build")?;

    // On Linux, we first try with `memfd_create` and `execveat` to perform in-memory execution without
    // relying on temporary files.
    #[cfg(target_os = "linux")]
    {
        use std::ffi::CString;
        use std::fs::File;
        use std::os::fd::{FromRawFd, IntoRawFd};
        use std::os::unix::prelude::OsStringExt;

        use nix::sys::memfd::{memfd_create, MemFdCreateFlag};
        use nix::unistd::fexecve;

        let memfd_name = CString::new("cargo-multivers")?;
        let mut file = memfd_create(&memfd_name, MemFdCreateFlag::MFD_CLOEXEC)
            .map(|fd| unsafe { File::from_raw_fd(fd) })
            .context("Failed to create an anomymous memory file")?;
        file.write_all(&build)
            .context("Failed to write the build to an anomymous memory file")?;

        let args = std::env::args_os()
            .map(|arg| CString::new(arg.into_vec()).map_err(Into::into))
            .collect::<Result<Vec<CString>>>()?;
        let env = std::env::vars_os()
            .map(|(mut key, value)| {
                key.push("=");
                key.push(value);

                CString::new(key.into_vec()).map_err(Into::into)
            })
            .collect::<Result<Vec<CString>>>()?;
        let _ = fexecve(file.into_raw_fd(), &args, &env);
        // If fexecve failed, let's try with a temporary file.
    }

    let mut file = tempfile::NamedTempFile::new().context("Failed to create a temporary file")?;
    // On Linux, execution with a temporary file will likely fail since it is common for `/tmp` to be mounted with noexec.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let metadata = file.as_file().metadata()?;
        let mut permissions = metadata.permissions();

        permissions.set_mode(0o700);
    }

    file.write_all(&build).with_context(|| {
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
