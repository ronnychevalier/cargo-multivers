//! Cargo subcommand to build multiple versions of the same binary, each with a different CPU features set, merged into a single portable optimized binary.
use std::io::Write;

use anyhow::Context;

use clap::Parser;

use crate::cli::{Cargo, Print};
use crate::features::Cpus;
use crate::multivers::Multivers;
use crate::rustc::Rustc;

mod cargo;
mod cli;
mod features;
mod metadata;
mod multivers;
mod runner;
mod rustc;

fn main() -> anyhow::Result<()> {
    let Cargo::Multivers(args) = Cargo::parse();

    if matches!(args.print, Some(Print::CpuFeatures)) {
        let target = args.target()?.into_owned();

        let cpus = Cpus::builder(target)
            .context("Failed to get the set of CPU features for the target")?
            .cpus(args.cpus)
            .build()?;
        let mut stdout = std::io::stdout().lock();
        for feature in cpus.features() {
            let _ = writeln!(stdout, "{feature}");
        }

        return Ok(());
    }

    anyhow::ensure!(
        Rustc::is_nightly(),
        "You must run cargo multivers with Rust nightly channel. For example, you can run: `cargo +nightly multivers`"
    );

    Multivers::from_args(args)?.build()
}
