use std::borrow::Cow;
use std::path::PathBuf;

use clap::ColorChoice;

use crate::rustc::Rustc;

#[derive(clap::Parser)]
#[command(name = "cargo", bin_name = "cargo")]
pub enum Cargo {
    #[command(name = "multivers", version, author, about, long_about)]
    Multivers(Args),
}

/// Type of information to print on stdout
#[derive(clap::ValueEnum, Clone, Copy)]
pub enum Print {
    /// Prints the list of CPU features supported by the target
    CpuFeatures,
}

#[derive(clap::Args)]
pub struct Args {
    /// Arguments given to cargo build
    #[clap(raw = true)]
    pub args: Vec<String>,

    /// Build for the target triple
    #[clap(long, value_name = "TRIPLE", help_heading = "Compilation Options")]
    pub target: Option<String>,

    /// Print information on stdout
    #[clap(long, value_name = "INFORMATION")]
    pub print: Option<Print>,

    /// Comma-separated list of CPUs to use as a target
    #[clap(
        long,
        use_value_delimiter = true,
        value_delimiter = ',',
        value_name = "CPUs",
        help_heading = "Compilation Options"
    )]
    pub cpus: Option<Vec<String>>,

    /// Comma-separated list of CPU features to exclude from the builds
    #[clap(
        long,
        use_value_delimiter = true,
        value_delimiter = ',',
        value_name = "CPU-FEATURES",
        help_heading = "Compilation Options"
    )]
    pub exclude_cpu_features: Option<Vec<String>>,

    /// Specify the version of the runner to use
    #[clap(
        long,
        value_name = "VERSION",
        default_value = "0.3",
        help_heading = "Runner Options"
    )]
    pub runner_version: String,

    /// Build artifacts with the specified profile
    #[clap(
        long,
        value_name = "PROFILE-NAME",
        default_value = "release",
        help_heading = "Compilation Options"
    )]
    pub profile: String,

    /// Color preferences for program output
    #[clap(long, value_name = "WHEN", default_value = "auto")]
    pub color: ColorChoice,

    /// Copy final artifacts to this directory
    #[clap(long, value_name = "PATH", help_heading = "Compilation Options")]
    pub out_dir: Option<PathBuf>,

    #[clap(long, value_delimiter = ' ', help_heading = "Runner Options")]
    /// Space-separated list of features to activate for the runner (only "debug" is available at the moment)
    pub runner_features: Vec<String>,

    #[clap(long, help_heading = "Runner Options")]
    /// Path to a custom runner Cargo.toml (by default, one is generated automatically)
    pub runner_manifest_path: Option<PathBuf>,

    #[command(flatten, next_help_heading = "Manifest Options")]
    pub manifest: clap_cargo::Manifest,

    #[command(flatten, next_help_heading = "Package Selection")]
    pub workspace: clap_cargo::Workspace,

    #[command(flatten, next_help_heading = "Feature Selection")]
    pub features: clap_cargo::Features,
}

impl Args {
    /// Returns the target given on the command line or the default target that rustc uses to build if none is provided
    pub fn target(&self) -> anyhow::Result<Cow<'_, str>> {
        self.target.as_deref().map_or_else(
            || Rustc::default_target().map(Cow::Owned),
            |target| Ok(target.into()),
        )
    }
}
