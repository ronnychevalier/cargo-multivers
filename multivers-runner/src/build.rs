use std::convert::Infallible;
use std::io::Write;

include!(concat!(env!("OUT_DIR"), "/builds.rs"));

/// Stores a build and the CPU features it requires
pub struct Build<'a> {
    compressed_build: &'a [u8],

    features: &'a [&'a str],
}

impl<'a> Build<'a> {
    /// Decompresses the build into a writer
    pub fn decompress_into(&self, output: impl Write) -> std::io::Result<()> {
        cfg_if::cfg_if! {
            if #[cfg(feature = "deflate")] {
                let mut output = output;
                let mut decoder = flate2::read::DeflateDecoder::new(self.compressed_build);
                std::io::copy(&mut decoder, &mut output).map(|_| ())
            } else if #[cfg(feature = "lz4")] {
                let mut output = output;
                let mut decoder = lz4_flex::frame::FrameDecoder::new(self.compressed_build);
                std::io::copy(&mut decoder, &mut output).map(|_| ())
            } else if #[cfg(feature = "zstd")] {
                zstd::stream::copy_decode(self.compressed_build, output)
            }
        }
    }

    /// Finds a version that matches the CPU features of the host
    pub fn find() -> Option<Self> {
        let supported_features: Vec<&str> = std_detect::detect::features()
            .filter_map(|(feature, supported)| supported.then_some(feature))
            .collect();

        BUILDS.into_iter().find_map(|build| {
            build
                .features
                .iter()
                .all(|feature| supported_features.contains(feature))
                .then_some(build)
        })
    }
}

pub trait Executable {
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
