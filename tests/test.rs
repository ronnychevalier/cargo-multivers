use std::path::Path;
use std::process::Command;

use assert_cmd::prelude::OutputAssertExt;

use escargot::CargoBuild;

use predicates::prelude::*;

#[cfg(test)]
fn cargo_multivers() -> Command {
    let multivers_manifest = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");

    let mut cargo_multivers = CargoBuild::new()
        .manifest_path(multivers_manifest)
        .run()
        .unwrap()
        .command();

    cargo_multivers.arg("multivers");

    cargo_multivers
}

#[cfg(test)]
fn build_crate(
    name: &str,
    modify_command_callback: impl FnOnce(&mut std::process::Command),
) -> (Command, tempfile::TempDir) {
    let out_dir: tempfile::TempDir = tempfile::tempdir().unwrap();
    let test_manifest = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join(name)
        .join("Cargo.toml");

    let mut cargo_multivers = cargo_multivers();
    cargo_multivers
        .arg("--manifest-path")
        .arg(test_manifest)
        .arg("--out-dir")
        .arg(out_dir.path());

    modify_command_callback(&mut cargo_multivers);

    let _assert: assert_cmd::assert::Assert = cargo_multivers.assert().success();

    let multivers_runner = out_dir
        .path()
        .join(format!("{name}{}", std::env::consts::EXE_SUFFIX));

    assert_eq!(std::fs::read_dir(out_dir.path()).into_iter().count(), 1);
    assert!(multivers_runner.exists());

    (Command::new(multivers_runner), out_dir)
}

#[test]
fn print_cpu_features() {
    let mut cargo_multivers = cargo_multivers();
    cargo_multivers.arg("--print=cpu-features");
    cargo_multivers.arg("--target=x86_64-unknown-linux-gnu");
    let assert = cargo_multivers.assert().success();
    let output = &assert.get_output().stdout;
    assert!(!output.is_empty());

    let output = String::from_utf8_lossy(output);
    assert!(output.contains("avx2"));
    assert!(output.contains("xsave"));
}

/// Checks that we can build a crate that does nothing and that it can run successfully.
///
/// It should build without a runner since every build leads to the same binary.
#[test]
fn crate_that_does_nothing() {
    build_crate("test-nothing", |_| ())
        .0
        .assert()
        .success()
        .stdout("");
}

/// Checks that we can build a crate that prints its argv and that works as expected
#[test]
fn crate_that_prints_argv() {
    let expected_args = ["z", "foo2", "''"];
    build_crate("test-argv", |_| ())
        .0
        .args(expected_args)
        .assert()
        .success()
        .stdout(predicate::str::ends_with(format!(
            "{}\n",
            expected_args.join(" ")
        )));
}

/// Checks that `-- --target-dir` works.
#[test]
fn target_dir_arg() {
    let target_dir = tempfile::tempdir().unwrap();
    let expected_args = ["target", "diiiiir", "''"];
    build_crate("test-argv", |command| {
        command.arg("--").arg("--target-dir").arg(target_dir.path());
    })
    .0
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
        .0
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
    .0
    .args(expected_args)
    .assert()
    .success()
    .stdout(predicate::str::ends_with(format!(
        "{}\n",
        expected_args.join(" ")
    )));
}

#[test]
fn profile_dev() {
    let expected_args = ["z", "foo2", "''"];
    build_crate("test-argv", |command| {
        command.arg("--profile=dev");
    })
    .0
    .args(expected_args)
    .assert()
    .success()
    .stdout(predicate::str::ends_with(format!(
        "{}\n",
        expected_args.join(" ")
    )));
}
