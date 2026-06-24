use std::convert::Infallible;
use std::ffi::c_char;
use std::io::{Read, Write};

include!(concat!(env!("OUT_DIR"), "/builds.rs"));

/// Stores a build and the CPU features it requires
#[cfg_attr(test, derive(Eq, Debug, Clone))]
pub struct Build<'a> {
    compressed: &'a [u8],

    /// A function pointer that, when called, returns true if the running CPU supports all build's features
    all_features_supported: fn() -> bool,

    #[cfg(any(test, feature = "debug"))]
    /// A comma-separated list of CPU features (e.g., `"avx, cmpxchg16b, fxsr, pclmulqdq, popcnt, sse, sse2, sse3, sse4.1, sse4.2, ssse3, xsave, xsaveopt"`)
    features: &'a str,

    /// The source of this build (`None` if it is not a patch, but a source and it only needs to be uncompressed)
    source: Option<&'a Self>,
}

#[cfg(test)]
impl PartialEq for Build<'_> {
    fn eq(&self, other: &Self) -> bool {
        #[inline]
        fn make_eq_key(build: &Build<'_>) -> impl Eq {
            (
                build.compressed,
                #[cfg(any(test, feature = "debug"))]
                build.features,
                build.source,
            )
        }

        make_eq_key(self) == make_eq_key(other)
    }
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
            let mut decoder = lz4_flex::frame::FrameDecoder::new(source.compressed);

            let mut source = Vec::with_capacity(source.compressed.len());
            decoder.read_to_end(&mut source)?;

            let mut patch = Vec::with_capacity(self.compressed.len());
            let mut decoder = lz4_flex::frame::FrameDecoder::new(self.compressed);
            decoder.read_to_end(&mut patch)?;

            let result = gdelta::decode(&patch, &source).map_err(|_| std::io::Error::other(""))?;
            output.write_all(&result)?;
        } else {
            let mut decoder = lz4_flex::frame::FrameDecoder::new(self.compressed);

            std::io::copy(&mut decoder, &mut output)?;
        }

        Ok(())
    }

    /// Finds a version that matches the CPU features of the host
    pub fn find_from(builds: impl IntoIterator<Item = Self>) -> Option<Self> {
        builds.into_iter().find(|build| {
            #[cfg(feature = "debug")]
            log::debug!("Checking build requiring CPU features: {}", build.features);

            (build.all_features_supported)()
        })
    }

    /// Finds a version that matches the CPU features of the host
    pub fn find() -> Option<Self> {
        Self::find_from(PATCHES)
    }

    /// List of CPU features required by the build
    #[cfg(feature = "debug")]
    pub fn features(&self) -> &str {
        self.features
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
        argv: *const *const c_char,
        envp: *const *const c_char,
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
    use std::io::Write;

    use crate::Build;

    #[test]
    fn find_none() {
        assert_eq!(Build::find_from(None), None);
    }

    #[test]
    fn find_no_features() {
        let build = Build {
            compressed: b"test",
            all_features_supported: || true,
            features: "",
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
            all_features_supported: || false,
            features: "unknown feature",
            source: None,
        };
        assert_eq!(Build::find_from(std::iter::once(build.clone())), None);
    }

    #[test]
    fn extract_into_fail_not_compressed() {
        let build = Build {
            compressed: b"invalid compressed data",
            all_features_supported: || true,
            features: "",
            source: None,
        };
        let mut v = vec![];
        build.extract_into(&mut v).unwrap_err();
    }

    #[test]
    fn extract_into() {
        let expected_data = b"data that will be compressed";
        let mut encoder = lz4_flex::frame::FrameEncoder::new(Vec::new());
        encoder.write_all(expected_data).unwrap();
        let compressed = encoder.finish().unwrap();

        let build = Build {
            compressed: &compressed,
            all_features_supported: || true,
            features: "",
            source: None,
        };
        let mut decompressed_data = vec![];
        build.extract_into(&mut decompressed_data).unwrap();
        assert_eq!(&decompressed_data, &expected_data);
    }
}
