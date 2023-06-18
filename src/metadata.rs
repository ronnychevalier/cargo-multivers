use std::collections::HashMap;
use std::str::FromStr;

use cargo_metadata::Package;

use serde::{Deserialize, Deserializer};

use target_lexicon::Architecture;

#[derive(PartialEq, Eq, Hash, Debug)]
#[repr(transparent)]
struct ArchitectureWrapper(Architecture);

impl<'de> Deserialize<'de> for ArchitectureWrapper {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let str = String::deserialize(deserializer)?;
        let arch = Architecture::from_str(&str).map_err(|_| {
            serde::de::Error::invalid_value(
                serde::de::Unexpected::Other(&str),
                &"a CPU architecture",
            )
        })?;

        Ok(Self(arch))
    }
}

#[derive(PartialEq, Eq, Hash, Debug)]
pub struct TargetMetadata {
    pub cpus: Option<Vec<String>>,
}

impl<'de> Deserialize<'de> for TargetMetadata {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let mut metadata = HashMap::<String, Option<Vec<String>>>::deserialize(deserializer)?;
        let cpus = metadata.remove("cpus").unwrap_or_default();

        Ok(Self { cpus })
    }
}

/// Contents of the `[package.metadata.multivers]` section in a `Cargo.toml`.
///
/// # Example
///
/// ```toml
/// [package.metadata.multivers.x86_64]
/// cpus = ["alderlake", "skylake", "sandybridge", "ivybridge"]
/// ```
#[derive(Debug)]
pub struct MultiversMetadata {
    targets: HashMap<Architecture, TargetMetadata>,
}

impl MultiversMetadata {
    /// Parses the multivers metadata from a [`Package`].
    pub fn from_package(package: &Package) -> anyhow::Result<Option<Self>> {
        if package.metadata.is_null() {
            return Ok(None);
        }

        let mut metadata: HashMap<String, serde_json::Value> =
            serde_json::from_value(package.metadata.clone())?;
        let Some(metadata) = metadata.remove("multivers") else {
            return Ok(None);
        };

        let targets: HashMap<ArchitectureWrapper, _> = serde_json::from_value(metadata)?;
        if targets.is_empty() {
            return Ok(None);
        }

        Ok(Some(Self {
            targets: targets
                .into_iter()
                .map(|(key, value)| (key.0, value))
                .collect(),
        }))
    }

    pub fn get(&self, k: &Architecture) -> Option<&TargetMetadata> {
        self.targets.get(k)
    }
}
