//! Integration tests for the `sluice` CLI binary.
//!
//! Several tests below depend on properties of `fixtures/chunk-sample.gz`:
//!  - it contains at least one classified artifact (sources/javadoc/etc.),
//!  - it contains at least one record where `--full` adds output beyond the
//!    default classifier=NA filter.
//!
//! If you regenerate the fixture (`just regen-fixture`) from a chunk that
//! lacks these properties, those tests will fail.

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
fn include_removes_flag_is_accepted() {
    // The committed sample fixture happens to contain only adds, so we cannot
    // assert that "remove" lines actually appear. This just exercises the flag
    // wiring; per-record behaviour is covered by the core library tests.
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
    // Assert on the path itself (stable) rather than the human-readable
    // context verb (which can be reworded without changing behaviour).
    sluice()
        .arg("/nonexistent/path.gz")
        .assert()
        .failure()
        .stderr(predicate::str::contains("/nonexistent/path.gz"));
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

#[test]
fn full_mode_emits_classifier_field_for_classified_artifacts() {
    let output = sluice()
        .arg(fixture_path())
        .arg("--full")
        .output()
        .expect("binary runs");
    let stdout_str = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout_str.contains("\"classifier\""),
        "--full should emit at least one record with a classifier field"
    );
}

#[test]
fn full_mode_emits_more_records_than_default_mode() {
    fn line_count(args: &[&str]) -> usize {
        let out = sluice().args(args).output().expect("binary runs").stdout;
        String::from_utf8(out).expect("utf8 output").lines().count()
    }
    let default_count = line_count(&[fixture_path().to_str().unwrap()]);
    let full_count = line_count(&[fixture_path().to_str().unwrap(), "--full"]);
    assert!(
        full_count > default_count,
        "--full should yield more records ({full_count}) than default ({default_count})"
    );
}

#[test]
fn every_emitted_line_is_valid_json() {
    let output = sluice().arg(fixture_path()).output().expect("binary runs");
    let stdout_str = String::from_utf8(output.stdout).unwrap();
    let mut count = 0usize;
    for line in stdout_str.lines() {
        let parsed: serde_json::Value =
            serde_json::from_str(line).unwrap_or_else(|e| panic!("bad json: {line}: {e}"));
        // Every record must carry a type discriminator.
        assert!(parsed.get("type").is_some(), "missing type in: {line}");
        count += 1;
    }
    assert!(count > 0, "fixture should produce at least one line");
}

#[test]
fn stdin_input_is_accepted() {
    use std::fs;
    let bytes = fs::read(fixture_path()).expect("read fixture");
    sluice()
        .write_stdin(bytes)
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""type":"add""#));
}
