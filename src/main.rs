use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::io::Write;
use std::path::PathBuf;
use std::str::FromStr;

use anyhow::Context;

use cargo_metadata::{Metadata, Package};

use clap::Parser;

use escargot::CargoBuild;

use serde::Serialize;

use target_lexicon::{Architecture, Triple};

use indicatif::ProgressBar;
use indicatif::ProgressStyle;

use console::style;
use console::Term;

use rayon::prelude::IntoParallelRefIterator;
use rayon::prelude::ParallelIterator;

use sha3::Digest;
use sha3::Sha3_256;

mod cli;
mod runner;
mod rustc;

use crate::cli::{Args, Cargo, Print};
use crate::runner::RunnerBuilder;
use crate::rustc::Rustc;

fn is_cpu_for_target_valid(triple: &Triple, cpu: &str) -> bool {
    // We need to ignore some CPUs, otherwise we get errors like `LLVM ERROR: 64-bit code requested on a subtarget that doesn't support it!`
    // See https://github.com/rust-lang/rust/issues/81148
    if triple.architecture == Architecture::X86_64
        && [
            "athlon",
            "athlon-4",
            "athlon-xp",
            "athlon-mp",
            "athlon-tbird",
            "c3",
            "c3-2",
            "geode",
            "i386",
            "i486",
            "i586",
            "i686",
            "k6",
            "k6-2",
            "k6-3",
            "lakemont",
            "pentium",
            "pentium-m",
            "pentium-mmx",
            "pentium2",
            "pentium3",
            "pentium3m",
            "pentium4",
            "pentium4m",
            "pentiumpro",
            "pentiumprescott",
            "prescott",
            "winchip-c6",
            "winchip2",
            "yonah",
        ]
        .contains(&cpu)
    {
        return false;
    }

    true
}

fn cpu_features(args: &Args, target: &str) -> anyhow::Result<BTreeMap<String, Vec<String>>> {
    let triple = Triple::from_str(target).context("Failed to parse the target")?;
    Rustc::cpus_from_target(target)
        .context("Failed to get the set of CPUs for the target")?
        .par_iter()
        .filter(|cpu| is_cpu_for_target_valid(&triple, cpu))
        .filter_map(|cpu| Rustc::features_from_cpu(target, cpu).ok())
        .filter_map(|mut features| {
            for exclude in args.exclude_cpu_features.iter().flatten() {
                features.remove(exclude);
            }
            if features.is_empty() {
                return None;
            }

            Some(features)
        })
        .map(|features| {
            let features_flags = features
                .iter()
                .fold(String::new(), |mut features, feature| {
                    features.push('+');
                    features.push_str(feature);
                    features.push(',');
                    features
                });
            let features_flags = features_flags.trim_end_matches(',');
            Ok((features_flags.to_owned(), features.into_iter().collect()))
        })
        .collect::<anyhow::Result<BTreeMap<String, Vec<String>>>>()
}

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

struct Multivers {
    metadata: Metadata,
    target: String,
    runner: RunnerBuilder,
    workspace: clap_cargo::Workspace,
    output_directory: PathBuf,
    features: clap_cargo::Features,
    cpu_features: BTreeMap<String, Vec<String>>,
    progress: ProgressBar,
    cargo_args: Vec<String>,
}

impl Multivers {
    fn from_args(args: Args) -> anyhow::Result<Self> {
        let metadata = args
            .manifest
            .metadata()
            .exec()
            .context("Failed to execute `cargo metadata`")?;

        // We must set a target, otherwise RUSTFLAGS will be used by the build scripts as well
        // (which we don't want since we might build with features not supported by the CPU building the package).
        // See https://github.com/rust-lang/cargo/issues/4423
        let target = args.target()?;

        let cpu_features = cpu_features(&args, &target)
            .context("Failed to get the set of CPU features for the target")?;

        let output_directory = metadata
            .target_directory
            .join(clap::crate_name!())
            .into_std_path_buf();

        let runner = RunnerBuilder::generate_crate_sources(output_directory.clone())
            .context("Failed to generate the source files of the runner")?;

        Ok(Self {
            metadata,
            target,
            runner,
            workspace: args.workspace,
            output_directory,
            features: args.features,
            cpu_features,
            progress: indicatif::ProgressBar::new(0).with_style(
                ProgressStyle::with_template(if Term::stdout().size().1 > 80 {
                    "{prefix:>12.cyan.bold} [{bar:57}] {pos}/{len} (time remaining {eta}) {wide_msg}"
                } else {
                    "{prefix:>12.cyan.bold} [{bar:57}] {pos}/{len}"
                })?
                .progress_chars("=> "),
            ),
            cargo_args: args.args
        })
    }

