# `cargo-multivers`

[![Latest Version]][crates.io]
![MSRV][rustc-image]
![CI status][ci-image]
![Apache 2.0 OR MIT licensed][license-image]

Cargo subcommand to build multiple versions of the same binary, each with a different CPU features set, merged into a single portable optimized binary.

## Overview

`cargo-multivers` builds multiple versions of the binary of a Rust package.
Each version is built with a set of CPU features (e.g., `+cmpxchg16b,+fxsr,+sse,+sse2,+sse3`) from a CPU (e.g., `ivybridge`) supported by the target (e.g., `x86_64-pc-windows-msvc`).
It does not build the powerset of the CPU features, but only a subset: from the list of CPU known to `rustc` for a given target, it fetches each set of CPU features and filters out
the duplicates.
Each version built is then compressed.
Finally, it builds a runner that contains all the compressed versions.
For instance, when building for the target `x86_64-pc-windows-msvc`, 37 different versions
will be built, compressed, and merged into a single portable binary.

When executed, the runner uncompresses and executes the build that matches the CPU features
of the host.

## Intended Use

While `cargo-multivers` could be used to build any kind of binary from a Rust package,
it is mostly intended for the following use cases:

- To build a project that is distributed to multiple users with different microarchitectures (e.g., a release version of your project).
- To build a program that performs long running tasks (e.g., heavy computations, a server, or a game).

## Installation

```bash
cargo install --locked cargo-multivers
```

## Usage

```bash
cargo +nightly multivers
```

## Related Work

- If you want to apply this approach only at the function level, take a look at the [multiversion](https://crates.io/crates/multiversion) crate.
- <https://www.intel.com/content/www/us/en/develop/documentation/vtune-cookbook/top/methodologies/compile-portable-optimized-binary.html>

## License

Licensed under either of

- Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or <https://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <https://opensource.org/licenses/MIT>)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any
additional terms or conditions.

[Latest Version]: https://img.shields.io/crates/v/cargo-multivers.svg
[crates.io]: https://crates.io/crates/cargo-multivers
[ci-image]: https://img.shields.io/github/actions/workflow/status/ronnychevalier/cargo-multivers/ci.yml
[rustc-image]: https://img.shields.io/badge/rustc-1.64+-blue.svg
[license-image]: https://img.shields.io/crates/l/cargo-multivers.svg
