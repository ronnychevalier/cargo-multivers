use std::io::Write;

use flate2::read::DeflateDecoder;

include!(concat!(env!("OUT_DIR"), "/builds.rs"));

/// Stores a build and the CPU features it requires
pub struct Build<'a> {
    compressed_build: &'a [u8],

    features: &'a [&'a str],
}

impl<'a> Build<'a> {
    /// Decompresses the build into a writer
    pub fn decompress_into(&self, mut output: impl Write) -> std::io::Result<()> {
        let mut decoder = DeflateDecoder::new(self.compressed_build);

        std::io::copy(&mut decoder, &mut output).map(|_| ())
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
