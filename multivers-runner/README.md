# `multivers-runner`

This crate can be used to create a portable binary that embeds multiple versions of an executable each using a different CPU feature set.

Take a look at [`cargo multivers`][cargo-multivers], it does all the work for you: build the multiple versions and build the final binary that embeds them.

## How Does it Work?

The build script parses a JSON description file (see an example below) that contains a set of paths to executables with their dependency on CPU features
from the environment variable `MULTIVERS_BUILDS_DESCRIPTION_PATH`.
Then, it generates a Rust file that contains a compressed source binary and compressed binary patches to regenerate the other binaries from the source.
  
```json
{
  "builds": [
    {
      "path": "/path/to/binary-with-additional-cpu-features",
      "features": [
        "aes",
        "avx",
        "avx2",
        "sse",
        "sse2",
        "sse3",
        "sse4.1",
        "sse4.2",
        "ssse3",
      ]
    },
    {
      "path": "/path/to/binary-source",
      "features": [
        "sse",
        "sse2"
      ]
    }
  ]
}

```

At runtime, the function `main` uncompresses and executes the version that matches the CPU features of the host.
On Linux, it uses `memfd_create` and `fexecve` to do an in-memory execution.
On Windows, however, it writes the version in a temporary file and executes it.

## `cargo multivers`

This library is used by [`cargo multivers`][cargo-multivers] to build the final binary that embeds the multiple versions.

[cargo-multivers]: https://crates.io/crates/cargo-multivers
