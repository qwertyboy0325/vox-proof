use std::ffi::OsStr;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

fn run_with_stdin(input: &str) -> Output {
    run_with_args_and_stdin(&[], input)
}

fn run_with_args_and_stdin(args: &[&str], input: &str) -> Output {
    run_with_args_stdin_and_profiles(args, input, None, None)
}

fn run_with_args_stdin_and_profiles(
    args: &[&str],
    input: &str,
    pinyin_profile: Option<&str>,
    latin_profile: Option<&str>,
) -> Output {
    run_with_args_stdin_and_os_profiles(
        args,
        input,
        pinyin_profile.map(OsStr::new),
        latin_profile.map(OsStr::new),
    )
}

fn run_with_args_stdin_and_os_profiles(
    args: &[&str],
    input: &str,
    pinyin_profile: Option<&OsStr>,
    latin_profile: Option<&OsStr>,
) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_vox-proof"));
    command
        .args(args)
        .env_remove("VOX_PROOF_EXPERIMENT_PINYIN_PROFILE")
        .env_remove("VOX_PROOF_EXPERIMENT_LATIN_SPAN_PROFILE")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(profile) = pinyin_profile {
        command.env("VOX_PROOF_EXPERIMENT_PINYIN_PROFILE", profile);
    }
    if let Some(profile) = latin_profile {
        command.env("VOX_PROOF_EXPERIMENT_LATIN_SPAN_PROFILE", profile);
    }
    let mut child = command.spawn().expect("spawn binary");

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

fn write_description(dir: &std::path::Path, contents: &str) -> PathBuf {
    let path = dir.join("session-description.txt");
    std::fs::write(&path, contents).expect("write session description");
    path
}

#[test]
fn experimental_selection_writes_only_sidecar_marker_and_keeps_exact_output_authority() {
    let dir = temp_dir("experimental-sidecar-marker");
    let input_path = write_input_srt(&dir, "1\n00:00:00,000 --> 00:00:01,000\n卡夫卡");
    let terms_path = write_session_terms(&dir, "Kafka | alias:Kafka");
    let description_path = write_description(&dir, "Synthetic Kafka technical discussion.");
    let report_path = dir.join("experimental-report.json");
    let reviewed_path = dir.join("reviewed.srt");
    let log_path = dir.join("decision-log.txt");
    let summary_path = dir.join("session-summary.txt");

    let output = run_with_args_and_stdin(
        &[
            "review-experiment",
            input_path.to_str().expect("utf8 input path"),
            terms_path.to_str().expect("utf8 terms path"),
            description_path.to_str().expect("utf8 description path"),
            "fake",
            report_path.to_str().expect("utf8 report path"),
            reviewed_path.to_str().expect("utf8 reviewed path"),
            log_path.to_str().expect("utf8 log path"),
            summary_path.to_str().expect("utf8 summary path"),
        ],
        "s experimental-0\n",
    );

    assert!(output.status.success(), "{output:?}");
    let report = std::fs::read_to_string(&report_path).expect("read experimental report");
    let reviewed = std::fs::read_to_string(&reviewed_path).expect("read reviewed output");
    let decision_log = std::fs::read_to_string(&log_path).expect("read decision log");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(report.contains("manual_correction_markers"));
    assert!(report.contains("manual_correction_requested"));
    assert!(report.contains("experimental-0"));
    assert!(report.contains("Experimental only"));
    let report_json: serde_json::Value =
        serde_json::from_str(&report).expect("parse experimental report");
    assert_eq!(
        report_json["schema_revision"],
        "experimental-contextual-resolution-sidecar-v3"
    );
    assert_eq!(
        report_json["pinyin_eligibility_profile"],
        "suppress_short_han_to_short_uppercase_acronym_v1"
    );
    assert_eq!(
        report_json["latin_span_eligibility_profile"],
        "suppress_target_embedded_in_larger_window_v1"
    );
    assert!(reviewed.contains("卡夫卡"));
    assert!(!decision_log.contains("manual_correction"));
    assert!(stdout.contains("reviewed SRT remains unchanged"));
    assert!(stdout.contains("Add `alias:卡夫卡`"));
}

