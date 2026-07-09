use std::io::Write;
use std::process::{Command, Output, Stdio};

fn run_with_stdin(input: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_vox-proof"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn binary");

    child
        .stdin
        .take()
        .expect("stdin is piped")
        .write_all(input.as_bytes())
        .expect("write stdin");

    child.wait_with_output().expect("wait for binary output")
}

#[test]
fn reports_parsed_segments_and_no_issues_for_valid_srt() {
    let output = run_with_stdin("1\n00:00:00,000 --> 00:00:02,500\nhello");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("parsed 1 segment"));
    assert!(stdout.contains("no validation issues"));
}

#[test]
fn reports_validation_issue_for_reversed_timing() {
    let output = run_with_stdin("1\n00:00:03,000 --> 00:00:02,500\nreversed");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("1 validation issue"));
    assert!(stdout.contains("segment position 0 (cue index 1):"));
}

#[test]
fn fails_on_malformed_srt() {
    let output = run_with_stdin("x\n00:00:00,000 --> 00:00:01,000\nhello");

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    assert!(stderr.contains("failed to parse"));
}
