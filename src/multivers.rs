use std::ffi::OsString;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;

use anyhow::Context;

use cargo_metadata::{Metadata, Package};

use clap::ColorChoice;

use escargot::CargoBuild;

use serde::Serialize;

use target_lexicon::{Environment, Triple};

use indicatif::{ProgressBar, ProgressStyle};

use console::{style, Term};

use sha3::{Digest, Sha3_256};

use crate::cargo::CommandMessagesExt;
use crate::cli::Args;
use crate::features::{CpuFeatures, Cpus, CpusBuilder};
use crate::metadata::MultiversMetadata;
use crate::runner::RunnerBuilder;

#[derive(Serialize)]
struct BuildDescription {
    path: PathBuf,

    features: Vec<String>,

    #[serde(skip)]
    hash: Option<Vec<u8>>,

    #[serde(skip)]
    original_filename: Option<OsString>,
}

#[derive(Serialize)]
struct BuildsDescription {
    builds: Vec<BuildDescription>,
}

/// Build multiple versions of the same binary, each with a different CPU features set, merged into a single portable optimized binary
pub struct Multivers {
    metadata: Metadata,
    target: String,
    runner: RunnerBuilder,
    workspace: clap_cargo::Workspace,
    target_dir: PathBuf,
    out_dir: Option<PathBuf>,
    features: clap_cargo::Features,
    cpus: CpusBuilder,
    progress: ProgressBar,
    profile: String,
    cargo_args: Vec<String>,
}

impl Multivers {
    pub fn from_args(args: Args) -> anyhow::Result<Self> {
        let metadata = args
            .manifest
            .metadata()
            .exec()
            .context("Failed to execute `cargo metadata`")?;

        // We must set a target, otherwise RUSTFLAGS will be used by the build scripts as well
        // (which we don't want since we might build with features not supported by the CPU building the package).
        // See https://github.com/rust-lang/cargo/issues/4423
        let target = args.target()?.into_owned();
        let cpus = Cpus::builder(target.clone())
            .context("Failed to get the set of CPU features for the target")?
            .exclude_features(args.exclude_cpu_features)
            .cpus(args.cpus);

        let target_dir = metadata
            .target_directory
            .join(clap::crate_name!())
            .into_std_path_buf();

        let runner =
            RunnerBuilder::generate_crate_sources(target_dir.clone(), &args.runner_version)
                .context("Failed to generate the source files of the runner")?;

        let progress = indicatif::ProgressBar::new(0).with_style(
            ProgressStyle::with_template(
                "{prefix:>12.cyan.bold} [{bar:57}] {pos}/{len} {spinner}",
            )?
            .progress_chars("=> "),
        );
        progress.enable_steady_tick(Duration::from_millis(200));

        if args.color == ColorChoice::Never {
            console::set_colors_enabled(false);
        } else if args.color == ColorChoice::Always {
            console::set_colors_enabled(true);
        }

        Ok(Self {
            metadata,
            target,
            runner,
            workspace: args.workspace,
            target_dir,
            out_dir: args.out_dir,
            features: args.features,
            cpus,
            progress,
            cargo_args: args.args,
            profile: args.profile,
        })
    }

    fn cpu_features(&self, metadata: &MultiversMetadata) -> anyhow::Result<Vec<CpuFeatures>> {
        let cpus = self.cpus.clone().metadata(metadata)?.build()?;

        Ok(cpus.features_sets().cloned().collect())
    }