#[test]
fn experimental_unknown_pinyin_profile_fails_without_sidecar() {
    let dir = temp_dir("experimental-unknown-pinyin-profile");
    let input_path = write_input_srt(&dir, "1\n00:00:00,000 --> 00:00:01,000\n卡夫卡");
    let terms_path = write_session_terms(&dir, "Kafka | alias:Kafka");
    let description_path = write_description(&dir, "Synthetic Kafka technical discussion.");
    let report_path = dir.join("experimental-report.json");
    let reviewed_path = dir.join("reviewed.srt");
    let log_path = dir.join("decision-log.txt");
    let summary_path = dir.join("session-summary.txt");

    let output = run_with_args_stdin_and_profiles(
        &[
            "review-experiment",
            input_path.to_str().expect("utf8 input path"),
            terms_path.to_str().expect("utf8 terms path"),
            description_path.to_str().expect("utf8 description path"),
            "rules-only",
            report_path.to_str().expect("utf8 report path"),
            reviewed_path.to_str().expect("utf8 reviewed path"),
            log_path.to_str().expect("utf8 log path"),
            summary_path.to_str().expect("utf8 summary path"),
        ],
        "",
        Some("unknown-profile"),
        None,
    );

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    assert!(stderr.contains("unknown VOX_PROOF_EXPERIMENT_PINYIN_PROFILE 'unknown-profile'"));
    assert!(stderr.contains("unfiltered-baseline-v1"));
    assert!(stderr.contains("suppress-short-han-to-short-uppercase-acronym-v1"));
    assert!(!report_path.exists());
}

#[test]
fn experimental_unknown_latin_profile_fails_without_sidecar() {
    let dir = temp_dir("experimental-unknown-latin-profile");
    let input_path = write_input_srt(&dir, "1\n00:00:00,000 --> 00:00:01,000\nhello");
    let terms_path = write_session_terms(&dir, "Kafka | alias:Kafka");
    let description_path = write_description(&dir, "Synthetic technical discussion.");
    let report_path = dir.join("experimental-report.json");
    let reviewed_path = dir.join("reviewed.srt");
    let log_path = dir.join("decision-log.txt");
    let summary_path = dir.join("session-summary.txt");

    let output = run_with_args_stdin_and_profiles(
        &[
            "review-experiment",
            input_path.to_str().expect("utf8 input path"),
            terms_path.to_str().expect("utf8 terms path"),
            description_path.to_str().expect("utf8 description path"),
            "rules-only",
            report_path.to_str().expect("utf8 report path"),
            reviewed_path.to_str().expect("utf8 reviewed path"),
            log_path.to_str().expect("utf8 log path"),
            summary_path.to_str().expect("utf8 summary path"),
        ],
        "",
        None,
        Some("unknown-profile"),
    );

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    assert!(stderr.contains("unknown VOX_PROOF_EXPERIMENT_LATIN_SPAN_PROFILE 'unknown-profile'"));
    assert!(stderr.contains("unfiltered-baseline-v1"));
    assert!(stderr.contains("suppress-target-embedded-in-larger-window-v1"));
    assert!(!report_path.exists());
}

#[cfg(unix)]
#[test]
fn experimental_non_unicode_latin_profile_fails_without_sidecar() {
    use std::os::unix::ffi::OsStringExt;

    let dir = temp_dir("experimental-non-unicode-latin-profile");
    let input_path = write_input_srt(&dir, "1\n00:00:00,000 --> 00:00:01,000\nhello");
    let terms_path = write_session_terms(&dir, "Kafka | alias:Kafka");
    let description_path = write_description(&dir, "Synthetic technical discussion.");
    let report_path = dir.join("experimental-report.json");
    let reviewed_path = dir.join("reviewed.srt");
    let log_path = dir.join("decision-log.txt");
    let summary_path = dir.join("session-summary.txt");
    let invalid_profile = std::ffi::OsString::from_vec(vec![0xff]);

    let output = run_with_args_stdin_and_os_profiles(
        &[
            "review-experiment",
            input_path.to_str().expect("utf8 input path"),
            terms_path.to_str().expect("utf8 terms path"),
            description_path.to_str().expect("utf8 description path"),
            "rules-only",
            report_path.to_str().expect("utf8 report path"),
            reviewed_path.to_str().expect("utf8 reviewed path"),
            log_path.to_str().expect("utf8 log path"),
            summary_path.to_str().expect("utf8 summary path"),
        ],
        "",
        None,
        Some(invalid_profile.as_os_str()),
    );

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    assert!(
        stderr.contains("VOX_PROOF_EXPERIMENT_LATIN_SPAN_PROFILE contains a non-Unicode value")
    );
    assert!(!report_path.exists());
}

