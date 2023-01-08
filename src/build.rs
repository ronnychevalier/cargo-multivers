use anyhow::Context;

use bincode::{config, Decode, Encode};

use flate2::read::DeflateDecoder;
use flate2::write::DeflateEncoder;
use flate2::Compression;

/// Stores a build and the CPU features it requires
#[derive(Encode, Decode)]
pub struct Build {
    build: Box<[u8]>,

    features: Box<[String]>,
}

impl Build {
    pub fn new(build: Vec<u8>, features: Vec<String>) -> Self {
        Self {
            build: build.into_boxed_slice(),
            features: features.into_boxed_slice(),
        }
    }

    pub fn required_cpu_features(&self) -> &[String] {
        self.features.as_ref()
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.build.as_ref()
    }

    pub fn into_encoded_bytes(self, compress: bool) -> anyhow::Result<Vec<u8>> {
        let config = config::standard();

        if compress {
            let mut encoder = DeflateEncoder::new(Vec::new(), Compression::best());
            bincode::encode_into_std_write(self, &mut encoder, config)
                .context("Failed to encode build")?;
            encoder.finish().context("Failed to compress build")
        } else {
            bincode::encode_to_vec(self, config).context("Failed to encode build")
        }
    }

    pub fn from_encoded_bytes(bytes: impl AsRef<[u8]>) -> anyhow::Result<Self> {
        let config = config::standard();

        let mut decoder = DeflateDecoder::new(bytes.as_ref());
        bincode::decode_from_std_read(&mut decoder, config).context("Failed to decode build")
    }
}
