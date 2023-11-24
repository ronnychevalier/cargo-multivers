use std::path::Path;
use std::process::Command;

use assert_cmd::prelude::OutputAssertExt;

use escargot::CargoBuild;

use predicates::prelude::*;

#[cfg(test)]
fn build_crate(name: &str) -> Command {
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

    Command::new(multivers_runner)
}

/// Checks that we can build a crate that does nothing and that it can run successfully
#[test]
fn crate_that_does_nothing() {
    build_crate("test-nothing").assert().success().stdout("");
}

/// Checks that we can build a crate that prints its argv and that works as expected
#[test]
fn crate_that_prints_argv() {
    let expected_args = ["z", "foo2", "''"];
    build_crate("test-argv")
        .args(expected_args)
        .assert()
        .success()
        .stdout(predicate::str::ends_with(format!(
            "{}\n",
            expected_args.join(" ")
        )));
}
