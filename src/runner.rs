use std::path::PathBuf;

use anyhow::Context;

use escargot::CargoBuild;

const RUNNER_CARGO_TOML: &[u8] = include_bytes!("../multivers-runner/Cargo.toml");
const RUNNER_CARGO_LOCK: &[u8] = include_bytes!("../multivers-runner/Cargo.lock");
const RUNNER_MAIN: &[u8] = include_bytes!("../multivers-runner/src/main.rs");
const RUNNER_BUILD: &[u8] = include_bytes!("build.rs");

pub struct RunnerBuilder {
    output_directory: PathBuf,
    manifest_path: PathBuf,
    rebuild_std: bool,
}

impl RunnerBuilder {
    /// Generates the sources of the crate to build the runner
    pub fn generate_crate_sources(output_directory: PathBuf) -> anyhow::Result<Self> {
        let root_directory = output_directory.join("multivers-runner");
        let src_directory = root_directory.join("src");
        let manifest_path = root_directory.join("Cargo.toml");
        let lock_path = root_directory.join("Cargo.lock");

        std::fs::create_dir_all(&src_directory)?;
        std::fs::write(src_directory.join("main.rs"), RUNNER_MAIN)?;
        std::fs::write(src_directory.join("build.rs"), RUNNER_BUILD)?;
        std::fs::write(&manifest_path, RUNNER_CARGO_TOML)?;
        std::fs::write(lock_path, RUNNER_CARGO_LOCK)?;

        Ok(Self {
            output_directory,
            manifest_path,
            rebuild_std: false,
        })
    }

    /// Rebuilds the std for the runner
    pub const fn rebuild_std(mut self, yes: bool) -> Self {
        self.rebuild_std = yes;
        self
    }

    /// Builds a runner that includes the given builds
    pub fn build(&self, target: &str, builds_path: PathBuf) -> anyhow::Result<PathBuf> {
        let cargo = CargoBuild::new()
            .release()
            .target(target)
            .target_dir(&self.output_directory)
            .manifest_path(&self.manifest_path)
            .env("CARGO_MULTIVERS_BUILDS_PATH", builds_path);

        let cargo = if self.rebuild_std {
            cargo.args(["-Zbuild-std=std"])
            // TODO: -Zbuild-std-features
        } else {
            cargo
        };

        let cargo = cargo
            .exec()
            .context("Failed to execute cargo to build the runner")?;

        let bin_path = cargo
            .into_iter()
            .find_map(|message| {
                let message = match message {
                    Ok(message) => message,
                    Err(e) => {
                        eprintln!("{e}");
                        return None;
                    }
                };
                match message.decode() {
                    Ok(escargot::format::Message::CompilerArtifact(artifact)) => {
                        if !artifact.profile.test
                            && artifact.target.crate_types == ["bin"]
                            && artifact.target.kind == ["bin"]
                        {
                            Some(artifact.filenames.get(0)?.to_path_buf())
                        } else {
                            None
                        }
                    }
                    Err(e) => {
                        eprintln!("{e}");
                        None
                    }
                    _ => {
                        // Ignored
                        None
                    }
                }
            })
            .ok_or_else(|| anyhow::anyhow!("Failed to build the runner"))?;

        Ok(bin_path)
    }
}
