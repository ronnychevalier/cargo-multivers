use std::collections::BTreeSet;
use std::io::Write;

use anyhow::Context;

use clap::Parser;

use crate::cli::{Cargo, Print};
use crate::multivers::{cpu_features, Multivers};
use crate::rustc::Rustc;

mod cli;
mod multivers;
mod runner;
mod rustc;

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
