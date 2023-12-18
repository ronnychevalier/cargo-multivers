use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use anyhow::Context;

use escargot::CargoBuild;

use crate::cargo::CommandMessagesExt;

pub struct RunnerBuilder {
    output_directory: PathBuf,
    manifest_path: PathBuf,
}

impl RunnerBuilder {
    /// Generates the sources of the crate to build the runner
    pub fn generate_crate_sources(
        output_directory: PathBuf,
        multivers_runner_version: &str,
    ) -> anyhow::Result<Self> {
        let root_directory = output_directory.join("package-runner");
        let src_directory = root_directory.join("src");
        let manifest_path = root_directory.join("Cargo.toml");
        let main_path = src_directory.join("main.rs");
        let local_multivers_runner_dependency =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("multivers-runner");

        let dependency = if local_multivers_runner_dependency.exists() {
            let local_multivers_runner_dependency = local_multivers_runner_dependency
                .to_string_lossy()
                .replace('\\', "/");
            format!(
                r#"multivers-runner = {{ version = "{multivers_runner_version}", path = "{local_multivers_runner_dependency}" }}"#,
            )
        } else {
            format!(r#"multivers-runner = "{multivers_runner_version}""#)
        };

        let manifest = format!(
            r#"
        [package]
        name = "package-multivers"
        version = "0.1.0"
        edition = "2021"
        
        [dependencies]
        {dependency}

        [profile.release]
        lto = true
        strip = "symbols"
        opt-level = "z"
        codegen-units = 1
        panic = "abort"

        [workspace]
        "#
        );
        let main = b"
        #![no_main]
        pub use multivers_runner::main;
        ";

        std::fs::create_dir_all(&src_directory)?;
        std::fs::write(&manifest_path, manifest)?;
        std::fs::write(main_path, main)?;

        Ok(Self {
            output_directory,
            manifest_path,
        })
    }

    /// Builds a runner that includes the given builds
    pub fn build(
        &self,
        target: &str,
        builds_path: &Path,
        original_filename: &OsStr,
    ) -> anyhow::Result<PathBuf> {
        let cargo = CargoBuild::new()
            .release()
            .target(target)
            .target_dir(&self.output_directory)
            .manifest_path(&self.manifest_path)
            .env("MULTIVERS_BUILDS_DESCRIPTION_PATH", builds_path);

        let cargo = cargo
            .exec()
            .context("Failed to execute cargo to build the runner")?;

        let bin_path = cargo
            .find_executable()?
            .ok_or_else(|| anyhow::anyhow!("Failed to build the runner"))?;

        let mut output_path = bin_path.clone();
        output_path.set_file_name(original_filename);

        std::fs::rename(&bin_path, &output_path)?;

        Ok(output_path)
    }
}
