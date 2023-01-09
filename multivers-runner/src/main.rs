#![feature(stdsimd)]

use std::io::Write;
use std::process::Command;

use anyhow::Context;

use bincode::config;

use flate2::read::DeflateDecoder;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

mod build;

use build::Build;

const BUILDS: &[u8] = include_bytes!(env!("CARGO_MULTIVERS_BUILDS_PATH"));

fn main() -> anyhow::Result<()> {
    let supported_features: Vec<&str> = std_detect::detect::features()
        .filter_map(|(feature, supported)| supported.then_some(feature))
        .collect();

    let config = config::standard();
    let mut decoder = DeflateDecoder::new(BUILDS);
    let builds: Vec<Build> = bincode::decode_from_std_read(&mut decoder, config)
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

    let mut file = tempfile::NamedTempFile::new().context("Failed to create a temporary file")?;
    #[cfg(unix)]
    {
        let metadata = file.as_file().metadata()?;
        let mut permissions = metadata.permissions();

        permissions.set_mode(0o700);
    }

    file.write_all(build.as_bytes()).with_context(|| {
        format!(
            "Failed to write the build to the temporary file `{}`",
            file.path().display()
        )
    })?;

    let path = file.into_temp_path();

    let exit_status = Command::new(&path)
        .args(std::env::args_os())
        .status()
        .with_context(|| format!("Failed to execute temporary file `{}`", path.display()))?;

    std::process::exit(exit_status.code().unwrap_or_default());
}
