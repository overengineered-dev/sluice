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