#[test]
fn experimental_explicit_default_latin_profile_is_accepted() {
    let dir = temp_dir("experimental-explicit-default-latin-profile");
    let input_path = write_input_srt(&dir, "1\n00:00:00,000 --> 00:00:01,000\nhello");
    let terms_path = write_session_terms(&dir, "Kafka | alias:Kafka");
    let description_path = write_description(&dir, "Synthetic technical discussion.");
    let report_path = dir.join("experimental-report.json");
    let reviewed_path = dir.join("reviewed.srt");
    let log_path = dir.join("decision-log.txt");
    let summary_path = dir.join("session-summary.txt");

    let output = run_with_args_stdin_and_profiles(
        &[
            "review-experiment",
            input_path.to_str().expect("utf8 input path"),
            terms_path.to_str().expect("utf8 terms path"),
            description_path.to_str().expect("utf8 description path"),
            "rules-only",
            report_path.to_str().expect("utf8 report path"),
            reviewed_path.to_str().expect("utf8 reviewed path"),
            log_path.to_str().expect("utf8 log path"),
            summary_path.to_str().expect("utf8 summary path"),
        ],
        "",
        None,
        Some("suppress-target-embedded-in-larger-window-v1"),
    );

    assert!(output.status.success(), "{output:?}");
    let report: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(report_path).expect("read experimental report"),
    )
    .expect("parse experimental report");
    assert_eq!(
        report["latin_span_eligibility_profile"],
        "suppress_target_embedded_in_larger_window_v1"
    );
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
    let terms_path = write_session_terms(&dir, "Apache Kafka | alias:Kafka");
    let reviewed_path = dir.join("reviewed.srt");
    let log_path = dir.join("decision-log.txt");
    let summary_path = dir.join("session-summary.txt");

    let output = run_with_args_and_stdin(
        &[
            "review",
            input_path.to_str().expect("utf8 input path"),
            terms_path.to_str().expect("utf8 terms path"),
            reviewed_path.to_str().expect("utf8 reviewed path"),
            log_path.to_str().expect("utf8 log path"),
            summary_path.to_str().expect("utf8 summary path"),
        ],
        "a 0\n",
    );

    assert!(output.status.success());
    let reviewed_srt = std::fs::read_to_string(&reviewed_path).expect("read reviewed srt");
    let decision_log = std::fs::read_to_string(&log_path).expect("read decision log");
    let session_summary = std::fs::read_to_string(&summary_path).expect("read session summary");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("loaded 1 session term entries"));
    assert!(stdout.contains("wrote session summary"));
    assert!(reviewed_srt.contains("Apache Kafka"));
    assert!(decision_log.contains("decision: accept_alternative"));
    assert!(decision_log.contains("alternative_index: 0"));
    assert!(session_summary.contains("transcript_segments: 1"));
    assert!(session_summary.contains("session_term_entries: 1"));
    assert!(session_summary.contains("review_cases_raised: 1"));
    assert!(session_summary.contains("accepted_alternatives: 1"));
    assert!(session_summary.contains("accepted_replacements_materialized: 1"));
    assert!(session_summary.contains("glossary-alias-match @ 0.1.0: 1"));
    assert!(session_summary.contains(summary_path.to_str().expect("utf8 summary path")));
}

