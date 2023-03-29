use std::io::Write;

use flate2::read::DeflateDecoder;

/// Stores a build and the CPU features it requires
pub struct Build<'a> {
    pub compressed_build: &'a [u8],

    pub features: &'a [&'a str],
}

impl<'a> Build<'a> {
    pub const fn required_cpu_features(&self) -> &[&str] {
        &self.features
    }

    /// Decompresses the build into a writer
    pub fn decompress_into(&self, mut output: impl Write) -> std::io::Result<()> {
        let mut decoder = DeflateDecoder::new(self.compressed_build);

        std::io::copy(&mut decoder, &mut output).map(|_| ())
    }
}
