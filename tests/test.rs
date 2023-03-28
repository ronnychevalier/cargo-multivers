use std::path::Path;
use std::process::Command;

use assert_cmd::assert::Assert;
use assert_cmd::prelude::OutputAssertExt;

use escargot::CargoBuild;

fn build_and_run_crate(name: &str) -> Assert {
    let multivers_manifest = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let test_manifest = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join(name)
        .join("Cargo.toml");

    let assert = CargoBuild::new()
        .manifest_path(multivers_manifest)
        .run()
        .unwrap()
        .command()
        .arg("multivers")
        .arg("--manifest-path")
        .arg(test_manifest)
        .assert()
        .success();

    // Until we output json like cargo we need to parse the output manually
    let output = assert.get_output();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let last_line = stdout.lines().last().unwrap();
    let (_, path) = last_line.split_once('(').unwrap();
    let multivers_runner = path.strip_suffix(')').map(Path::new).unwrap();

    Command::new(multivers_runner).assert().success()
}

/// Checks that we can build a crate that does nothing
/// and that it can run successfully.
#[test]
fn crate_that_does_nothing() {
    build_and_run_crate("test-nothing").stdout("");
}
