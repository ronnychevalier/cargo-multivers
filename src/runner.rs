use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use anyhow::Context;

use escargot::CargoBuild;

const RUNNER_CARGO_TOML: &[u8] = include_bytes!("../multivers-runner/Cargo.toml.template");
const RUNNER_CARGO_LOCK: &[u8] = include_bytes!("../multivers-runner/Cargo.lock");
const RUNNER_BUILD_SCRIPT: &[u8] = include_bytes!("../multivers-runner/build.rs");
const RUNNER_MAIN: &[u8] = include_bytes!("../multivers-runner/src/main.rs");
const RUNNER_BUILD: &[u8] = include_bytes!("../multivers-runner/src/build.rs");
const RUNNER_BUILD_LINUX: &[u8] = include_bytes!("../multivers-runner/src/build/linux.rs");
const RUNNER_BUILD_GENERIC: &[u8] = include_bytes!("../multivers-runner/src/build/generic.rs");

pub struct RunnerBuilder {
    output_directory: PathBuf,
    manifest_path: PathBuf,
}

impl RunnerBuilder {
    /// Generates the sources of the crate to build the runner
    pub fn generate_crate_sources(output_directory: PathBuf) -> anyhow::Result<Self> {
        let root_directory = output_directory.join("multivers-runner");
        let src_directory = root_directory.join("src");
        let build_directory = src_directory.join("build");
        let manifest_path = root_directory.join("Cargo.toml");

        std::fs::create_dir_all(&build_directory)?;
        std::fs::write(src_directory.join("main.rs"), RUNNER_MAIN)?;
        std::fs::write(src_directory.join("build.rs"), RUNNER_BUILD)?;
        std::fs::write(build_directory.join("linux.rs"), RUNNER_BUILD_LINUX)?;
        std::fs::write(build_directory.join("generic.rs"), RUNNER_BUILD_GENERIC)?;
        std::fs::write(&manifest_path, RUNNER_CARGO_TOML)?;
        std::fs::write(root_directory.join("Cargo.lock"), RUNNER_CARGO_LOCK)?;
        std::fs::write(root_directory.join("build.rs"), RUNNER_BUILD_SCRIPT)?;

        Ok(Self {
            output_directory,
            manifest_path,
        })
    }

    /// Builds a runner that includes the given builds
    pub fn build(
        &self,
        cargo_args: impl IntoIterator<Item = impl AsRef<OsStr>>,
        target: &str,
        builds_path: &Path,
        original_filename: &OsStr,
    ) -> anyhow::Result<PathBuf> {
        let cargo = CargoBuild::new()
            .release()
            .target(target)
            .target_dir(&self.output_directory)
            .manifest_path(&self.manifest_path)
            .args(cargo_args)
            .env("MULTIVERS_BUILDS_DESCRIPTION_PATH", builds_path);

        let cargo = cargo
            .exec()
            .context("Failed to execute cargo to build the runner")?;

        let bin_path = cargo
            .into_iter()
            .find_map(|message| {
                let message = match message {
                    Ok(message) => message,
                    Err(e) => return Some(Err(e)),
                };
                match message.decode() {
                    Ok(escargot::format::Message::CompilerArtifact(artifact)) => {
                        if !artifact.profile.test
                            && artifact.target.crate_types == ["bin"]
                            && artifact.target.kind == ["bin"]
                        {
                            Some(Ok(artifact.filenames.get(0)?.to_path_buf()))
                        } else {
                            None
                        }
                    }
                    Ok(escargot::format::Message::CompilerMessage(e)) => {
                        if let Some(rendered) = e.message.rendered {
                            eprint!("{rendered}");
                        }

                        None
                    }
                    Ok(_) => {
                        // Ignored
                        None
                    }
                    Err(e) => Some(Err(e)),
                }
            })
            .ok_or_else(|| anyhow::anyhow!("Failed to build the runner"))??;

        let mut output_path = bin_path.clone();
        output_path.set_file_name(original_filename);

        std::fs::rename(&bin_path, &output_path)?;

        Ok(output_path)
    }
}
