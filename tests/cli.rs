use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

fn run_with_stdin(input: &str) -> Output {
    run_with_args_and_stdin(&[], input)
}

fn run_with_args_and_stdin(args: &[&str], input: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_vox-proof"))
        .args(args)
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

fn temp_dir(test_name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock is after unix epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "vox-proof-{test_name}-{}-{nanos}",
        std::process::id()
    ));
    std::fs::create_dir_all(&path).expect("create temp dir");
    path
}

fn write_input_srt(dir: &std::path::Path, contents: &str) -> PathBuf {
    let path = dir.join("input.srt");
    std::fs::write(&path, contents).expect("write input srt");
    path
}

fn write_session_terms(dir: &std::path::Path, contents: &str) -> PathBuf {
    let path = dir.join("session-terms.txt");
    std::fs::write(&path, contents).expect("write session terms");
    path
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

#[test]
fn review_accept_alternative_writes_reviewed_srt_and_decision_log() {
    let dir = temp_dir("review-accept");
    let input_path = write_input_srt(&dir, "1\n00:00:00,000 --> 00:00:01,000\nI use Kafka");
    let terms_path = write_session_terms(&dir, "Apache Kafka | Kafka");
    let reviewed_path = dir.join("reviewed.srt");
    let log_path = dir.join("decision-log.txt");

    let output = run_with_args_and_stdin(
        &[
            "review",
            input_path.to_str().expect("utf8 input path"),
            terms_path.to_str().expect("utf8 terms path"),
            reviewed_path.to_str().expect("utf8 reviewed path"),
            log_path.to_str().expect("utf8 log path"),
        ],
        "a 0\n",
    );

    assert!(output.status.success());
    let reviewed_srt = std::fs::read_to_string(&reviewed_path).expect("read reviewed srt");
    let decision_log = std::fs::read_to_string(&log_path).expect("read decision log");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("loaded 1 session term entries"));
    assert!(reviewed_srt.contains("Apache Kafka"));
    assert!(decision_log.contains("decision: accept_alternative"));
    assert!(decision_log.contains("alternative_index: 0"));
}

#[test]
fn review_reject_writes_unchanged_reviewed_srt_and_decision_log() {
    let dir = temp_dir("review-reject");
    let input_path = write_input_srt(&dir, "1\n00:00:00,000 --> 00:00:01,000\nI use Kafka");
    let terms_path = write_session_terms(&dir, "Apache Kafka | Kafka");
    let reviewed_path = dir.join("reviewed.srt");
    let log_path = dir.join("decision-log.txt");

    let output = run_with_args_and_stdin(
        &[
            "review",
            input_path.to_str().expect("utf8 input path"),
            terms_path.to_str().expect("utf8 terms path"),
            reviewed_path.to_str().expect("utf8 reviewed path"),
            log_path.to_str().expect("utf8 log path"),
        ],
        "r\n",
    );

    assert!(output.status.success());
    let reviewed_srt = std::fs::read_to_string(&reviewed_path).expect("read reviewed srt");
    let decision_log = std::fs::read_to_string(&log_path).expect("read decision log");
    assert!(reviewed_srt.contains("I use Kafka"));
    assert!(!reviewed_srt.contains("Apache Kafka"));
    assert!(decision_log.contains("decision: reject"));
}

