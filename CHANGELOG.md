# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/)
and this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Changed

- The runner no longer depends on a Git repository.
  `std_detect` has been forked and an up-to-date version called `notstd_detect` has been uploaded to `crates.io` to access the requires features.
- Update dependencies.

[Unreleased]: https://github.com/ronnychevalier/cargo-multivers/compare/v0.5.0...HEAD

## [0.5.0] - 12-08-2023

### Changed

- `cargo multivers` now stops on the first error it encounters.
- The file that contains the list of versions built given to the runner is now in JSON.
- Updated dependencies to fix build with Rust nightly and proc-macro2.
- MSRV is now 1.66.

### Fixed

- Removed duplicated error messages.
- Do not display a false ETA for the first build.

[0.5.0]: https://github.com/ronnychevalier/cargo-multivers/compare/v0.4.1...v0.5.0

## [0.4.1] - 18-06-2023

### Added

- You can now specify the list of CPUs you want to target with `--cpus` on the command line or by specifying it in your `Cargo.toml` like:

```toml
[package.metadata.multivers.x86_64]
cpus = ["generic", "alderlake", "skylake", "sandybridge", "ivybridge"]
```

### Changed

- Updated dependencies to remove the duplicated ones
- Removed unneeded unsafe code

[0.4.1]: https://github.com/ronnychevalier/cargo-multivers/compare/v0.4.0...v0.4.1

## [0.4.0] - 20-05-2023

### Changed

- Changed the runner to use compressed binary patches instead of compressing each version to reduce the size of the resulting runner.
  The runner now contains a compressed source binary and a set of compressed binary patches.
  When executed, the runner will find the patch the source binary with a patch that relies on CPU features that the host has and run the resulting binary.
- Updated dependencies
- Improved the error messages of the runner's build script.

[0.4.0]: https://github.com/ronnychevalier/cargo-multivers/compare/v0.3.2...v0.4.0

## [0.3.2] - 14-04-2023

### Changed

- Improved the runner performance
- Reduced the size of the runner
- Updated dependencies

[0.3.2]: https://github.com/ronnychevalier/cargo-multivers/compare/v0.3.1...v0.3.2

## [0.3.1] - 14-04-2023

### Changed

- Improved the runner performance
- Reduced the size of the runner
- Updated dependencies

[0.3.1]: https://github.com/ronnychevalier/cargo-multivers/compare/v0.3.0...v0.3.1

## [0.3.0] - 28-03-2023

### Changed

- A hash of each version built is computed to filter out duplicates (the ones requiring more features are removed) before building the runner. It means it can reduce the number of versions included in the runner, thus reducing its size.
- When building with MSVC the `/Brepro` linker flag is added to have reproducible builds and to ensure we can filter out duplicated versions.
- The runner is no longer built if only one version is left after removing the duplicated builds (the original binary is used).
- ⚠️ Breaking change: The final binary (the runner) has now the same name as the original name of the binary built (instead of `multivers-runner`).

[0.3.0]: https://github.com/ronnychevalier/cargo-multivers/compare/v0.2.1...v0.3.0

## [0.2.1] - 05-03-2023

### Changed

- Check that `cargo multivers` is running with Rust nightly channel before building everything.

### Fixed

- Do not overwrite each build with the next one (fix of the previous refactoring)

[0.2.1]: https://github.com/ronnychevalier/cargo-multivers/compare/v0.2.0...v0.2.1

## [0.2.0] - 27-02-2023

### Changed

- The last arguments are now forwarded to `cargo build`.
- The runner compresses the builds during its compilation.
- The runner no longer depends on `bincode`, since it does not serialize the builds anymore (they are only compressed).
- ⚠️ Breaking change: The `--rebuild-std` option has been removed. The last arguments are now forwarded to `cargo build`, the same can be achieved by giving `-- -Zbuild-std=std`.

[0.2.0]: https://github.com/ronnychevalier/cargo-multivers/compare/v0.1.0...v0.2.0

## [0.1.0] - 09-02-2023

This was the initial release of `cargo-multivers`.

[0.1.0]: https://github.com/ronnychevalier/cargo-multivers/releases/tag/v0.1.0
