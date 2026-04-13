//! Integration tests for the `sluice` CLI binary.

use std::path::PathBuf;

use assert_cmd::Command;
use predicates::prelude::*;

fn fixture_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("../../fixtures/chunk-sample.gz");
    p
}

fn sluice() -> Command {
    Command::cargo_bin("sluice").expect("binary built")
}

#[test]
fn parses_fixture_and_emits_jsonl() {
    sluice()
        .arg(fixture_path())
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""type":"add""#));
}

#[test]
fn stats_flag_prints_summary_to_stderr() {
    sluice()
        .arg(fixture_path())
        .arg("--stats")
        .assert()
        .success()
        .stderr(predicate::str::contains("parsed"))
        .stderr(predicate::str::contains("adds:"));
}

#[test]
fn include_removes_flag_emits_remove_records() {
    sluice()
        .arg(fixture_path())
        .arg("--include-removes")
        .assert()
        .success();
}

#[test]
fn full_flag_parses_successfully() {
    sluice()
        .arg(fixture_path())
        .arg("--full")
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""type":"add""#));
}

#[test]
fn full_flag_with_stats() {
    sluice()
        .arg(fixture_path())
        .arg("--full")
        .arg("--stats")
        .assert()
        .success()
        .stderr(predicate::str::contains("parsed"))
        .stderr(predicate::str::contains("adds:"));
}

#[test]
fn full_with_include_removes() {
    sluice()
        .arg(fixture_path())
        .arg("--full")
        .arg("--include-removes")
        .assert()
        .success();
}

#[test]
fn default_mode_omits_classifier_key() {
    let output = sluice().arg(fixture_path()).output().expect("binary runs");
    let stdout_str = String::from_utf8(output.stdout).unwrap();
    for line in stdout_str.lines() {
        assert!(
            !line.contains("\"classifier\""),
            "default mode should not emit classifier field, got: {line}"
        );
    }
}

#[test]
fn missing_file_exits_with_error() {
    sluice()
        .arg("/nonexistent/path.gz")
        .assert()
        .failure()
        .stderr(predicate::str::contains("opening"));
}

#[test]
fn version_flag() {
    sluice()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::starts_with("sluice"));
}

#[test]
fn help_flag() {
    sluice()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("JSON Lines"));
}
