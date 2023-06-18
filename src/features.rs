use std::collections::{BTreeMap, BTreeSet};
use std::ops::Deref;
use std::str::FromStr;

use anyhow::Context;

use target_lexicon::{Architecture, Triple};

use rayon::prelude::{IntoParallelIterator, ParallelIterator};

use itertools::Itertools;

use crate::rustc::Rustc;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CpuFeatures(BTreeSet<String>);

impl CpuFeatures {
    /// Builds a string of CPU feature flags that can be given to  `rustc -C target-feature=` (e.g., `+aes,+avx,+sse`)
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
    pub fn builder<'a>(
        target: impl Into<String>,
        cpus: Option<Vec<String>>,
    ) -> anyhow::Result<CpusBuilder<'a>> {
        CpusBuilder::new(target, cpus)
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

pub struct CpusBuilder<'a> {
    iter: rayon::vec::IntoIter<String>,
    excluded_features: Option<&'a [String]>,
    target: String,
    triple: Triple,
}

impl<'a> CpusBuilder<'a> {
    pub fn new(target: impl Into<String>, cpus: Option<Vec<String>>) -> anyhow::Result<Self> {
        let target = target.into();
        let triple = Triple::from_str(&target).context("Failed to parse the target")?;
        let iter = if let Some(cpus) = cpus {
            cpus
        } else {
            Rustc::cpus_from_target(&target)
                .context("Failed to get the set of CPUs for the target")?
        }
        .into_par_iter();

        Ok(Self {
            iter,
            excluded_features: None,
            target,
            triple,
        })
    }

    /// Excludes the given list of CPU features when building the set of CPUs and their features.
    ///
    /// If a CPU no longer has any feature, it is not included in the final list.
    pub fn exclude_features(mut self, cpu_features: impl Into<Option<&'a [String]>>) -> Self {
        self.excluded_features = cpu_features.into();
        self
    }

    pub fn build(self) -> Cpus {
        let features = self
            .iter
            .filter(|cpu| Self::is_cpu_for_target_valid(&self.triple, cpu))
            .filter_map(|cpu| {
                let features = Rustc::features_from_cpu(&self.target, &cpu).ok()?;
                Some((cpu, features))
            })
            .filter_map(|(cpu, mut features)| {
                for exclude in self.excluded_features.into_iter().flatten() {
                    features.remove(exclude);
                }
                if features.is_empty() {
                    return None;
                }

                Some((cpu, CpuFeatures(features)))
            })
            .collect();

        Cpus { features }
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
