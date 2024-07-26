use std::collections::HashMap;
use std::str::FromStr;

use cargo_metadata::Package;

use serde::{Deserialize, Deserializer};

use serde_json::Value;

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

/// The options set for a given target
#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub struct TargetMetadata {
    cpus: Option<Vec<String>>,
}

impl TargetMetadata {
    /// Returns a reference to set of CPUs explicitly enabled for this target.
    pub fn cpus(&self) -> Option<&[String]> {
        self.cpus.as_deref()
    }
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
#[derive(PartialEq, Eq, Debug)]
pub struct MultiversMetadata {
    targets: HashMap<Architecture, TargetMetadata>,
}

impl Default for MultiversMetadata {
    fn default() -> Self {
        let targets = HashMap::from([(
            Architecture::X86_64,
            TargetMetadata {
                cpus: Some(vec![
                    "x86-64".into(),
                    "x86-64-v2".into(),
                    "x86-64-v3".into(),
                    "x86-64-v4".into(),
                ]),
            },
        )]);

        Self { targets }
    }
}

impl MultiversMetadata {
    /// Parses the multivers metadata from a [`Package`] and use default values if not set
    pub fn from_package_with_default(package: &Package) -> anyhow::Result<Self> {
        Self::from_value_with_default(&package.metadata)
    }

    /// Parses the multivers metadata from a [`Value`] and use default values if not set
    pub fn from_value_with_default(value: &Value) -> anyhow::Result<Self> {
        let Some(from_value) = Self::from_value(value)? else {
            return Ok(Self::default());
        };

        let mut default = Self::default();
        default.update(&from_value);

        Ok(default)
    }

    /// Parses the multivers metadata from a [`Package`].
    pub fn from_package(package: &Package) -> anyhow::Result<Option<Self>> {
        Self::from_value(&package.metadata)
    }

