use std::convert::Infallible;
use std::io::Write;

use bzip2::read::BzDecoder;

use qbsdiff::Bspatch;

include!(concat!(env!("OUT_DIR"), "/builds.rs"));

/// Stores a build and the CPU features it requires
pub struct Build<'a> {
    compressed_build: &'a [u8],

    /// A list of CPU features (e.g., `["avx" , "cmpxchg16b" , "fxsr" , "pclmulqdq" , "popcnt" , "sse" , "sse2" , "sse3" , "sse4.1" , "sse4.2" , "ssse3" , "xsave" , "xsaveopt"]`)
    features: &'a [&'a str],

    /// Whether the build is the source (i.e., it is not a patch, it only needs to be uncompressed)
    source: bool,
}

impl<'a> Build<'a> {
    /// Extracts the build into a writer
    pub fn extract_into(&self, mut output: impl Write) -> std::io::Result<()> {
        let mut decoder = BzDecoder::new(SOURCE.compressed_build);
        if self.source {
            std::io::copy(&mut decoder, &mut output)?;
        } else {
            let patcher = Bspatch::new(self.compressed_build)?;

            let mut source = Vec::new();
            std::io::copy(&mut decoder, &mut source)?;

            patcher.apply(&source, output)?;
        }

        Ok(())
    }

    /// Finds a version that matches the CPU features of the host
    pub fn find() -> Self {
        let supported_features: Vec<&str> = notstd_detect::detect::features()
            .filter_map(|(feature, supported)| supported.then_some(feature))
            .collect();

        PATCHES
            .into_iter()
            .find_map(|build| {
                build
                    .features
                    .iter()
                    .all(|feature| supported_features.contains(feature))
                    .then_some(build)
            })
            .unwrap_or(SOURCE)
    }
}

/// A type that can be executed like a standard program.
pub trait Executable {
    /// Executes the program.
    ///
    /// The arguments (`argc`, `argv`, and `envp`) can be used by the implementation
    /// for optimization purposes, but they may be ignored (and fetched with [`std::env::args_os()`]).
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