    pub fn build_package(&self, package: &Package) -> anyhow::Result<BuildsDescription> {
        let manifest_path = package.manifest_path.as_std_path().to_path_buf();
        let features_list = self.features.features.join(" ");
        let rust_flags = std::env::var("RUST_FLAGS").unwrap_or_default();

        self.progress.set_length(self.cpu_features.len() as u64);
        self.progress.set_prefix("Building");

        let mut hasher = Sha3_256::new();
        let mut builds = self
            .cpu_features
            .iter()
            .filter_map(move |(target_features_flags, cpu_features)| {
                self.progress.println(format!(
                    "{:>12} {target_features_flags}",
                    style("Compiling").bold().green()
                ));

                let rust_flags = format!("{rust_flags} -Ctarget-feature={target_features_flags}");
                let cargo = CargoBuild::new()
                    .release()
                    .target(&self.target)
                    .target_dir(&self.output_directory)
                    .manifest_path(&manifest_path)
                    .args(&self.cargo_args)
                    .env("RUSTFLAGS", rust_flags);

                let cargo = if self.features.all_features {
                    cargo.all_features()
                } else if self.features.no_default_features {
                    cargo.no_default_features()
                } else {
                    cargo.features(&features_list)
                };

                let cargo = match cargo.exec() {
                    Ok(cargo) => cargo,
                    Err(e) => {
                        eprintln!("Failed to build with features `{target_features_flags}`: {e}");
                        return None;
                    }
                };

                let bin_path = cargo.into_iter().find_map(|message| {
                    let message = match message {
                        Ok(message) => message,
                        Err(e) => {
                            eprintln!(
                                "Failed to build with features `{target_features_flags}`: {e}"
                            );
                            return None;
                        }
                    };
                    match message.decode() {
                        Ok(escargot::format::Message::CompilerArtifact(artifact)) => {
                            if !artifact.profile.test
                                && artifact.target.crate_types == ["bin"]
                                && artifact.target.kind == ["bin"]
                            {
                                self.progress.inc(1);
                                Some(artifact.filenames.get(0)?.to_path_buf())
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
                        Err(e) => {
                            eprintln!(
                                "Failed to build with features `{target_features_flags}`: {e}"
                            );
                            None
                        }
                        _ => {
                            // Ignored
                            None
                        }
                    }
                })?;

                hasher.update(target_features_flags.as_bytes());
                let filename = format!("{:x}", hasher.finalize_reset());

                let mut output_path = self
                    .output_directory
                    .join(&self.target)
                    .join("release")
                    .join(filename);
                output_path.set_extension(std::env::consts::EXE_EXTENSION);

                if let Err(e) = std::fs::rename(&bin_path, &output_path) {
                    return Some(Err(e.into()));
                }

                let hash = std::fs::read(&output_path).ok().map(|bytes| {
                    hasher.update(&bytes);
                    hasher.finalize_reset().to_vec()
                });

                let build = BuildDescription {
                    path: output_path,
                    features: cpu_features.clone(),
                    hash,
                    original_filename: bin_path.file_name().map(ToOwned::to_owned),
                };

                Some(Ok(build))
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
                .unwrap_or(format!("multivers-runner{}", std::env::consts::EXE_SUFFIX).into());

            let encoded =
                rmp_serde::to_vec_named(&builds).context("Failed to encode the builds")?;

            let package_output_directory = self.output_directory.join(&selected_package.name);
            std::fs::create_dir_all(&package_output_directory)
                .context("Failed to create temporary output directory")?;
            let builds_path = package_output_directory.join("builds.msgpack");
            std::fs::write(&builds_path, encoded)
                .with_context(|| format!("Failed to write to `{}`", builds_path.display()))?;

            println!("{:>12} runner", style("Compiling").bold().green());

            let bin_path = self.runner.build(
                &self.cargo_args,
                &self.target,
                &builds_path,
                &original_filename,
            )?;

            println!(
                "{:>12} ({})",
                style("Finished").bold().green(),
                bin_path.display()
            );
        }

        Ok(())
    }
}

fn main() -> anyhow::Result<()> {
    let Cargo::Multivers(args) = Cargo::parse();

    if matches!(args.print, Some(Print::CpuFeatures)) {
        let target = args.target()?;

        let cpu_features = cpu_features(&args, &target)
            .context("Failed to get the set of CPU features for the target")?;

        let mut stdout = std::io::stdout().lock();
        for feature in cpu_features
            .into_iter()
            .flat_map(|(_, features)| features)
            .collect::<BTreeSet<_>>()
        {
            let _ = writeln!(stdout, "{feature}");
        }

        return Ok(());
    }

    anyhow::ensure!(
        Rustc::is_nightly(),
        "You must run cargo multivers with Rust nightly channel. For example, you can run: `cargo +nightly multivers`"
    );

    let multivers = Multivers::from_args(args)?;
    multivers.build()?;

    Ok(())
}