    /// Interprets a [`Value`] as a [`MultiversMetadata`].
    pub fn from_value(value: &Value) -> anyhow::Result<Option<Self>> {
        if value.is_null() {
            return Ok(None);
        }

        let mut metadata: HashMap<String, serde_json::Value> =
            serde_json::from_value(value.clone())?;
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

    /// Returns a reference to the [`TargetMetadata`] associated to a given [`Architecture`].
    pub fn get(&self, k: &Architecture) -> Option<&TargetMetadata> {
        self.targets.get(k)
    }

    pub fn update(&mut self, other: &Self) {
        for (k, v) in other.targets.clone() {
            self.targets.insert(k, v);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use serde_json::{json, Value};

    use target_lexicon::Architecture;

    use crate::metadata::TargetMetadata;

    use super::MultiversMetadata;

    #[test]
    fn test_target_empty_cpus() {
        let value = json!({
            "multivers": {
                "x86_64": {
                    "cpus": []
                }
            }
        });

        let metadata = MultiversMetadata::from_value(&value).unwrap().unwrap();
        assert_eq!(
            metadata,
            MultiversMetadata {
                targets: HashMap::from([(
                    Architecture::X86_64,
                    TargetMetadata {
                        cpus: Some(Vec::new())
                    }
                ),])
            }
        );
    }

    #[test]
    fn test_target_cpus_not_set() {
        let value = json!({
            "multivers": {
                "x86_64": {
                }
            }
        });

        let metadata = MultiversMetadata::from_value(&value).unwrap().unwrap();
        assert_eq!(
            metadata,
            MultiversMetadata {
                targets: HashMap::from([(Architecture::X86_64, TargetMetadata { cpus: None }),])
            }
        );
    }

    #[test]
    fn test_target_cpus_set() {
        let value = json!({
            "multivers": {
                "x86_64": {
                    "cpus": ["alderlake", "skylake", "sandybridge", "ivybridge"]
                }
            }
        });

        let metadata = MultiversMetadata::from_value(&value).unwrap().unwrap();
        let target: &TargetMetadata = metadata.get(&Architecture::X86_64).unwrap();
        assert_eq!(
            target.cpus(),
            Some(
                &[
                    "alderlake".into(),
                    "skylake".into(),
                    "sandybridge".into(),
                    "ivybridge".into()
                ][..]
            )
        );
    }

    #[test]
    fn test_target_invalid() {
        let value = json!({
            "multivers": {
                "x86-64": {
                    "cpus": []
                }
            }
        });

        MultiversMetadata::from_value(&value).unwrap_err();
    }

    #[test]
    fn test_without_metadata() {
        let value = json!({
            "somethingelse": {
                "aaaa": {
                    "bbb": []
                }
            }
        });

        assert_eq!(MultiversMetadata::from_value(&value).unwrap(), None);
        assert_eq!(MultiversMetadata::from_value(&Value::Null).unwrap(), None);
    }

    #[test]
    fn test_metadata_null() {
        assert_eq!(MultiversMetadata::from_value(&Value::Null).unwrap(), None);
    }

    #[test]
    fn test_empty_targets() {
        let value = json!({
            "multivers": {}
        });

        assert_eq!(MultiversMetadata::from_value(&value).unwrap(), None);
    }

    #[test]
    fn test_target_default_update() {
        let mut default: MultiversMetadata = MultiversMetadata::default();
        let target: &TargetMetadata = default.get(&Architecture::X86_64).unwrap();
        assert_eq!(
            target.cpus(),
            Some(
                &[
                    "x86-64".into(),
                    "x86-64-v2".into(),
                    "x86-64-v3".into(),
                    "x86-64-v4".into(),
                ][..]
            )
        );

        let value = json!({
            "multivers": {
                "x86_64": {
                    "cpus": ["alderlake", "skylake", "sandybridge", "ivybridge"]
                }
            }
        });
        let metadata = MultiversMetadata::from_value(&value).unwrap().unwrap();
        default.update(&metadata);
        assert_eq!(
            default,
            MultiversMetadata {
                targets: HashMap::from([(
                    Architecture::X86_64,
                    TargetMetadata {
                        cpus: Some(vec![
                            "alderlake".into(),
                            "skylake".into(),
                            "sandybridge".into(),
                            "ivybridge".into()
                        ])
                    }
                ),])
            }
        );

        let value = json!({
            "multivers": {
                "powerpc": {
                }
            }
        });
        let metadata = MultiversMetadata::from_value(&value).unwrap().unwrap();
        default.update(&metadata);
        assert_eq!(
            default,
            MultiversMetadata {
                targets: HashMap::from([
                    (
                        Architecture::X86_64,
                        TargetMetadata {
                            cpus: Some(vec![
                                "alderlake".into(),
                                "skylake".into(),
                                "sandybridge".into(),
                                "ivybridge".into()
                            ])
                        }
                    ),
                    (Architecture::Powerpc, TargetMetadata { cpus: None })
                ])
            }
        );

        let value = json!({
            "multivers": {
                "x86_64": {
                }
            }
        });
        let metadata = MultiversMetadata::from_value(&value).unwrap().unwrap();
        default.update(&metadata);
        assert_eq!(
            default,
            MultiversMetadata {
                targets: HashMap::from([
                    (Architecture::X86_64, TargetMetadata { cpus: None }),
                    (Architecture::Powerpc, TargetMetadata { cpus: None })
                ])
            }
        );
    }

    #[test]
    fn test_from_value_with_default() {
        let value = json!({
            "multivers": {
                "x86_64": {
                    "cpus": ["alderlake", "skylake", "sandybridge", "ivybridge"]
                }
            }
        });
        let metadata = MultiversMetadata::from_value_with_default(&value).unwrap();
        assert_eq!(
            metadata,
            MultiversMetadata {
                targets: HashMap::from([(
                    Architecture::X86_64,
                    TargetMetadata {
                        cpus: Some(vec![
                            "alderlake".into(),
                            "skylake".into(),
                            "sandybridge".into(),
                            "ivybridge".into()
                        ])
                    }
                ),])
            }
        );

        let value = json!({
            "multivers": {
                "powerpc": {
                }
            }
        });
        let metadata = MultiversMetadata::from_value_with_default(&value).unwrap();
        assert_eq!(
            metadata,
            MultiversMetadata {
                targets: HashMap::from([
                    (
                        Architecture::X86_64,
                        TargetMetadata {
                            cpus: Some(vec![
                                "x86-64".into(),
                                "x86-64-v2".into(),
                                "x86-64-v3".into(),
                                "x86-64-v4".into(),
                            ])
                        }
                    ),
                    (Architecture::Powerpc, TargetMetadata { cpus: None })
                ])
            }
        );

        let value = json!({
            "multivers": {
                "x86_64": {
                }
            }
        });
        let metadata = MultiversMetadata::from_value_with_default(&value).unwrap();
        assert_eq!(
            metadata,
            MultiversMetadata {
                targets: HashMap::from([(Architecture::X86_64, TargetMetadata { cpus: None }),])
            }
        );
    }
}
