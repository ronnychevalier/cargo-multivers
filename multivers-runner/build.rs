use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};

use flate2::{bufread::DeflateEncoder, Compression};

use quote::quote;

use serde::Deserialize;

#[derive(Default, Deserialize)]
struct BuildDescription {
    path: PathBuf,
    features: Vec<String>,
}

#[derive(Default, Deserialize)]
struct BuildsDescription {
    _version: Option<u8>,
    builds: Vec<BuildDescription>,
}

fn main() {
    let out_dir = std::env::var_os("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("builds.rs");

    let builds: BuildsDescription =
        if let Some(path) = option_env!("MULTIVERS_BUILDS_DESCRIPTION_PATH") {
            let file = File::open(path).expect("Failed to open the builds description file");
            rmp_serde::from_read(BufReader::new(file))
                .expect("Failed to parse the builds description file")
        } else {
            Default::default()
        };

    let builds = builds
        .builds
        .into_iter()
        .map(|build| {
            let file = File::open(&build.path).expect("Failed to open build");
            let reader = BufReader::new(file);
            let mut deflater = DeflateEncoder::new(reader, Compression::best());
            let mut buffer = Vec::new();
            deflater.read_to_end(&mut buffer).unwrap();

            let features = build.features;
            quote! {
                Build {
                    compressed_build: &[
                        #(#buffer),*
                    ],
                    features: &[
                        #(#features),*
                    ]
                }
            }
        })
        .collect::<Vec<_>>();

    let n_builds = builds.len();
    let tokens = quote! {
        const BUILDS: [Build; #n_builds] = [
            #(#builds),*
        ];
    };

    std::fs::write(&dest_path, tokens.to_string()).unwrap();

    println!("cargo:rerun-if-changed=build.rs");
}