#[test]
fn review_renders_and_summarizes_alias_and_observed_error_cases_distinctly() {
    let dir = temp_dir("review-alias-and-observed-error");
    let input_path = write_input_srt(
        &dir,
        "1\n00:00:00,000 --> 00:00:01,000\nPostgres then Postgre SQL",
    );
    let terms_path = write_session_terms(&dir, "PostgreSQL | alias:Postgres | error:Postgre SQL");
    let reviewed_path = dir.join("reviewed.srt");
    let log_path = dir.join("decision-log.txt");
    let summary_path = dir.join("session-summary.txt");

    let output = run_with_args_and_stdin(
        &[
            "review",
            input_path.to_str().expect("utf8 input path"),
            terms_path.to_str().expect("utf8 terms path"),
            reviewed_path.to_str().expect("utf8 reviewed path"),
            log_path.to_str().expect("utf8 log path"),
            summary_path.to_str().expect("utf8 summary path"),
        ],
        "r\na 0\n",
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    let reviewed_srt = std::fs::read_to_string(&reviewed_path).expect("read reviewed srt");
    let decision_log = std::fs::read_to_string(&log_path).expect("read decision log");
    let session_summary = std::fs::read_to_string(&summary_path).expect("read session summary");

    assert!(stdout.contains("evidence: glossary alias 'Postgres' for 'PostgreSQL'"));
    assert!(stdout.contains("evidence: observed error form 'Postgre SQL' for 'PostgreSQL'"));
    assert!(reviewed_srt.contains("Postgres then PostgreSQL"));
    assert!(decision_log.contains("decision: reject"));
    assert!(decision_log.contains("decision: accept_alternative"));
    assert!(session_summary.contains("glossary_alias_match: 2"));
    assert!(session_summary.contains("glossary-alias-match @ 0.1.0: 1"));
    assert!(session_summary.contains("observed-error-form-match @ 0.1.0: 1"));
}

#[test]
fn rejecting_observed_error_form_leaves_source_text_unchanged() {
    let dir = temp_dir("review-reject-observed-error");
    let input_path = write_input_srt(&dir, "1\n00:00:00,000 --> 00:00:01,000\nPostgre SQL");
    let terms_path = write_session_terms(&dir, "PostgreSQL | error:Postgre SQL");
    let reviewed_path = dir.join("reviewed.srt");
    let log_path = dir.join("decision-log.txt");
    let summary_path = dir.join("session-summary.txt");

    let output = run_with_args_and_stdin(
        &[
            "review",
            input_path.to_str().expect("utf8 input path"),
            terms_path.to_str().expect("utf8 terms path"),
            reviewed_path.to_str().expect("utf8 reviewed path"),
            log_path.to_str().expect("utf8 log path"),
            summary_path.to_str().expect("utf8 summary path"),
        ],
        "r\n",
    );

    assert!(output.status.success());
    let reviewed_srt = std::fs::read_to_string(&reviewed_path).expect("read reviewed srt");
    assert!(reviewed_srt.contains("Postgre SQL"));
    assert!(!reviewed_srt.contains("\nPostgreSQL\n"));
}

#[test]
fn review_reject_writes_unchanged_reviewed_srt_and_decision_log() {
    let dir = temp_dir("review-reject");
    let input_path = write_input_srt(&dir, "1\n00:00:00,000 --> 00:00:01,000\nI use Kafka");
    let terms_path = write_session_terms(&dir, "Apache Kafka | alias:Kafka");
    let reviewed_path = dir.join("reviewed.srt");
    let log_path = dir.join("decision-log.txt");
    let summary_path = dir.join("session-summary.txt");

    let output = run_with_args_and_stdin(
        &[
            "review",
            input_path.to_str().expect("utf8 input path"),
            terms_path.to_str().expect("utf8 terms path"),
            reviewed_path.to_str().expect("utf8 reviewed path"),
            log_path.to_str().expect("utf8 log path"),
            summary_path.to_str().expect("utf8 summary path"),
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
    let terms_path = write_session_terms(&dir, "Apache Kafka | alias:Kafka");
    let reviewed_path = dir.join("reviewed.srt");
    let log_path = dir.join("decision-log.txt");
    let summary_path = dir.join("session-summary.txt");

    let output = run_with_args_and_stdin(
        &[
            "review",
            input_path.to_str().expect("utf8 input path"),
            terms_path.to_str().expect("utf8 terms path"),
            reviewed_path.to_str().expect("utf8 reviewed path"),
            log_path.to_str().expect("utf8 log path"),
            summary_path.to_str().expect("utf8 summary path"),
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
    let terms_path = write_session_terms(&dir, "Apache Kafka | alias:Kafka");
    let reviewed_path = dir.join("reviewed.srt");
    let log_path = dir.join("decision-log.txt");
    let summary_path = dir.join("session-summary.txt");

    let output = run_with_args_and_stdin(
        &[
            "review",
            input_path.to_str().expect("utf8 input path"),
            terms_path.to_str().expect("utf8 terms path"),
            reviewed_path.to_str().expect("utf8 reviewed path"),
            log_path.to_str().expect("utf8 log path"),
            summary_path.to_str().expect("utf8 summary path"),
        ],
        "",
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    let reviewed_srt = std::fs::read_to_string(&reviewed_path).expect("read reviewed srt");
    let decision_log = std::fs::read_to_string(&log_path).expect("read decision log");
    let session_summary = std::fs::read_to_string(&summary_path).expect("read session summary");
    assert!(stdout.contains("no review cases found"));
    assert!(reviewed_srt.contains("hello"));
    assert_eq!(decision_log, "voxproof decision log v0\n");
    assert!(session_summary.contains("review_cases_raised: 0"));
    assert!(session_summary.contains("total_decisions_recorded: 0"));
}

#[test]
fn review_invalid_session_terms_fails_before_writing_outputs() {
    let dir = temp_dir("review-invalid-terms");
    let input_path = write_input_srt(&dir, "1\n00:00:00,000 --> 00:00:01,000\nI use Kafka");
    let terms_path = write_session_terms(&dir, "Apache Kafka");
    let reviewed_path = dir.join("reviewed.srt");
    let log_path = dir.join("decision-log.txt");
    let summary_path = dir.join("session-summary.txt");

    let output = run_with_args_and_stdin(
        &[
            "review",
            input_path.to_str().expect("utf8 input path"),
            terms_path.to_str().expect("utf8 terms path"),
            reviewed_path.to_str().expect("utf8 reviewed path"),
            log_path.to_str().expect("utf8 log path"),
            summary_path.to_str().expect("utf8 summary path"),
        ],
        "",
    );

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    assert!(stderr.contains("invalid session terms at line 1"));
    assert!(!reviewed_path.exists());
    assert!(!log_path.exists());
    assert!(!summary_path.exists());
}

#[test]
fn conflicting_alias_and_observed_error_fails_before_writing_outputs() {
    let dir = temp_dir("review-conflicting-source-forms");
    let input_path = write_input_srt(&dir, "1\n00:00:00,000 --> 00:00:01,000\nKafka");
    let terms_path = write_session_terms(
        &dir,
        "Apache Kafka | alias:Kafka\nOther Kafka | error:Kafka",
    );
    let reviewed_path = dir.join("reviewed.srt");
    let log_path = dir.join("decision-log.txt");
    let summary_path = dir.join("session-summary.txt");

    let output = run_with_args_and_stdin(
        &[
            "review",
            input_path.to_str().expect("utf8 input path"),
            terms_path.to_str().expect("utf8 terms path"),
            reviewed_path.to_str().expect("utf8 reviewed path"),
            log_path.to_str().expect("utf8 log path"),
            summary_path.to_str().expect("utf8 summary path"),
        ],
        "",
    );

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    assert!(stderr.contains("both an alias and observed error form"));
    assert!(stderr.contains("lines 1 and 2"));
    assert!(!reviewed_path.exists());
    assert!(!log_path.exists());
    assert!(!summary_path.exists());
}

#[test]
fn review_has_no_hard_coded_demo_glossary_fallback() {
    let dir = temp_dir("review-no-demo-fallback");
    let input_path = write_input_srt(&dir, "1\n00:00:00,000 --> 00:00:01,000\nI use Kafka");
    let terms_path = write_session_terms(&dir, "PostgreSQL | alias:Postgres");
    let reviewed_path = dir.join("reviewed.srt");
    let log_path = dir.join("decision-log.txt");
    let summary_path = dir.join("session-summary.txt");

    let output = run_with_args_and_stdin(
        &[
            "review",
            input_path.to_str().expect("utf8 input path"),
            terms_path.to_str().expect("utf8 terms path"),
            reviewed_path.to_str().expect("utf8 reviewed path"),
            log_path.to_str().expect("utf8 log path"),
            summary_path.to_str().expect("utf8 summary path"),
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
fn review_summary_write_failure_reports_incomplete_output_without_success_claim() {
    let dir = temp_dir("review-summary-write-failure");
    let input_path = write_input_srt(&dir, "1\n00:00:00,000 --> 00:00:01,000\nI use Kafka");
    let terms_path = write_session_terms(&dir, "Apache Kafka | alias:Kafka");
    let reviewed_path = dir.join("reviewed.srt");
    let log_path = dir.join("decision-log.txt");
    let summary_path = dir.join("summary-target");
    std::fs::create_dir(&summary_path).expect("create directory at summary output path");

    let output = run_with_args_and_stdin(
        &[
            "review",
            input_path.to_str().expect("utf8 input path"),
            terms_path.to_str().expect("utf8 terms path"),
            reviewed_path.to_str().expect("utf8 reviewed path"),
            log_path.to_str().expect("utf8 log path"),
            summary_path.to_str().expect("utf8 summary path"),
        ],
        "r\n",
    );

    assert!(!output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    assert!(!stdout.contains("wrote reviewed SRT"));
    assert!(!stdout.contains("wrote decision log"));
    assert!(!stdout.contains("wrote session summary"));
    assert!(stderr.contains("failed to write session summary"));
    assert!(stderr.contains("session output is incomplete"));
    assert!(reviewed_path.exists());
    assert!(log_path.exists());
}

#[test]
fn review_wrong_command_shape_prints_usage_and_exits_nonzero() {
    let output = run_with_args_and_stdin(&["review"], "");

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    assert!(stderr.contains("usage:"));
    assert!(stderr.contains(
        "vox-proof review <input.srt> <session-terms.txt> <reviewed-output.srt> <decision-log.txt> <session-summary.txt>"
    ));
}
