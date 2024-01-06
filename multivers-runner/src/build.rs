use std::convert::Infallible;
use std::io::{Read, Write};

use bzip2::read::BzDecoder;

use qbsdiff::Bspatch;

include!(concat!(env!("OUT_DIR"), "/builds.rs"));

/// Stores a build and the CPU features it requires
#[cfg_attr(test, derive(PartialEq, Eq, Debug, Clone))]
pub struct Build<'a> {
    compressed: &'a [u8],

    /// A list of CPU features (e.g., `["avx" , "cmpxchg16b" , "fxsr" , "pclmulqdq" , "popcnt" , "sse" , "sse2" , "sse3" , "sse4.1" , "sse4.2" , "ssse3" , "xsave" , "xsaveopt"]`)
    features: &'a [&'a str],

    /// The source of this build (`None` if it is not a patch, but a source and it only needs to be uncompressed)
    source: Option<&'a Self>,
}

impl Default for Build<'_> {
    fn default() -> Self {
        SOURCE
    }
}

impl Build<'_> {
    /// Extracts the build into a writer
    pub fn extract_into(&self, mut output: impl Write) -> std::io::Result<()> {
        if let Some(source) = self.source {
            let mut decoder = BzDecoder::new(source.compressed);
            let patcher = Bspatch::new(self.compressed)?;

            let mut source = Vec::with_capacity(source.compressed.len());
            decoder.read_to_end(&mut source)?;

            patcher.apply(&source, output)?;
        } else {
            let mut decoder = BzDecoder::new(self.compressed);

            std::io::copy(&mut decoder, &mut output)?;
        }

        Ok(())
    }

    /// Finds a version that matches the CPU features of the host
    pub fn find_from(builds: impl IntoIterator<Item = Self>) -> Option<Self> {
        let supported_features: Vec<&str> = notstd_detect::detect::features()
            .filter_map(|(feature, supported)| supported.then_some(feature))
            .collect();

        builds.into_iter().find_map(|build| {
            build
                .features
                .iter()
                .all(|feature| supported_features.contains(feature))
                .then_some(build)
        })
    }

    /// Finds a version that matches the CPU features of the host
    pub fn find() -> Option<Self> {
        Self::find_from(PATCHES)
    }
}

/// A type that can be executed like a standard program.
pub trait Executable {
    /// Executes the program.
    ///
    /// The arguments (`argc`, `argv`, and `envp`) can be used by the implementation
    /// for optimization purposes, but they may be ignored (and fetched with [`std::env::args_os()`]).
    ///
    /// # Safety
    ///
    /// - `argc` must never be negative.
    /// - `argv` and `envp` must be null-terminated arrays of valid pointers to null-terminated strings.
    /// - Each element of `argv` and `envp` must be valid for reads of bytes up to and including the null terminator.
    unsafe fn exec(
        self,
        argc: i32,
        argv: *const *const i8,
        envp: *const *const i8,
    ) -> Result<Infallible, proc_exit::Exit>;
}

cfg_if::cfg_if! {
    if #[cfg(target_os = "linux")] {
        mod linux;
    } else {
        mod generic;
    }
}

#[cfg(test)]
mod tests {
    use std::io::Read;

    use bzip2::read::BzEncoder;
    use bzip2::Compression;

    use crate::Build;

    #[test]
    fn find_none() {
        assert_eq!(Build::find_from(None), None);
    }

    #[cfg(target_feature = "sse")]
    #[test]
    fn find_x86_sse() {
        let build = Build {
            compressed: b"test",
            features: &["sse"],
            source: None,
        };
        assert_eq!(
            Build::find_from(std::iter::once(build.clone())),
            Some(build)
        );
    }

    #[test]
    fn find_no_features() {
        let build = Build {
            compressed: b"test",
            features: &[],
            source: None,
        };
        assert_eq!(
            Build::find_from(std::iter::once(build.clone())),
            Some(build)
        );
    }

    #[test]
    fn find_feature_not_found() {
        let build = Build {
            compressed: b"test",
            features: &["unknown feature"],
            source: None,
        };
        assert_eq!(Build::find_from(std::iter::once(build.clone())), None);
    }

    #[test]
    fn extract_into_fail_not_compressed() {
        let build = Build {
            compressed: b"test",
            features: &[],
            source: None,
        };
        let mut v = vec![];
        build.extract_into(&mut v).unwrap_err();
    }

    #[test]
    fn extract_into() {
        let expected_data = b"data that will be compressed";
        let mut encoder = BzEncoder::new(expected_data.as_slice(), Compression::best());
        let mut compressed = vec![];
        encoder.read_to_end(&mut compressed).unwrap();

        let build = Build {
            compressed: &compressed,
            features: &[],
            source: None,
        };
        let mut decompressed_data = vec![];
        build.extract_into(&mut decompressed_data).unwrap();
        assert_eq!(&decompressed_data, &expected_data);
    }
}
