use std::collections::BTreeSet;
use std::io::BufRead;
use std::path::PathBuf;
use std::process::Command;
use std::sync::LazyLock;

// We do not call "cargo rustc" (which would be simpler),
// because it takes too much time to execute each time.
// Calling rustc directly is faster.
static RUSTC: LazyLock<PathBuf> = LazyLock::new(|| {
    std::env::var_os("CARGO")
        .map(PathBuf::from)
        .and_then(|path| {
            path.parent().map(|bin| {
                bin.join("rustc")
                    .with_extension(std::env::consts::EXE_EXTENSION)
            })
        })
        .unwrap_or_else(|| "rustc".into())
});

/// Wrapper around the `rustc` command
pub struct Rustc;

impl Rustc {
    fn command() -> Command {
        Command::new(RUSTC.as_path())
    }

    /// Returns true if rustc is on the nightly release channel
    pub fn is_nightly() -> bool {
        let Ok(rustc_v) = Self::command().arg("-vV").output() else {
            return false;
        };

        let release = rustc_v
            .stdout
            .lines()
            .map_while(Result::ok)
            .find_map(|line| line.strip_prefix("release: ").map(ToOwned::to_owned))
            .unwrap_or_default();

        release.contains("nightly")
    }

    /// Returns the default target that rustc uses to build if none is provided (the host)
    pub fn default_target() -> anyhow::Result<String> {
        let rustc_v = Self::command().arg("-vV").output()?;

        rustc_v
            .stdout
            .lines()
            .map_while(Result::ok)
            .find_map(|line| line.strip_prefix("host: ").map(ToOwned::to_owned))
            .ok_or_else(|| anyhow::anyhow!("Failed to detect default target"))
    }

    /// Returns all CPU features supported by a given CPU on a target
    pub fn features_from_cpu(target: &str, cpu: &str) -> anyhow::Result<BTreeSet<String>> {
        let cfg = Self::command()
            .args([
                "--print=cfg",
                "--target",
                target,
                &format!("-Ctarget-cpu={cpu}"),
            ])
            .output()?;

        anyhow::ensure!(
            cfg.status.success() && cfg.stderr.is_empty(),
            "Invalid CPU `{cpu}`"
        );

        let features = cfg
            .stdout
            .lines()
            .map_while(Result::ok)
            .filter_map(|line| {
                let line = line.strip_prefix("target_feature=\"")?;
                // Ignores lines such as llvm14-builtins-abi
                if line.starts_with("llvm") {
                    return None;
                }

                line.strip_suffix('"').map(ToOwned::to_owned)
            })
            .collect();

        Ok(features)
    }

    /// Returns all the known CPUs of a given target
    pub fn cpus_from_target(target: &str) -> anyhow::Result<Vec<String>> {
        let cpus = Self::command()
            .args(["--print=target-cpus", "--target", target])
            .output()?;

        anyhow::ensure!(cpus.status.success(), "Invalid target `{target}`");

        let cpus = cpus
            .stdout
            .lines()
            .skip(1)
            .filter_map(Result::ok)
            .filter_map(|line| {
                let line = if let Some((line, _)) = line.split_once("- This is the default target")
                {
                    line
                } else {
                    &line
                };
                let line = line.trim();
                if line.starts_with("native") || line.is_empty() {
                    return None;
                }

                Some(line.to_owned())
            })
            .collect();

        Ok(cpus)
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use target_lexicon::Triple;

    use super::Rustc;

    #[test]
    fn test_finds_rustc_without_env_cargo() {
        unsafe { std::env::remove_var("CARGO") };
        let target = Rustc::default_target().unwrap();
        Triple::from_str(&target).unwrap();
    }

    #[test]
    fn test_default_target_valid() {
        let target = Rustc::default_target().unwrap();
        Triple::from_str(&target).unwrap();
    }

    #[test]
    fn test_cpus_from_target_not_empty() {
        let target = Rustc::default_target().unwrap();
        let cpus = Rustc::cpus_from_target(&target).unwrap();
        assert!(!cpus.is_empty());
    }

    #[test]
    fn test_cpus_from_target_invalid() {
        Rustc::cpus_from_target("invalid target").unwrap_err();
    }

    #[test]
    fn test_features_from_cpu_not_empty() {
        let target = Rustc::default_target().unwrap();
        let cpus = Rustc::cpus_from_target(&target).unwrap();
        let features = Rustc::features_from_cpu(&target, cpus.first().unwrap()).unwrap();
        assert!(!features.is_empty());
    }

    #[test]
    fn test_features_from_cpu_invalid() {
        let target = Rustc::default_target().unwrap();
        Rustc::features_from_cpu(&target, "invalid invalid").unwrap_err();
    }
}
