use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};

use bzip2::read::BzEncoder;
use bzip2::Compression;

use qbsdiff::Bsdiff;

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

fn compress(reader: impl BufRead) -> Vec<u8> {
    let mut compressor = BzEncoder::new(reader, Compression::best());
    let mut buffer = Vec::new();
    compressor.read_to_end(&mut buffer).unwrap();

    buffer
}

fn bsdiff(source: &[u8], target: &[u8]) -> Vec<u8> {
    let mut patch = Vec::new();
    Bsdiff::new(source, target)
        .compare(std::io::Cursor::new(&mut patch))
        .unwrap();
    patch
}

fn main() {
    let out_dir = std::env::var_os("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("builds.rs");

    let mut builds: BuildsDescription =
        if let Some(path) = option_env!("MULTIVERS_BUILDS_DESCRIPTION_PATH") {
            println!("cargo:rerun-if-changed={path}");

            let file = File::open(path).expect("Failed to open the builds description file");
            rmp_serde::from_read(BufReader::new(file))
                .expect("Failed to parse the builds description file")
        } else {
            Default::default()
        };
    // We sort them to put the builds requiring more features at the top.
    builds.builds.sort_unstable_by(|build1, build2| {
        build1.features.len().cmp(&build2.features.len()).reverse()
    });

    // The source is one requiring no or the minimum amount of features.
    // We should make it configurable at some point.
    let source_build = if let Some(source) = builds.builds.pop() {
        source
    } else {
        println!("No builds");
        std::process::exit(1);
    };

    let source = std::fs::read(source_build.path).unwrap();
    let source_features = source_build.features;

    let patches = builds
        .builds
        .into_iter()
        .map(|build| {
            println!("cargo:rerun-if-changed={}", build.path.display());

            let target = std::fs::read(&build.path).unwrap();
            let patch = bsdiff(&source, &target);
            println!("cargo:warning=PATCH SIZE {}", patch.len());
            let features = build.features;
            quote! {
                Build {
                    compressed_build: &[
                        #(#patch),*
                    ],
                    features: &[
                        #(#features),*
                    ],
                    source: false,
                }
            }
        })
        .collect::<Vec<_>>();

    let source = compress(&source[..]);

    let n_builds = patches.len();
    let tokens = quote! {
        const SOURCE: Build = Build {
            compressed_build: &[
                #(#source),*
            ],
            features: &[
                #(#source_features),*
            ],
            source: true,
        };
        const PATCHES: [Build; #n_builds] = [
            #(#patches),*
        ];
    };

    std::fs::write(dest_path, tokens.to_string()).unwrap();

    println!("cargo:rerun-if-changed=build.rs");
}
