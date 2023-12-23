use std::path::Path;
use std::process::Command;

use assert_cmd::prelude::OutputAssertExt;

use escargot::CargoBuild;

use predicates::prelude::*;

#[cfg(test)]
fn build_crate(
    name: &str,
    modify_command_callback: impl FnOnce(&mut std::process::Command),
) -> Command {
    let multivers_manifest = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let test_manifest = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join(name)
        .join("Cargo.toml");

    let mut command = CargoBuild::new()
        .manifest_path(multivers_manifest)
        .run()
        .unwrap()
        .command();

    command
        .arg("multivers")
        .arg("--manifest-path")
        .arg(test_manifest);

    modify_command_callback(&mut command);

    let assert = command.assert().success();

    // Until we output json like cargo we need to parse the output manually
    let output = assert.get_output();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let last_line = stdout.lines().last().unwrap();
    let (_, path) = last_line.split_once('(').unwrap();
    let multivers_runner = path.strip_suffix(')').map(Path::new).unwrap();

    Command::new(multivers_runner)
}

/// Checks that we can build a crate that does nothing and that it can run successfully.
///
/// It should build without a runner since every build leads to the same binary.
#[test]
#[cfg_attr(coverage, ignore)]
fn crate_that_does_nothing() {
    build_crate("test-nothing", |_| ())
        .assert()
        .success()
        .stdout("");
}

/// Checks that we can build a crate that prints its argv and that works as expected
#[test]
fn crate_that_prints_argv() {
    let expected_args = ["z", "foo2", "''"];
    build_crate("test-argv", |_| ())
        .args(expected_args)
        .assert()
        .success()
        .stdout(predicate::str::ends_with(format!(
            "{}\n",
            expected_args.join(" ")
        )));
}

/// Checks that we can build a crate that is part of a workspace.
///
/// Regression test (see #5).
#[test]
fn crate_within_workspace() {
    let expected_args = ["workspace", "abc", "0987"];
    build_crate("test-workspace", |_| ())
        .args(expected_args)
        .assert()
        .success()
        .stdout(predicate::str::ends_with(format!(
            "{}\n",
            expected_args.join(" ")
        )));
}

/// Checks that we can build a crate with `CARGO_UNSTABLE_BUILD_STD=std`.
///
/// Regression test (see #7).
#[test]
#[cfg_attr(coverage, ignore)]
fn rebuild_std_env() {
    let expected_args = ["z", "foo2", "''"];
    build_crate("test-argv", |command| {
        command.env("CARGO_UNSTABLE_BUILD_STD", "std");
    })
    .args(expected_args)
    .assert()
    .success()
    .stdout(predicate::str::ends_with(format!(
        "{}\n",
        expected_args.join(" ")
    )));
}
