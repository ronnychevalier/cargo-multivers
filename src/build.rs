use std::io::{Read, Write};

use anyhow::Context;

use bincode::{Decode, Encode};

use flate2::read::DeflateDecoder;
use flate2::write::DeflateEncoder;
use flate2::Compression;

/// Stores a build and the CPU features it requires
#[derive(Encode, Decode)]
pub struct Build {
    compressed_build: Box<[u8]>,

    features: Box<[String]>,
}

impl Build {
    pub fn compress(build: &[u8], features: Vec<String>) -> anyhow::Result<Self> {
        let mut encoder = DeflateEncoder::new(Vec::new(), Compression::best());
        encoder.write_all(build)?;
        let build = encoder.finish().context("Failed to compress build")?;

        Ok(Self {
            compressed_build: build.into_boxed_slice(),
            features: features.into_boxed_slice(),
        })
    }

    pub fn required_cpu_features(&self) -> &[String] {
        self.features.as_ref()
    }

    pub fn decompress(self) -> anyhow::Result<Box<[u8]>> {
        let mut build = Vec::with_capacity(self.compressed_build.len());
        let mut decoder = DeflateDecoder::new(self.compressed_build.as_ref());
        decoder.read_to_end(&mut build)?;

        Ok(build.into_boxed_slice())
    }
}
