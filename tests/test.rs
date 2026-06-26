//! Integration tests
use std::path::Path;
use std::process::Command;

use assert_cmd::assert::Assert;
use assert_cmd::prelude::OutputAssertExt;

use escargot::CargoBuild;

use predicates::prelude::*;

#[cfg(test)]
fn cargo_multivers() -> Command {
    let mut cargo_multivers = CargoBuild::new()
        .bin("cargo-multivers")
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
) -> (Assert, tempfile::TempDir) {
    use std::ffi::OsStr;

    use itertools::Itertools;

    let out_dir: tempfile::TempDir = tempfile::tempdir().unwrap();
    let target_dir = out_dir.path().join("target");
    std::fs::create_dir(&target_dir).unwrap();
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

    if !cargo_multivers.get_args().contains(OsStr::new("--")) {
        cargo_multivers.arg("--");
    }
    cargo_multivers.args(["--target-dir", &target_dir.display().to_string()]);

    (cargo_multivers.assert(), out_dir)
}

#[cfg(test)]
fn build_and_run_crate(
    crate_name: &str,
    bin_name: Option<&str>,
    modify_command_callback: impl FnOnce(&mut std::process::Command),
) -> (Command, tempfile::TempDir) {
    let (assert, out_dir) = build_crate(crate_name, modify_command_callback);
    println!("{assert}");

    let name = bin_name.unwrap_or(crate_name);
    let multivers_runner = out_dir
        .path()
        .join(format!("{name}{}", std::env::consts::EXE_SUFFIX));

    assert!(multivers_runner.exists());
    assert_eq!(
        std::fs::read_dir(out_dir.path())
            .unwrap()
            .filter_map(Result::ok)
            .filter(|e| e.file_type().unwrap().is_file())
            .count(),
        1
    );

    (Command::new(multivers_runner), out_dir)
}

/// Checks that we can build a crate that does nothing and that it can run successfully.
///
/// It should build without a runner since every build leads to the same binary.
#[test]
#[cfg(any(target_os = "linux", windows))]
fn crate_that_does_nothing() {
    build_and_run_crate("test-nothing", None, |_command| {
        #[cfg(coverage)]
        _command.env_remove("RUSTFLAGS");
    })
    .0
    .assert()
    .success()
    .stdout("");
}

/// Checks that it can build a crate that prints its argv
#[test]
fn crate_that_prints_argv() {
    let expected_args = ["z", "foo2", "''"];
    build_and_run_crate("test-argv", None, |_| ())
        .0
        .args(expected_args)
        .assert()
        .success()
        .stdout(predicate::str::ends_with(format!(
            "{}\n",
            expected_args.join(" ")
        )));
}

/// Checks that it can build a crate that prints its argv with a custom runner
#[test]
fn crate_that_prints_argv_with_custom_runner() {
    let custom_runner = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("custom-runner")
        .join("Cargo.toml");
    let expected_args = ["custom", "foo2", "''"];
    build_and_run_crate("test-argv", None, |command| {
        command.args([
            "--runner-manifest-path",
            custom_runner.as_os_str().to_str().unwrap(),
        ]);
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

/// Checks that it fails to build with an invalid custom runner manifest path
#[test]
fn invalid_custom_runner_manifest_path() {
    let custom_runner = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("custom-runner")
        .join("invalid.toml");
    build_crate("test-argv", |command| {
        command.args([
            "--runner-manifest-path",
            custom_runner.as_os_str().to_str().unwrap(),
        ]);
    })
    .0
    .failure();
}

/// Checks that `cargo multivers` can work by selecting a bin when a package has multiple bins.
///
/// See #15
#[test]
fn multiple_bins_selected() {
    build_and_run_crate("test-multiplebins", Some("bin1"), |command| {
        command.args(["--", "--bin", "bin1"]);
    })
    .0
    .assert()
    .success()
    .stdout("bin1\n");

    build_and_run_crate("test-multiplebins", Some("bin2"), |command| {
        command.args(["--", "--bin", "bin2"]);
    })
    .0
    .assert()
    .success()
    .stdout("bin2\n");
}

/// Checks that `$CARGO_HOME/config.toml` is taken into account when building a crate
///
/// See #11
#[test]
#[cfg_attr(coverage, ignore)]
fn crate_cargo_config_invalid() {
    let cargo_home = tempfile::tempdir().unwrap();
    let invalid_config = r#"[build]
rustflags = ["invalid flag"]
    "#;

    std::fs::write(cargo_home.path().join("config.toml"), invalid_config).unwrap();

    build_crate("test-argv", |command| {
        command.env("CARGO_HOME", cargo_home.path());
    })
    .0
    .failure();
}

/// Checks that we can build a crate that is part of a workspace.
///
/// Regression test (see #5).
#[test]
fn crate_within_workspace() {
    let expected_args = ["workspace", "abc", "0987"];
    build_and_run_crate("test-workspace", None, |_| ())
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
fn rebuild_std_env() {
    let expected_args = ["z", "foo2", "''"];
    build_and_run_crate("test-argv", None, |command| {
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

/// Checks that we can build using the dev profile
#[test]
fn profile_dev() {
    let expected_args = ["z", "foo2", "''"];
    build_and_run_crate("test-argv", None, |command| {
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

/// Checks that the produced binary uses the most optimized build possible.
///
/// Specifically, checks that `feature is supported -> feature is used at runtime`
/// holds (when `native` is among compiled-for CPUs).
///
/// Regression test (see #20).
#[test]
#[cfg(target_arch = "x86_64")]
fn correct_build_used() {
    macro_rules! runtime_features_string {
        ($($target_feature_name: tt)*) => {
            [
                $(
                    {
                        if is_x86_feature_detected!($target_feature_name) {
                            Some($target_feature_name)
                        } else {
                            None
                        }
                    }
                ),*
            ]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join(",")
        };
    }
    let runtime_features = runtime_features_string!(
        "adx" "aes" "avx" "avx2" "bmi1" "bmi2" "cmpxchg16b" "f16c" "fma"
        "fxsr" "lzcnt" "movbe" "pclmulqdq" "popcnt" "rdrand" "rdseed" "sha"
        "sse" "sse2" "sse3" "sse4.1" "sse4.2" "ssse3"
        "xsave" "xsavec" "xsaveopt" "xsaves"
    );

    build_and_run_crate("test-correct-build-used", None, |build_command| {
        build_command.args(["--cpus", "x86-64,x86-64-v2,x86-64-v3,x86-64-v4,native"]);
    })
    .0
    .assert()
    .success()
    .stdout(predicate::eq(runtime_features));
}
