# cargo multivers

Cargo subcommand to build multiple versions of the same binary to generate a portable optimized binary.

:construction: This is a WIP. It has not been published yet. :construction:

If you want to apply this approach only at the function level, take a look at the [multiversion](https://crates.io/crates/multiversion) crate.

## Usage

```bash
cargo +nightly multivers
```

## Related Work

- <https://crates.io/crates/multiversion>
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