    fn build_package(&self, package: &Package) -> anyhow::Result<BuildsDescription> {
        let triple: Triple =
            Triple::from_str(&self.target).context("Failed to parse the target")?;
        let manifest_path = package.manifest_path.as_std_path();
        let features_list = self.features.features.join(" ");
        let mut rust_flags = std::env::var("RUSTFLAGS").unwrap_or_default();

        let metadata = MultiversMetadata::from_package_with_default(package)
            .context("Failed to parse package's metadata")?;
        let cpu_features = self.cpu_features(&metadata)?;

        if cpu_features.is_empty() {
            anyhow::bail!("Empty set of CPU features");
        }

        self.progress.set_length(cpu_features.len() as u64);
        self.progress.set_prefix("Building");

        if triple.environment == Environment::Msvc {
            rust_flags.push_str(" -C link-args=/Brepro");
        };

        let profile_dir = if self.profile == "dev" {
            "debug"
        } else {
            &self.profile
        };

        let mut final_style_set = false;
        let mut hasher = Sha3_256::new();
        let mut builds = cpu_features
            .into_iter()
            .enumerate()
            .map(|(i, cpu_features)| {
                if !final_style_set && i > 0 {
                    self.progress.disable_steady_tick();
                    self.progress.set_style(
                        ProgressStyle::with_template(if Term::stdout().size().1 > 80 {
                            "{prefix:>12.cyan.bold} [{bar:57}] {pos}/{len} (time remaining {eta}) {wide_msg}"
                        } else {
                            "{prefix:>12.cyan.bold} [{bar:57}] {pos}/{len}"
                        })?
                        .progress_chars("=> "),
                    );
                    final_style_set = true;
                }
                let target_features_flags = cpu_features.to_compiler_flags();
                self.progress.println(format!(
                    "{:>12} {target_features_flags}",
                    style("Compiling").bold().green()
                ));

                let rust_flags = format!("{rust_flags} -Ctarget-feature={target_features_flags}");
                let cargo = CargoBuild::new()
                    .arg(format!("--profile={}", self.profile))
                    .target(&self.target)
                    .manifest_path(manifest_path)
                    .args(&self.cargo_args)
                    .env("RUSTFLAGS", rust_flags);

                let cargo = if self.features.all_features {
                    cargo.all_features()
                } else if self.features.no_default_features {
                    cargo.no_default_features()
                } else {
                    cargo.features(&features_list)
                };

                let cargo = cargo.exec()?;

                let bin_path = cargo
                    .find_executable()?
                    .ok_or_else(|| anyhow::anyhow!("Failed to find a binary"))?;

                self.progress.inc(1);

                hasher.update(target_features_flags.as_bytes());
                let filename = format!("{:x}", hasher.finalize_reset());

                let output_path_parent = self
                    .target_dir
                    .join(&self.target)
                    .join(profile_dir);
                let mut output_path = output_path_parent
                    .join(filename);
                output_path.set_extension(std::env::consts::EXE_EXTENSION);

                std::fs::create_dir_all(&output_path_parent)
                    .with_context(|| format!("Failed to create directory `{}`", output_path_parent.display()))?;
                std::fs::copy(&bin_path, &output_path)
                    .with_context(|| format!("Failed to copy `{}` to `{}`", bin_path.display(), output_path.display()))?;

                let hash = std::fs::read(&output_path).ok().map(|bytes| {
                    hasher.update(&bytes);
                    hasher.finalize_reset().to_vec()
                });

                let build = BuildDescription {
                    path: output_path,
                    features: cpu_features.into_vec(),
                    hash,
                    original_filename: bin_path.file_name().map(ToOwned::to_owned),
                };

                Ok(build)
            })
            .collect::<anyhow::Result<Vec<_>>>()?;
        builds.sort_unstable_by(|build1, build2| {
            // First, we sort based on the hash of each build.
            build1
                .hash
                .cmp(&build2.hash)
                // Then, based on the features.
                .then_with(|| build1.features.len().cmp(&build2.features.len()))
        });
        // So that we can remove the duplicated builds and we remove the ones requiring more features.
        builds.dedup_by(|a, b| match (&a.hash, &b.hash) {
            (Some(a), Some(b)) => a == b,
            _ => false,
        });
        // Finally, we sort them to put the builds requiring more features at the top.
        builds.sort_unstable_by(|build1, build2| {
            build1.features.len().cmp(&build2.features.len()).reverse()
        });

        self.progress.finish_and_clear();

        Ok(BuildsDescription { builds })
    }

    pub fn build(&self) -> anyhow::Result<()> {
        let (selected_packages, _) = self.workspace.partition_packages(&self.metadata);

        let has_bins = selected_packages
            .iter()
            .any(|&package| package.targets.iter().any(|target| target.is_bin()));
        if !has_bins {
            anyhow::bail!(
                "No binary package detected. Only binaries can be built using cargo multivers."
            );
        }

        let profile_dir = if self.profile == "dev" {
            "debug"
        } else {
            &self.profile
        };

        for selected_package in selected_packages {
            println!(
                "{:>12} {} v{} ({})",
                style("Compiling").bold().green(),
                selected_package.name,
                selected_package.version,
                self.metadata.workspace_root
            );

            let builds = self.build_package(selected_package)?;

            let original_filename = builds
                .builds
                .iter()
                .find_map(|build| build.original_filename.clone())
                .unwrap_or_else(|| {
                    format!("multivers-runner{}", std::env::consts::EXE_SUFFIX).into()
                });

            if let [build] = builds.builds.as_slice() {
                let output_path = self
                    .target_dir
                    .join(&self.target)
                    .join(profile_dir)
                    .join(&original_filename);

                std::fs::rename(&build.path, &output_path).with_context(|| {
                    format!(
                        "Failed to rename `{}` to `{}`",
                        build.path.display(),
                        output_path.display()
                    )
                })?;

                if let Some(out_dir) = self.out_dir.as_deref() {
                    std::fs::create_dir_all(out_dir).with_context(|| {
                        format!("Failed to create output directory `{}`", out_dir.display())
                    })?;
                    let to = out_dir.join(&original_filename);
                    std::fs::copy(&output_path, &to).with_context(|| {
                        format!(
                            "Failed to copy `{}` to `{}`",
                            output_path.display(),
                            to.display()
                        )
                    })?;
                }

                println!(
                    "{:>12} 1 version, no runner needed ({})",
                    style("Finished").bold().green(),
                    output_path.display()
                );
            } else {
                let encoded =
                    serde_json::to_vec_pretty(&builds).context("Failed to encode the builds")?;

                let package_output_directory = self.target_dir.join(&selected_package.name);
                std::fs::create_dir_all(&package_output_directory)
                    .context("Failed to create temporary output directory")?;
                let builds_path = package_output_directory.join("builds.json");
                std::fs::write(&builds_path, encoded)
                    .with_context(|| format!("Failed to write to `{}`", builds_path.display()))?;

                println!(
                    "{:>12} {} versions compressed into a runner",
                    style("Compiling").bold().green(),
                    builds.builds.len(),
                );

                let bin_path = self
                    .runner
                    .build(&self.target, &builds_path, &original_filename)?;

                if let Some(out_dir) = self.out_dir.as_deref() {
                    std::fs::create_dir_all(out_dir).with_context(|| {
                        format!("Failed to create output directory `{}`", out_dir.display())
                    })?;
                    let to = out_dir.join(&original_filename);
                    std::fs::copy(&bin_path, &to).with_context(|| {
                        format!(
                            "Failed to copy `{}` to `{}`",
                            bin_path.display(),
                            to.display()
                        )
                    })?;
                }

                println!(
                    "{:>12} ({})",
                    style("Finished").bold().green(),
                    bin_path.display()
                );
            }
        }

        Ok(())
    }
}