#[test]
fn review_invalid_decision_input_prompts_again() {
    let dir = temp_dir("review-invalid-then-accept");
    let input_path = write_input_srt(&dir, "1\n00:00:00,000 --> 00:00:01,000\nI use Kafka");
    let terms_path = write_session_terms(&dir, "Apache Kafka | Kafka");
    let reviewed_path = dir.join("reviewed.srt");
    let log_path = dir.join("decision-log.txt");

    let output = run_with_args_and_stdin(
        &[
            "review",
            input_path.to_str().expect("utf8 input path"),
            terms_path.to_str().expect("utf8 terms path"),
            reviewed_path.to_str().expect("utf8 reviewed path"),
            log_path.to_str().expect("utf8 log path"),
        ],
        "bad\na 0\n",
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    let reviewed_srt = std::fs::read_to_string(&reviewed_path).expect("read reviewed srt");
    assert!(stdout.contains("invalid decision"));
    assert!(reviewed_srt.contains("Apache Kafka"));
}

#[test]
fn review_no_cases_writes_reviewed_srt_and_header_only_decision_log() {
    let dir = temp_dir("review-no-cases");
    let input_path = write_input_srt(&dir, "1\n00:00:00,000 --> 00:00:01,000\nhello");
    let terms_path = write_session_terms(&dir, "Apache Kafka | Kafka");
    let reviewed_path = dir.join("reviewed.srt");
    let log_path = dir.join("decision-log.txt");

    let output = run_with_args_and_stdin(
        &[
            "review",
            input_path.to_str().expect("utf8 input path"),
            terms_path.to_str().expect("utf8 terms path"),
            reviewed_path.to_str().expect("utf8 reviewed path"),
            log_path.to_str().expect("utf8 log path"),
        ],
        "",
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    let reviewed_srt = std::fs::read_to_string(&reviewed_path).expect("read reviewed srt");
    let decision_log = std::fs::read_to_string(&log_path).expect("read decision log");
    assert!(stdout.contains("no review cases found"));
    assert!(reviewed_srt.contains("hello"));
    assert_eq!(decision_log, "voxproof decision log v0\n");
}

#[test]
fn review_invalid_session_terms_fails_before_writing_outputs() {
    let dir = temp_dir("review-invalid-terms");
    let input_path = write_input_srt(&dir, "1\n00:00:00,000 --> 00:00:01,000\nI use Kafka");
    let terms_path = write_session_terms(&dir, "Apache Kafka");
    let reviewed_path = dir.join("reviewed.srt");
    let log_path = dir.join("decision-log.txt");

    let output = run_with_args_and_stdin(
        &[
            "review",
            input_path.to_str().expect("utf8 input path"),
            terms_path.to_str().expect("utf8 terms path"),
            reviewed_path.to_str().expect("utf8 reviewed path"),
            log_path.to_str().expect("utf8 log path"),
        ],
        "",
    );

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    assert!(stderr.contains("invalid session terms at line 1"));
    assert!(!reviewed_path.exists());
    assert!(!log_path.exists());
}

#[test]
fn review_has_no_hard_coded_demo_glossary_fallback() {
    let dir = temp_dir("review-no-demo-fallback");
    let input_path = write_input_srt(&dir, "1\n00:00:00,000 --> 00:00:01,000\nI use Kafka");
    let terms_path = write_session_terms(&dir, "PostgreSQL | Postgres");
    let reviewed_path = dir.join("reviewed.srt");
    let log_path = dir.join("decision-log.txt");

    let output = run_with_args_and_stdin(
        &[
            "review",
            input_path.to_str().expect("utf8 input path"),
            terms_path.to_str().expect("utf8 terms path"),
            reviewed_path.to_str().expect("utf8 reviewed path"),
            log_path.to_str().expect("utf8 log path"),
        ],
        "",
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    let reviewed_srt = std::fs::read_to_string(&reviewed_path).expect("read reviewed srt");
    assert!(stdout.contains("no review cases found"));
    assert!(reviewed_srt.contains("I use Kafka"));
    assert!(!reviewed_srt.contains("Apache Kafka"));
}

#[test]
fn review_wrong_command_shape_prints_usage_and_exits_nonzero() {
    let output = run_with_args_and_stdin(&["review"], "");

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    assert!(stderr.contains("usage:"));
    assert!(stderr.contains(
        "vox-proof review <input.srt> <session-terms.txt> <reviewed-output.srt> <decision-log.txt>"
    ));
}
