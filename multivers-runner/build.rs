//! Build script that generates a Rust file that contains a compressed source binary and a set of compressed patches for each CPU features set.
//!
//! It reads a JSON file that contains a set of paths to executables and their dependency on CPU features
//! from the environment variable `MULTIVERS_BUILDS_DESCRIPTION_PATH`.
//! Then, it generates a Rust file that contains the source and the patches.
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};

use bzip2::Compression;
use bzip2::read::BzEncoder;

use qbsdiff::Bsdiff;

use quote::quote;

use serde::Deserialize;

use proc_exit::Exit;

#[derive(Default, Deserialize)]
struct BuildDescription {
    path: PathBuf,
    features: Vec<String>,
}

#[derive(Default, Deserialize)]
struct BuildsDescription {
    builds: Vec<BuildDescription>,
}

impl BuildsDescription {
    /// Loads a [`BuildsDescription`] from a JSON file located at the path in the environment variable `MULTIVERS_BUILDS_DESCRIPTION_PATH`
    pub fn from_env() -> Option<Result<Self, Exit>> {
        let path = option_env!("MULTIVERS_BUILDS_DESCRIPTION_PATH")?;

        println!("cargo:rerun-if-env-changed=MULTIVERS_BUILDS_DESCRIPTION_PATH");
        println!("cargo:rerun-if-changed={path}");

        Some(Self::from_path(path))
    }

    fn from_path(path: impl AsRef<Path>) -> Result<Self, Exit> {
        let path = path.as_ref();
        let file = File::open(path).map_err(|_| {
            proc_exit::sysexits::IO_ERR.with_message(format!(
                "Failed to open the builds description file {}",
                path.display()
            ))
        })?;
        let mut builds_desc: Self =
            serde_json::from_reader(BufReader::new(file)).map_err(|_| {
                proc_exit::sysexits::DATA_ERR.with_message(format!(
                    "Failed to parse the builds description file {}",
                    path.display(),
                ))
            })?;

        builds_desc.sort_by_features();
        builds_desc.print_rerun();

        Ok(builds_desc)
    }

    fn remove_source(&mut self) -> Option<BuildDescription> {
        // The source is one requiring no or the minimum amount of features.
        // Since we sorted the builds by features, we just have to remove the last element.
        // We should make it configurable at some point.
        self.builds.pop()
    }

    /// Sort the builds to put the ones requiring more features at the top
    fn sort_by_features(&mut self) {
        self.builds.sort_unstable_by(|build1, build2| {
            build1.features.len().cmp(&build2.features.len()).reverse()
        });
    }

    /// Prints on stdout `cargo:rerun-if-changed` for each build
    fn print_rerun(&self) {
        let mut stdout = std::io::stdout().lock();
        for build in &self.builds {
            let _ = writeln!(stdout, "cargo:rerun-if-changed={}", build.path.display());
        }
    }

    pub fn generate_sources(mut self, dest_path: &Path) -> Result<(), Exit> {
        let source_build = self.remove_source();

        if source_build.is_none() {
            println!(
                "cargo:warning=The JSON file loaded from the environment variable MULTIVERS_BUILDS_DESCRIPTION_PATH must contain builds."
            );
            println!("cargo:warning=It will build, but it will fail at runtime.");
        }

        let source = source_build
            .as_ref()
            .map(|source| {
                std::fs::read(&source.path).map_err(|_| {
                    proc_exit::sysexits::IO_ERR.with_message(format!(
                        "Failed to read source build {}",
                        source.path.display(),
                    ))
                })
            })
            .transpose()?
            .unwrap_or_default();
        let source_features = source_build.map(|s| s.features).unwrap_or_default();

        let out_dir_env = std::env::var_os("OUT_DIR").ok_or_else(|| {
            proc_exit::sysexits::SOFTWARE_ERR.with_message("Missing OUT_DIR environment variable")
        })?;
        let out_dir = Path::new(&out_dir_env);

        let patches = self
            .builds
            .into_iter()
            .enumerate()
            .map(|(i, build)| {
                let target = std::fs::read(&build.path).map_err(|_| {
                    proc_exit::sysexits::IO_ERR
                        .with_message(format!("Failed to read build {}", build.path.display()))
                })?;
                let patch = bsdiff(&source, &target)?;
                let features = build.features;

                let patch_filename = format!("patch_{i}.bin");
                let patch_path = out_dir.join(&patch_filename);
                std::fs::write(&patch_path, &patch).map_err(|_| {
                    proc_exit::sysexits::IO_ERR.with_message(format!(
                        "Failed to write patch file {}",
                        patch_path.display(),
                    ))
                })?;

                Ok(quote! {
                    Build {
                        compressed: include_bytes!(concat!(env!("OUT_DIR"), "/", #patch_filename)),
                        features: &[
                            #(#features),*
                        ],
                        source: Some(&SOURCE),
                    }
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

        let source_compressed = compress(&source[..])?;

        let source_filename = "source.bin";
        let source_file = out_dir.join(source_filename);
        std::fs::write(&source_file, &source_compressed).map_err(|_| {
            proc_exit::sysexits::IO_ERR.with_message(format!(
                "Failed to write compressed source file {}",
                source_file.display(),
            ))
        })?;

        let n_builds = patches.len();
        let tokens = quote! {
            const SOURCE: Build<'_> = Build {
                compressed: include_bytes!(concat!(env!("OUT_DIR"), "/", #source_filename)),
                features: &[
                    #(#source_features),*
                ],
                source: None,
            };
            const PATCHES: [Build<'_>; #n_builds] = [
                #(#patches),*
            ];
        };

        std::fs::write(dest_path, tokens.to_string()).map_err(|_| {
            proc_exit::sysexits::IO_ERR.with_message(format!(
                "Failed to write generated Rust file to {}",
                dest_path.display(),
            ))
        })?;

        Ok(())
    }
}

fn compress(reader: impl BufRead) -> Result<Vec<u8>, Exit> {
    let mut compressor = BzEncoder::new(reader, Compression::best());
    let mut buffer = Vec::new();
    compressor
        .read_to_end(&mut buffer)
        .map_err(|_| proc_exit::sysexits::IO_ERR.with_message("Failed to compress data"))?;

    Ok(buffer)
}

fn bsdiff(source: &[u8], target: &[u8]) -> Result<Vec<u8>, Exit> {
    let mut patch = Vec::new();
    Bsdiff::new(source, target)
        .compare(std::io::Cursor::new(&mut patch))
        .map_err(|_| proc_exit::sysexits::IO_ERR.with_message("Failed to generate a patch"))?;
    Ok(patch)
}

fn main() -> Result<(), Exit> {
    println!("cargo:rerun-if-changed=build.rs");

    let out_dir = std::env::var_os("OUT_DIR").ok_or_else(|| {
        proc_exit::sysexits::SOFTWARE_ERR.with_message("Missing OUT_DIR environment variable")
    })?;
    let dest_path = Path::new(&out_dir).join("builds.rs");

    let builds = BuildsDescription::from_env()
        .transpose()?
        .unwrap_or_default();

    builds.generate_sources(&dest_path)?;

    Ok(())
}
