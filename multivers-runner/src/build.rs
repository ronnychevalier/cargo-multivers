use std::io::Read;

use flate2::read::DeflateDecoder;

/// Stores a build and the CPU features it requires
pub struct Build<'a> {
    pub compressed_build: &'a [u8],

    pub features: &'a [&'a str],
}

impl<'a> Build<'a> {
    pub fn required_cpu_features(&self) -> &[&str] {
        self.features.as_ref()
    }

    pub fn decompress(self) -> anyhow::Result<Box<[u8]>> {
        let mut build = Vec::with_capacity(self.compressed_build.len());
        let mut decoder = DeflateDecoder::new(self.compressed_build);
        decoder.read_to_end(&mut build)?;

        Ok(build.into_boxed_slice())
    }
}
