use std::io::stdout;
use std::io::BufRead;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::Context;

use bincode::config;

use cargo_metadata::{Metadata, Package};

use flate2::write::DeflateEncoder;
use flate2::Compression;

use clap::Parser;

use escargot::CargoBuild;

use humansize::{SizeFormatter, DECIMAL};

mod build;

use build::Build;

const RUNNER_CARGO_TOML: &[u8] = include_bytes!("../multivers-runner/Cargo.toml");
const RUNNER_MAIN: &[u8] = include_bytes!("../multivers-runner/src/main.rs");
const RUNNER_BUILD: &[u8] = include_bytes!("build.rs");

include!(concat!(env!("OUT_DIR"), "/codegen.rs"));

#[derive(clap::Args)]
struct Args {
    /// Build for the target triple
    #[clap(long, value_name = "TRIPLE")]
    target: Option<String>,

    /// Rebuild the std for each feature set as well
    #[clap(long)]
    rebuild_std: bool,

    // /// Build only the specified binary
    // #[clap(short, long)]
    // bin: String,
    //
    #[command(flatten)]
    manifest: clap_cargo::Manifest,

    #[command(flatten)]
    workspace: clap_cargo::Workspace,

    #[command(flatten)]
    features: clap_cargo::Features,
}

impl Args {
    pub fn target(&self) -> anyhow::Result<String> {
        let rustc_v = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
            .args(["rustc", "--", "-vV"])
            .output()?;

        if let Some(target) = &self.target {
            Ok(target.to_owned())
        } else {
            rustc_v
                .stdout
                .lines()
                .into_iter()
                .filter_map(Result::ok)
                .find_map(|line| line.strip_prefix("host: ").map(ToOwned::to_owned))
                .ok_or_else(|| anyhow::anyhow!("Failed to detect default target"))
        }
    }
}

fn build_everything(
    target: &str,
    package: &Package,
    rebuild_std: bool,
    metadata: &Metadata,
    features: &clap_cargo::Features,
) -> anyhow::Result<Vec<Build>> {
    let output_directory = metadata.target_directory.join(clap::crate_name!());
    let manifest_path = package.manifest_path.as_std_path().to_path_buf();
    let features_list = features.features.join(" ");
    let rust_flags = std::env::var("RUST_FLAGS").unwrap_or_default();

    let mut builds = CPUS["X86"]
        .into_iter()
        .filter_map(move |(cpu, cpu_features)| {
            println!("    Building for x86-{cpu}");

            let rust_flags = format!("{rust_flags} -Ctarget-cpu={cpu}");
            let cargo = CargoBuild::new()
                .release()
                .target(target)
                .target_dir(&output_directory)
                .manifest_path(&manifest_path)
                .env("RUSTFLAGS", rust_flags);

            let cargo = if rebuild_std {
                cargo.args(["-Zbuild-std=std"])
                // TODO: -Zbuild-std-features
            } else {
                cargo
            };

            let cargo = if features.all_features {
                cargo.all_features()
            } else if features.no_default_features {
                cargo.no_default_features()
            } else {
                cargo.features(&features_list)
            };

            let cargo = match cargo.exec() {
                Ok(cargo) => cargo,
                Err(e) => {
                    eprintln!("{e}");
                    return None;
                }
            };

            let bin_path = cargo.into_iter().find_map(|message| {
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
            })?;

            let bytes = match std::fs::read(&bin_path)
                .with_context(|| format!("Failed to read the executable `{}`", bin_path.display()))
            {
                Ok(bytes) => bytes,
                Err(e) => return Some(Err(e)),
            };
            let build = Build::new(
                bytes,
                cpu_features
                    .into_iter()
                    .map(|&cpu_feature| cpu_feature.to_owned())
                    .collect(),
            );

            Some(Ok(build))
        })
        .into_iter()
        .collect::<anyhow::Result<Vec<_>>>()?;
    builds.sort_unstable_by(|build1, build2| {
        // Awful heuristic just to use in priority the build with the largest feature set supported
        build1
            .required_cpu_features()
            .len()
            .cmp(&build2.required_cpu_features().len())
            .reverse()
    });

    Ok(builds)
}

fn create_runner_crate(output_directory: &Path) -> anyhow::Result<PathBuf> {
    let root_directory = output_directory.join("multivers-runner");
    let src_directory = root_directory.join("src");
    let manifest_path = root_directory.join("Cargo.toml");

    std::fs::create_dir_all(&src_directory)?;
    std::fs::write(src_directory.join("main.rs"), RUNNER_MAIN)?;
    std::fs::write(src_directory.join("build.rs"), RUNNER_BUILD)?;
    std::fs::write(&manifest_path, RUNNER_CARGO_TOML)?;

    Ok(manifest_path)
}

#[derive(clap::Parser)]
#[command(name = "cargo", bin_name = "cargo")]
enum Cargo {
    #[command(name = "multivers", version, author, about, long_about)]
    Multivers(Args),
}

fn main() -> anyhow::Result<()> {
    let Cargo::Multivers(args) = Cargo::parse();

    // We must set a target, otherwise RUSTFLAGS will be used by the build scripts as well
    // (which we don't want since we might build with features not supported by the CPU building the package).
    // See https://github.com/rust-lang/cargo/issues/4423
    let target = args.target()?;
    let metadata = args
        .manifest
        .metadata()
        .exec()
        .context("Failed to execute `cargo metadata`")?;
    let (selected_packages, _) = args.workspace.partition_packages(&metadata);
    let output_directory = metadata.target_directory.join(clap::crate_name!());

    for selected_package in selected_packages {
        println!("Building package {}", selected_package.name);

        let builds = build_everything(
            &target,
            selected_package,
            args.rebuild_std,
            &metadata,
            &args.features,
        )?;

        print!("    Encoding and compressing builds");
        let _ = stdout().flush();

        // TODO: compress the builds independently
        // to avoid the need to decompress them all when loading
        // and stop decompressing as soon as one matches
        let config = config::standard();
        let mut encoder = DeflateEncoder::new(Vec::new(), Compression::best());
        bincode::encode_into_std_write(builds, &mut encoder, config)
            .context("Failed to encode the builds")?;
        let encoded = encoder.finish().context("Failed to compress the builds")?;

        println!(" [{}]", SizeFormatter::new(encoded.len(), DECIMAL));

        let package_output_directory = output_directory.join(&selected_package.name);
        std::fs::create_dir_all(&package_output_directory)
            .context("Failed to create temporary output directory")?;
        let builds_path = package_output_directory.join("builds.bin");
        std::fs::write(&builds_path, encoded)
            .with_context(|| format!("Failed to write to `{builds_path}`"))?;

        println!("    Building runner");

        let runner_manifest_path = create_runner_crate(output_directory.as_std_path())
            .context("Failed to generate the source files of the runner")?;

        let cargo = CargoBuild::new()
            .release()
            .target_dir(&output_directory)
            .manifest_path(&runner_manifest_path)
            .env("CARGO_MULTIVERS_BUILDS_PATH", builds_path);

        let cargo = if args.rebuild_std {
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
        println!("    Done [{}]", bin_path.display());
    }

    Ok(())
}
