use std::collections::{BTreeMap, BTreeSet};
use std::ops::Deref;
use std::str::FromStr;

use anyhow::Context;

use target_lexicon::{Architecture, Triple};

use rayon::prelude::{IntoParallelIterator, ParallelIterator};

use itertools::Itertools;

use crate::metadata::{MultiversMetadata, TargetMetadata};
use crate::rustc::Rustc;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CpuFeatures(BTreeSet<String>);

impl CpuFeatures {
    /// Builds a string of CPU feature flags that can be given to `rustc -C target-feature=` (e.g., `+aes,+avx,+sse`)
    pub fn to_compiler_flags(&self) -> String {
        let features_flags = self.0.iter().fold(String::new(), |mut features, feature| {
            features.push('+');
            features.push_str(feature);
            features.push(',');
            features
        });

        features_flags.trim_end_matches(',').into()
    }

    /// Consumes the [`CpuFeatures`] and returns a [`Vec`] containing the features
    pub fn into_vec(self) -> Vec<String> {
        self.0.into_iter().collect()
    }
}

pub struct Cpus {
    /// Maps a CPU (e.g., `alderlake`) to its list of CPU features (`adx`, `aes`,...)
    features: BTreeMap<String, CpuFeatures>,
}

impl Cpus {
    pub fn builder(target: impl Into<String>) -> anyhow::Result<CpusBuilder> {
        CpusBuilder::new(target)
    }

    /// Returns a sorted and deduplicated iterator of CPU features set for each CPU
    pub fn features_sets(&self) -> impl Iterator<Item = &CpuFeatures> {
        self.features.values().sorted().dedup()
    }

    /// Returns a sorted and deduplicated iterator of CPU features supported by these CPUs
    pub fn features(&self) -> impl Iterator<Item = &str> {
        self.features
            .values()
            .flat_map(|features| &features.0)
            .map(|s| s.deref())
            .sorted()
            .dedup()
    }

    pub fn get(&self, cpu: &str) -> Option<&CpuFeatures> {
        self.features.get(cpu)
    }
}

#[derive(Clone)]
pub struct CpusBuilder {
    excluded_features: Option<Vec<String>>,
    target: String,
    triple: Triple,
    cpus: Option<Vec<String>>,
    metadata_cpus: Option<Vec<String>>,
}

impl CpusBuilder {
    pub fn new(target: impl Into<String>) -> anyhow::Result<Self> {
        let target = target.into();
        let triple = Triple::from_str(&target).context("Failed to parse the target")?;

        Ok(Self {
            excluded_features: None,
            target,
            triple,
            cpus: None,
            metadata_cpus: None,
        })
    }

    pub fn cpus(mut self, cpus: impl Into<Option<Vec<String>>>) -> Self {
        self.cpus = cpus.into();
        self
    }

    pub fn metadata(mut self, metadata: &MultiversMetadata) -> anyhow::Result<Self> {
        if let Some(cpus) = metadata
            .get(&self.triple.architecture)
            .and_then(TargetMetadata::cpus)
        {
            anyhow::ensure!(!cpus.is_empty(), "Empty list of CPUs");

            self.metadata_cpus = Some(cpus.to_vec());
        }

        Ok(self)
    }

    /// Excludes the given list of CPU features when building the set of CPUs and their features.
    ///
    /// If a CPU no longer has any feature, it is not included in the final list.
    pub fn exclude_features(mut self, cpu_features: impl Into<Option<Vec<String>>>) -> Self {
        self.excluded_features = cpu_features.into();
        self
    }

    pub fn build(&self) -> anyhow::Result<Cpus> {
        let iter = if let Some(cpus) = self.cpus.as_ref().or(self.metadata_cpus.as_ref()) {
            cpus.to_owned()
        } else {
            Rustc::cpus_from_target(&self.target)
                .context("Failed to get the set of CPUs for the target")?
        }
        .into_par_iter();

        let features: BTreeMap<_, _> = iter
            .filter(|cpu| Self::is_cpu_for_target_valid(&self.triple, cpu))
            .map(|cpu| {
                let features = Rustc::features_from_cpu(&self.target, &cpu)?;

                Ok((cpu, features))
            })
            .collect::<anyhow::Result<_>>()?;

        let features = features
            .into_iter()
            .filter_map(|(cpu, mut features)| {
                for exclude in self.excluded_features.iter().flatten() {
                    features.remove(exclude);
                }
                if features.is_empty() {
                    return None;
                }

                Some((cpu, CpuFeatures(features)))
            })
            .collect();

        Ok(Cpus { features })
    }

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
}
