#![feature(stdsimd)]
use std::path::PathBuf;

use anyhow::Result;

mod build;
mod r#impl;

use build::Build;
use r#impl::exec;

include!(concat!(env!("OUT_DIR"), "/builds.rs"));

fn main() -> Result<()> {
    let supported_features: Vec<&str> = std_detect::detect::features()
        .filter_map(|(feature, supported)| supported.then_some(feature))
        .collect();

    let build = BUILDS
        .into_iter()
        .find_map(|build| {
            build
                .required_cpu_features()
                .iter()
                .all(|feature| supported_features.contains(feature))
                .then_some(build)
        })
        .ok_or_else(|| {
            anyhow::anyhow!("Failed to find a build satisfying the current CPU's features")
        })?;

    let exe_filename = std::env::args_os()
        .next()
        .map(PathBuf::from)
        .and_then(|path| path.file_name().map(ToOwned::to_owned))
        .unwrap_or_default();

    exec(build, exe_filename)?;

    Ok(())
}
