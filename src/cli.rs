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
    /// Build for the target triple
    #[clap(long, value_name = "TRIPLE")]
    pub target: Option<String>,

    /// Print information on stdout
    #[clap(long, value_name = "INFORMATION")]
    pub print: Option<Print>,

    /// Comma-separated list of CPUs to use as a target
    #[clap(
        long,
        use_value_delimiter = true,
        value_delimiter = ',',
        value_name = "CPUs"
    )]
    pub cpus: Option<Vec<String>>,

    /// Comma-separated list of CPU features to exclude from the builds
    #[clap(
        long,
        use_value_delimiter = true,
        value_delimiter = ',',
        value_name = "CPU-FEATURES"
    )]
    pub exclude_cpu_features: Option<Vec<String>>,

    /// Specify the version of the runner to use
    #[clap(long, value_name = "VERSION", default_value = "0.1")]
    pub runner_version: String,

    /// Build artifacts with the specified profile
    #[clap(long, value_name = "PROFILE-NAME", default_value = "release")]
    pub profile: String,

    /// Color preferences for program output
    #[clap(long, value_name = "WHEN", default_value = "auto")]
    pub color: ColorChoice,

    /// Copy final artifacts to this directory
    #[clap(long, value_name = "PATH")]
    pub out_dir: Option<PathBuf>,

    #[command(flatten)]
    pub manifest: clap_cargo::Manifest,

    #[command(flatten)]
    pub workspace: clap_cargo::Workspace,

    #[command(flatten)]
    pub features: clap_cargo::Features,

    /// Arguments given to cargo build
    #[clap(raw = true)]
    pub args: Vec<String>,
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
