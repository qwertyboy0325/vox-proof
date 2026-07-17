use std::ffi::OsStr;
use std::fs::{File, OpenOptions};
use std::io::{Seek, SeekFrom, Write};
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

fn run_with_seekable_stdin(args: &[&str], stdin_path: &std::path::Path, payload: &[u8]) -> Output {
    {
        let mut writer = File::create(stdin_path).expect("create stdin file");
        writer.write_all(payload).expect("write stdin payload");
        writer.flush().expect("flush stdin payload");
    }

    let mut position_probe = OpenOptions::new()
        .read(true)
        .write(true)
        .open(stdin_path)
        .expect("open stdin for shared offset probe");
    position_probe
        .seek(SeekFrom::Start(0))
        .expect("rewind shared stdin offset");

    let stdin_for_child = position_probe
        .try_clone()
        .expect("clone stdin handle for child");

    let mut command = Command::new(env!("CARGO_BIN_EXE_vox-proof"));
    command
        .args(args)
        .env_remove("VOX_PROOF_EXPERIMENT_PINYIN_PROFILE")
        .env_remove("VOX_PROOF_EXPERIMENT_LATIN_SPAN_PROFILE")
        .stdin(stdin_for_child)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let output = command
        .spawn()
        .expect("spawn binary")
        .wait_with_output()
        .expect("wait for binary output");

    assert_eq!(
        position_probe.stream_position().expect("stdin offset"),
        0,
        "evaluate must not read seekable stdin"
    );
    assert_eq!(
        std::fs::read(stdin_path).expect("read stdin payload"),
        payload,
        "stdin payload must remain byte-identical"
    );

    output
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
    write_named_input_srt(dir, "input.srt", contents)
}

fn write_named_input_srt(dir: &std::path::Path, filename: &str, contents: &str) -> PathBuf {
    let path = dir.join(filename);
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
        "r\nr\na 0\n",
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
fn review_renders_and_summarizes_phonetic_similarity_case() {
    let dir = temp_dir("review-phonetic-similarity");
    let input_path = write_input_srt(&dir, "1\n00:00:00,000 --> 00:00:01,000\nPostgre sequel");
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
        "r\n",
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    let session_summary = std::fs::read_to_string(&summary_path).expect("read session summary");

    assert!(stdout.contains("evidence: phonetic similarity 'Postgre sequel' -> 'PostgreSQL'"));
    assert!(stdout.contains("phonetic_source: normalized='postgresequel'"));
    assert!(stdout.contains("phonetic_target: normalized='postgresql'"));
    assert!(stdout.contains("phonetic_score: distance="));
    assert!(stdout.contains("matched_key='PSTKRSKL'"));
    assert!(stdout.contains(
        "phonetic_identity: config=canonical-session-term-cue-local/0.2.0 algorithm=canonical-exact-plus-ascii-double-metaphone-levenshtein/rphonetic-3.0.6-v1"
    ));
    assert!(session_summary.contains("phonetic_similarity: 1"));
    assert!(session_summary.contains("ascii-latin-phonetic-similarity @ 0.1.0: 1"));
}

#[test]
fn review_rejects_canonical_only_phonetic_case_without_changing_srt() {
    let dir = temp_dir("review-canonical-only-reject");
    let input_srt = "1\n00:00:00,000 --> 00:00:01,000\nASIS\n";
    let input_path = write_input_srt(&dir, input_srt);
    let terms_path = write_session_terms(&dir, "ASUS\n");
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

    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    let reviewed_srt = std::fs::read_to_string(&reviewed_path).expect("read reviewed srt");
    let decision_log = std::fs::read_to_string(&log_path).expect("read decision log");
    let session_summary = std::fs::read_to_string(&summary_path).expect("read session summary");

    assert!(stdout.contains("loaded 1 session term entries"));
    assert!(stdout.contains("evidence: phonetic similarity 'ASIS' -> 'ASUS' (canonical_term)"));
    assert!(stdout.contains("distance=1 ratio=3/4 permille=750 matched_key='ASS'"));
    assert_eq!(reviewed_srt, input_srt);
    assert!(decision_log.contains("decision: reject"));
    assert!(session_summary.contains("review_cases_raised: 1"));
    assert!(session_summary.contains("rejected: 1"));
    assert!(session_summary.contains("accepted_replacements_materialized: 0"));
}

#[test]
fn review_accepts_canonical_only_phonetic_case_and_materializes_target() {
    let dir = temp_dir("review-canonical-only-accept");
    let input_path = write_input_srt(&dir, "1\n00:00:00,000 --> 00:00:01,000\nASIS\n");
    let terms_path = write_session_terms(&dir, "ASUS\n");
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

    assert!(output.status.success(), "{output:?}");
    let reviewed_srt = std::fs::read_to_string(&reviewed_path).expect("read reviewed srt");
    let decision_log = std::fs::read_to_string(&log_path).expect("read decision log");
    let session_summary = std::fs::read_to_string(&summary_path).expect("read session summary");

    assert_eq!(reviewed_srt, "1\n00:00:00,000 --> 00:00:01,000\nASUS\n");
    assert!(decision_log.contains("decision: accept_alternative"));
    assert!(decision_log.contains("alternative_index: 0"));
    assert!(session_summary.contains("phonetic_similarity: 1"));
    assert!(session_summary.contains("accepted_alternatives: 1"));
    assert!(session_summary.contains("accepted_replacements_materialized: 1"));
    assert!(session_summary.contains("source_segments_affected: 1"));
}

#[test]
fn review_accepts_mixed_canonical_only_alias_and_error_form_file() {
    let dir = temp_dir("review-mixed-session-term-kinds");
    let input_path = write_input_srt(
        &dir,
        "1\n00:00:00,000 --> 00:00:01,000\nASUS\n\n2\n00:00:01,000 --> 00:00:02,000\nKafka\n\n3\n00:00:02,000 --> 00:00:03,000\npost gray sequel\n",
    );
    let terms_path = write_session_terms(
        &dir,
        "ASUS\nApache Kafka | alias:Kafka\nPostgreSQL | error:post gray sequel\n",
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
        &"r\n".repeat(20),
    );

    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    let reviewed_srt = std::fs::read_to_string(&reviewed_path).expect("read reviewed srt");

    assert!(stdout.contains("loaded 3 session term entries"));
    assert!(stdout.contains("evidence: glossary alias 'Kafka' for 'Apache Kafka'"));
    assert!(stdout.contains("evidence: observed error form 'post gray sequel' for 'PostgreSQL'"));
    assert!(reviewed_srt.contains("\nASUS\n"));
    assert!(reviewed_srt.contains("\nKafka\n"));
    assert!(reviewed_srt.contains("\npost gray sequel\n"));
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
        "r\nr\n",
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
    let terms_path = write_session_terms(&dir, "Apache Kafka | alias:");
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

#[test]
fn review_prompt_shows_adjacent_source_cue_context_for_middle_cue() {
    let dir = temp_dir("review-nearby-context-middle");
    let input_path = write_input_srt(
        &dir,
        "1\n00:00:00,000 --> 00:00:01,000\nfirst cue\n\n2\n00:00:01,000 --> 00:00:02,000\nmiddle Kafka cue\n\n3\n00:00:02,000 --> 00:00:03,000\nlast cue",
    );
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
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains(
        "nearby_context_note: presentation only; not evidence, not ranking input, and not used for materialization"
    ));
    assert!(stdout.contains("previous_cue_index: 1"));
    assert!(stdout.contains("previous_cue_text: first cue"));
    assert!(stdout.contains("cue_index: 2"));
    assert!(stdout.contains("cue_text: middle Kafka cue"));
    assert!(!stdout.contains("review_cue_"));
    assert!(stdout.contains("following_cue_index: 3"));
    assert!(stdout.contains("following_cue_text: last cue"));
}

#[test]
fn review_prompt_omits_missing_previous_context_for_first_cue() {
    let dir = temp_dir("review-nearby-context-first");
    let input_path = write_input_srt(
        &dir,
        "1\n00:00:00,000 --> 00:00:01,000\nfirst Kafka cue\n\n2\n00:00:01,000 --> 00:00:02,000\nlast cue",
    );
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
        "r\nr\n",
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("previous_cue_index: (none)"));
    assert!(stdout.contains("previous_cue_text: (none)"));
    assert!(stdout.contains("cue_index: 1"));
    assert!(stdout.contains("cue_text: first Kafka cue"));
    assert!(!stdout.contains("review_cue_"));
    assert!(stdout.contains("following_cue_index: 2"));
    assert!(stdout.contains("following_cue_text: last cue"));
}

#[test]
fn review_prompt_omits_missing_following_context_for_last_cue() {
    let dir = temp_dir("review-nearby-context-last");
    let input_path = write_input_srt(
        &dir,
        "1\n00:00:00,000 --> 00:00:01,000\nfirst cue\n\n2\n00:00:01,000 --> 00:00:02,000\nlast Kafka cue",
    );
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
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("previous_cue_index: 1"));
    assert!(stdout.contains("previous_cue_text: first cue"));
    assert!(stdout.contains("cue_index: 2"));
    assert!(stdout.contains("cue_text: last Kafka cue"));
    assert!(!stdout.contains("review_cue_"));
    assert!(stdout.contains("following_cue_index: (none)"));
    assert!(stdout.contains("following_cue_text: (none)"));
}

#[test]
fn review_prompt_renders_unicode_and_multiline_nearby_context() {
    let dir = temp_dir("review-nearby-context-unicode-multiline");
    let input_path = write_input_srt(
        &dir,
        "1\n00:00:00,000 --> 00:00:01,000\n前段\n\n2\n00:00:01,000 --> 00:00:02,000\n中間\nKafka\n段\n\n3\n00:00:02,000 --> 00:00:03,000\n後段",
    );
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
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("previous_cue_text: 前段"));
    assert!(stdout.contains("cue_text: 中間\nKafka\n段"));
    assert!(!stdout.contains("review_cue_"));
    assert!(stdout.contains("following_cue_text: 後段"));
}

#[test]
fn review_nearby_context_does_not_change_accept_or_reject_outputs() {
    let dir = temp_dir("review-nearby-context-output-unchanged");
    let input_path = write_input_srt(
        &dir,
        "1\n00:00:00,000 --> 00:00:01,000\nbefore\n\n2\n00:00:01,000 --> 00:00:02,000\nI use Kafka\n\n3\n00:00:02,000 --> 00:00:03,000\nafter",
    );
    let terms_path = write_session_terms(&dir, "Apache Kafka | alias:Kafka");
    let reviewed_path = dir.join("reviewed.srt");
    let log_path = dir.join("decision-log.txt");
    let summary_path = dir.join("session-summary.txt");

    let accept_output = run_with_args_and_stdin(
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

    assert!(accept_output.status.success());
    let accept_stdout = String::from_utf8(accept_output.stdout).expect("utf8 stdout");
    let accept_reviewed = std::fs::read_to_string(&reviewed_path).expect("read reviewed srt");
    let accept_log = std::fs::read_to_string(&log_path).expect("read decision log");
    let accept_summary = std::fs::read_to_string(&summary_path).expect("read session summary");
    assert!(accept_stdout.contains("nearby_context_note:"));
    assert!(accept_stdout.contains("cue_index:"));
    assert!(accept_stdout.contains("cue_text:"));
    assert!(!accept_stdout.contains("review_cue_"));
    assert!(accept_reviewed.contains("I use Apache Kafka"));
    assert!(accept_log.contains("decision: accept_alternative"));
    assert!(accept_summary.contains("accepted_alternatives: 1"));

    let reject_output = run_with_args_and_stdin(
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

    assert!(reject_output.status.success());
    let reject_stdout = String::from_utf8(reject_output.stdout).expect("utf8 stdout");
    let reject_reviewed = std::fs::read_to_string(&reviewed_path).expect("read reviewed srt");
    let reject_log = std::fs::read_to_string(&log_path).expect("read decision log");
    let reject_summary = std::fs::read_to_string(&summary_path).expect("read session summary");
    assert!(reject_stdout.contains("nearby_context_note:"));
    assert!(reject_stdout.contains("cue_index:"));
    assert!(reject_stdout.contains("cue_text:"));
    assert!(!reject_stdout.contains("review_cue_"));
    assert!(reject_reviewed.contains("I use Kafka"));
    assert!(!reject_reviewed.contains("Apache Kafka"));
    assert!(reject_log.contains("decision: reject"));
    assert!(reject_summary.contains("rejected: 1"));
}

fn run_compare(args: &[&str]) -> Output {
    run_with_args_and_stdin(args, "")
}

#[test]
fn compare_writes_strict_skeleton_aligned_report() {
    let dir = temp_dir("compare-success");
    let raw_path = write_named_input_srt(
        &dir,
        "raw.srt",
        "1\n00:00:00,000 --> 00:00:01,000\nfirst\n\n2\n00:00:01,000 --> 00:00:02,000\nKafka\n\n3\n00:00:02,000 --> 00:00:03,000\nlast",
    );
    let final_path = write_named_input_srt(
        &dir,
        "final.srt",
        "1\n00:00:00,000 --> 00:00:01,000\nfirst\n\n2\n00:00:01,000 --> 00:00:02,000\nApache Kafka\n\n3\n00:00:02,000 --> 00:00:03,000\nlast",
    );
    let report_path = dir.join("comparison-report.json");

    let output = run_compare(&[
        "compare",
        raw_path.to_str().expect("utf8 raw path"),
        final_path.to_str().expect("utf8 final path"),
        report_path.to_str().expect("utf8 report path"),
    ]);

    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    let report = std::fs::read_to_string(&report_path).expect("read report");
    assert!(stdout.contains("wrote comparison report:"));
    assert!(report.contains("\"schema_revision\": \"voxproof-calibration-comparison-v0\""));
    assert!(
        report.contains("\"compatibility_policy_id\": \"identical-cue-count-index-and-timing-v0\"")
    );
    assert!(report.contains("\"cue_count\": 3"));
    assert!(report.contains("\"unchanged_count\": 2"));
    assert!(report.contains("\"text_changed_count\": 1"));
    assert!(report.contains("\"change_kind\": \"text_changed\""));
    assert!(report.contains("\"raw_text\": \"Kafka\""));
    assert!(report.contains("\"final_text\": \"Apache Kafka\""));
    assert!(report.ends_with('\n'));
}

#[test]
fn compare_rejects_malformed_raw_srt() {
    let dir = temp_dir("compare-malformed-raw");
    let raw_path = write_named_input_srt(&dir, "raw.srt", "not srt");
    let final_path =
        write_named_input_srt(&dir, "final.srt", "1\n00:00:00,000 --> 00:00:01,000\none");
    let report_path = dir.join("comparison-report.json");

    let output = run_compare(&[
        "compare",
        raw_path.to_str().expect("utf8 raw path"),
        final_path.to_str().expect("utf8 final path"),
        report_path.to_str().expect("utf8 report path"),
    ]);

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    assert!(stderr.contains("failed to parse raw SRT"));
    assert!(!report_path.exists());
}

#[test]
fn compare_rejects_malformed_final_srt() {
    let dir = temp_dir("compare-malformed-final");
    let raw_path = write_named_input_srt(&dir, "raw.srt", "1\n00:00:00,000 --> 00:00:01,000\none");
    let final_path = write_named_input_srt(&dir, "final.srt", "not srt");
    let report_path = dir.join("comparison-report.json");

    let output = run_compare(&[
        "compare",
        raw_path.to_str().expect("utf8 raw path"),
        final_path.to_str().expect("utf8 final path"),
        report_path.to_str().expect("utf8 report path"),
    ]);

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    assert!(stderr.contains("failed to parse final SRT"));
    assert!(!report_path.exists());
}

#[test]
fn compare_rejects_raw_validation_issue() {
    let dir = temp_dir("compare-raw-validation");
    let raw_path = write_named_input_srt(
        &dir,
        "raw.srt",
        "1\n00:00:03,000 --> 00:00:02,500\nreversed",
    );
    let final_path =
        write_named_input_srt(&dir, "final.srt", "1\n00:00:00,000 --> 00:00:01,000\none");
    let report_path = dir.join("comparison-report.json");

    let output = run_compare(&[
        "compare",
        raw_path.to_str().expect("utf8 raw path"),
        final_path.to_str().expect("utf8 final path"),
        report_path.to_str().expect("utf8 report path"),
    ]);

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    assert!(stderr.contains("raw SRT has validation issues; comparison refused"));
    assert!(stderr.contains("segment position 0 (cue index 1):"));
    assert!(stderr.contains("EndBeforeStart { start_ms: 3000, end_ms: 2500 }"));
    assert!(!report_path.exists());
}

#[test]
fn compare_rejects_final_validation_issue() {
    let dir = temp_dir("compare-final-validation");
    let raw_path = write_named_input_srt(&dir, "raw.srt", "1\n00:00:00,000 --> 00:00:01,000\none");
    let final_path = write_named_input_srt(
        &dir,
        "final.srt",
        "1\n00:00:03,000 --> 00:00:02,500\nreversed",
    );
    let report_path = dir.join("comparison-report.json");

    let output = run_compare(&[
        "compare",
        raw_path.to_str().expect("utf8 raw path"),
        final_path.to_str().expect("utf8 final path"),
        report_path.to_str().expect("utf8 report path"),
    ]);

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    assert!(stderr.contains("final SRT has validation issues; comparison refused"));
    assert!(stderr.contains("segment position 0 (cue index 1):"));
    assert!(stderr.contains("EndBeforeStart { start_ms: 3000, end_ms: 2500 }"));
    assert!(!report_path.exists());
}

#[test]
fn compare_rejects_unreadable_raw_input() {
    let dir = temp_dir("compare-unreadable-raw");
    let final_path =
        write_named_input_srt(&dir, "final.srt", "1\n00:00:00,000 --> 00:00:01,000\none");
    let report_path = dir.join("comparison-report.json");
    let missing_raw = dir.join("missing-raw.srt");

    let output = run_compare(&[
        "compare",
        missing_raw.to_str().expect("utf8 raw path"),
        final_path.to_str().expect("utf8 final path"),
        report_path.to_str().expect("utf8 report path"),
    ]);

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    assert!(stderr.contains("failed to read raw input"));
    assert!(!report_path.exists());
}

#[test]
fn compare_rejects_unreadable_final_input() {
    let dir = temp_dir("compare-unreadable-final");
    let raw_path = write_named_input_srt(&dir, "raw.srt", "1\n00:00:00,000 --> 00:00:01,000\none");
    let report_path = dir.join("comparison-report.json");
    let missing_final = dir.join("missing-final.srt");

    let output = run_compare(&[
        "compare",
        raw_path.to_str().expect("utf8 raw path"),
        missing_final.to_str().expect("utf8 final path"),
        report_path.to_str().expect("utf8 report path"),
    ]);

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    assert!(stderr.contains("failed to read final input"));
    assert!(!report_path.exists());
}

#[test]
fn compare_rejects_cue_count_mismatch_without_report() {
    let dir = temp_dir("compare-count-mismatch");
    let raw_path = write_named_input_srt(
        &dir,
        "raw.srt",
        "1\n00:00:00,000 --> 00:00:01,000\none\n\n2\n00:00:01,000 --> 00:00:02,000\ntwo",
    );
    let final_path =
        write_named_input_srt(&dir, "final.srt", "1\n00:00:00,000 --> 00:00:01,000\none");
    let report_path = dir.join("comparison-report.json");

    let output = run_compare(&[
        "compare",
        raw_path.to_str().expect("utf8 raw path"),
        final_path.to_str().expect("utf8 final path"),
        report_path.to_str().expect("utf8 report path"),
    ]);

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    assert!(stderr.contains("comparison refused: cue count mismatch"));
    assert!(!report_path.exists());
}

#[test]
fn compare_rejects_index_mismatch_without_report() {
    let dir = temp_dir("compare-index-mismatch");
    let raw_path = write_named_input_srt(&dir, "raw.srt", "1\n00:00:00,000 --> 00:00:01,000\none");
    let final_path =
        write_named_input_srt(&dir, "final.srt", "2\n00:00:00,000 --> 00:00:01,000\none");
    let report_path = dir.join("comparison-report.json");

    let output = run_compare(&[
        "compare",
        raw_path.to_str().expect("utf8 raw path"),
        final_path.to_str().expect("utf8 final path"),
        report_path.to_str().expect("utf8 report path"),
    ]);

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    assert!(stderr.contains("comparison refused: cue index mismatch"));
    assert!(!report_path.exists());
}

#[test]
fn compare_rejects_timing_mismatch_without_report() {
    let dir = temp_dir("compare-timing-mismatch");
    let raw_path = write_named_input_srt(&dir, "raw.srt", "1\n00:00:00,000 --> 00:00:01,000\none");
    let final_path =
        write_named_input_srt(&dir, "final.srt", "1\n00:00:00,000 --> 00:00:01,100\none");
    let report_path = dir.join("comparison-report.json");

    let output = run_compare(&[
        "compare",
        raw_path.to_str().expect("utf8 raw path"),
        final_path.to_str().expect("utf8 final path"),
        report_path.to_str().expect("utf8 report path"),
    ]);

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    assert!(stderr.contains("comparison refused: end timing mismatch"));
    assert!(!report_path.exists());
}

#[test]
fn compare_refuses_existing_destination_and_preserves_bytes() {
    let dir = temp_dir("compare-existing-destination");
    let raw_path = write_named_input_srt(&dir, "raw.srt", "1\n00:00:00,000 --> 00:00:01,000\none");
    let final_path =
        write_named_input_srt(&dir, "final.srt", "1\n00:00:00,000 --> 00:00:01,000\none");
    let report_path = dir.join("comparison-report.json");
    let existing = b"{\"existing\":true}\n";
    std::fs::write(&report_path, existing).expect("seed report");

    let output = run_compare(&[
        "compare",
        raw_path.to_str().expect("utf8 raw path"),
        final_path.to_str().expect("utf8 final path"),
        report_path.to_str().expect("utf8 report path"),
    ]);

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    assert!(stderr.contains("destination already exists"));
    assert_eq!(
        std::fs::read(&report_path).expect("read preserved report"),
        existing
    );
}

#[test]
fn compare_refuses_output_path_equal_to_raw_input_and_preserves_raw_bytes() {
    let dir = temp_dir("compare-output-equals-raw");
    let raw_path = write_named_input_srt(&dir, "raw.srt", "1\n00:00:00,000 --> 00:00:01,000\none");
    let final_path =
        write_named_input_srt(&dir, "final.srt", "1\n00:00:00,000 --> 00:00:01,000\none");
    let raw_bytes = std::fs::read(&raw_path).expect("read raw bytes");

    let output = run_compare(&[
        "compare",
        raw_path.to_str().expect("utf8 raw path"),
        final_path.to_str().expect("utf8 final path"),
        raw_path.to_str().expect("utf8 report path"),
    ]);

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    assert!(stderr.contains("destination already exists"));
    assert_eq!(std::fs::read(&raw_path).expect("read raw bytes"), raw_bytes);
}

#[test]
fn compare_wrong_arity_prints_usage_and_exits_nonzero() {
    let output = run_compare(&["compare", "raw.srt", "final.srt"]);

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    assert!(stderr.contains("usage:"));
    assert!(
        stderr.contains(
            "vox-proof compare <raw-input.srt> <final-input.srt> <comparison-report.json>"
        )
    );
}

fn run_evaluate(args: &[&str]) -> Output {
    run_with_args_and_stdin(args, "")
}

#[test]
fn evaluate_writes_calibration_correspondence_report() {
    let dir = temp_dir("evaluate-success");
    let raw_path = write_named_input_srt(
        &dir,
        "raw.srt",
        "1\n00:00:00,000 --> 00:00:01,000\nsame\n\n2\n00:00:01,000 --> 00:00:02,000\nASIS\n\n3\n00:00:02,000 --> 00:00:03,000\nlast",
    );
    let final_path = write_named_input_srt(
        &dir,
        "final.srt",
        "1\n00:00:00,000 --> 00:00:01,000\nsame\n\n2\n00:00:01,000 --> 00:00:02,000\nASUS\n\n3\n00:00:02,000 --> 00:00:03,000\nlast",
    );
    let terms_path = write_session_terms(&dir, "ASUS\n");
    let report_path = dir.join("evaluation-report.json");

    let output = run_evaluate(&[
        "evaluate",
        raw_path.to_str().expect("utf8 raw path"),
        final_path.to_str().expect("utf8 final path"),
        terms_path.to_str().expect("utf8 terms path"),
        report_path.to_str().expect("utf8 report path"),
    ]);

    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    let report = std::fs::read_to_string(&report_path).expect("read report");
    assert!(stdout.contains("wrote calibration evaluation report:"));
    assert!(report.contains("\"schema_revision\": \"voxproof-calibration-correspondence-v0\""));
    assert!(report.contains("\"max_lcs_cells\": 4000000"));
    assert!(report.contains("\"review_case_count\": 1"));
    assert!(report.contains("\"changed_cue_count\": 1"));
    assert!(report.contains("\"unchanged_cue_count\": 2"));
}

#[test]
fn evaluate_zero_case_report_still_inventories_edits() {
    let dir = temp_dir("evaluate-zero-cases");
    let raw_path = write_named_input_srt(&dir, "raw.srt", "1\n00:00:00,000 --> 00:00:01,000\nxxx");
    let final_path =
        write_named_input_srt(&dir, "final.srt", "1\n00:00:00,000 --> 00:00:01,000\nyyy");
    let terms_path = write_session_terms(&dir, "Kafka\n");
    let report_path = dir.join("evaluation-report.json");

    let output = run_evaluate(&[
        "evaluate",
        raw_path.to_str().expect("utf8 raw path"),
        final_path.to_str().expect("utf8 final path"),
        terms_path.to_str().expect("utf8 terms path"),
        report_path.to_str().expect("utf8 report path"),
    ]);

    assert!(output.status.success(), "{output:?}");
    let report = std::fs::read_to_string(&report_path).expect("read report");
    assert!(report.contains("\"review_case_count\": 0"));
    assert!(report.contains("\"local_edit_count\": 1"));
}

#[test]
fn evaluate_rejects_malformed_raw_srt() {
    let dir = temp_dir("evaluate-malformed-raw");
    let raw_path = write_named_input_srt(&dir, "raw.srt", "1\nbad timing\nhello");
    let final_path =
        write_named_input_srt(&dir, "final.srt", "1\n00:00:00,000 --> 00:00:01,000\nhello");
    let terms_path = write_session_terms(&dir, "Kafka\n");
    let report_path = dir.join("evaluation-report.json");

    let output = run_evaluate(&[
        "evaluate",
        raw_path.to_str().expect("utf8 raw path"),
        final_path.to_str().expect("utf8 final path"),
        terms_path.to_str().expect("utf8 terms path"),
        report_path.to_str().expect("utf8 report path"),
    ]);

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    assert!(stderr.contains("failed to parse raw SRT"));
    assert!(!report_path.exists());
}

#[test]
fn evaluate_rejects_cue_count_mismatch_without_report() {
    let dir = temp_dir("evaluate-count-mismatch");
    let raw_path = write_named_input_srt(
        &dir,
        "raw.srt",
        "1\n00:00:00,000 --> 00:00:01,000\none\n\n2\n00:00:01,000 --> 00:00:02,000\ntwo",
    );
    let final_path =
        write_named_input_srt(&dir, "final.srt", "1\n00:00:00,000 --> 00:00:01,000\none");
    let terms_path = write_session_terms(&dir, "Kafka\n");
    let report_path = dir.join("evaluation-report.json");

    let output = run_evaluate(&[
        "evaluate",
        raw_path.to_str().expect("utf8 raw path"),
        final_path.to_str().expect("utf8 final path"),
        terms_path.to_str().expect("utf8 terms path"),
        report_path.to_str().expect("utf8 report path"),
    ]);

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    assert!(stderr.contains("cue count mismatch"));
    assert!(!report_path.exists());
}

#[test]
fn evaluate_refuses_existing_destination_and_preserves_bytes() {
    let dir = temp_dir("evaluate-existing-destination");
    let raw_path = write_named_input_srt(&dir, "raw.srt", "1\n00:00:00,000 --> 00:00:01,000\none");
    let final_path =
        write_named_input_srt(&dir, "final.srt", "1\n00:00:00,000 --> 00:00:01,000\none");
    let terms_path = write_session_terms(&dir, "Kafka\n");
    let report_path = dir.join("evaluation-report.json");
    let existing = b"{\"existing\":true}\n";
    std::fs::write(&report_path, existing).expect("write existing report");

    let output = run_evaluate(&[
        "evaluate",
        raw_path.to_str().expect("utf8 raw path"),
        final_path.to_str().expect("utf8 final path"),
        terms_path.to_str().expect("utf8 terms path"),
        report_path.to_str().expect("utf8 report path"),
    ]);

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    assert!(stderr.contains("destination already exists"));
    assert_eq!(
        std::fs::read(&report_path).expect("read report bytes"),
        existing
    );
}

#[test]
fn evaluate_refuses_output_path_equal_to_raw_input_and_preserves_raw_bytes() {
    let dir = temp_dir("evaluate-output-equals-raw");
    let raw_path = write_named_input_srt(&dir, "raw.srt", "1\n00:00:00,000 --> 00:00:01,000\none");
    let final_path =
        write_named_input_srt(&dir, "final.srt", "1\n00:00:00,000 --> 00:00:01,000\none");
    let terms_path = write_session_terms(&dir, "Kafka\n");
    let raw_bytes = std::fs::read(&raw_path).expect("read raw bytes");

    let output = run_evaluate(&[
        "evaluate",
        raw_path.to_str().expect("utf8 raw path"),
        final_path.to_str().expect("utf8 final path"),
        terms_path.to_str().expect("utf8 terms path"),
        raw_path.to_str().expect("utf8 report path"),
    ]);

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    assert!(stderr.contains("destination must differ from all inputs"));
    assert_eq!(std::fs::read(&raw_path).expect("read raw bytes"), raw_bytes);
}

#[test]
fn evaluate_wrong_arity_prints_usage_and_exits_nonzero() {
    let output = run_evaluate(&["evaluate", "raw.srt", "final.srt", "terms.txt"]);

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    assert!(stderr.contains("usage:"));
    assert!(stderr.contains(
        "vox-proof evaluate <raw-input.srt> <final-input.srt> <session-terms.txt> <evaluation-report.json>"
    ));
}

#[test]
fn evaluate_work_budget_refusal_creates_no_report() {
    let dir = temp_dir("evaluate-work-budget");
    let raw_text = "a".repeat(2001);
    let final_text = "b".repeat(2000);
    let raw_path = write_named_input_srt(
        &dir,
        "raw.srt",
        &format!("1\n00:00:00,000 --> 00:00:01,000\n{raw_text}"),
    );
    let final_path = write_named_input_srt(
        &dir,
        "final.srt",
        &format!("1\n00:00:00,000 --> 00:00:01,000\n{final_text}"),
    );
    let terms_path = write_session_terms(&dir, "Kafka\n");
    let report_path = dir.join("evaluation-report.json");

    let output = run_evaluate(&[
        "evaluate",
        raw_path.to_str().expect("utf8 raw path"),
        final_path.to_str().expect("utf8 final path"),
        terms_path.to_str().expect("utf8 terms path"),
        report_path.to_str().expect("utf8 report path"),
    ]);

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    assert!(stderr.contains("local diff work budget exceeded"));
    assert!(stderr.contains("raw_scalar_count: 2001"));
    assert!(stderr.contains("final_scalar_count: 2000"));
    assert!(stderr.contains("max_lcs_cells: 4000000"));
    assert!(!report_path.exists());
}

#[test]
fn evaluate_rejects_malformed_final_srt() {
    let dir = temp_dir("evaluate-malformed-final");
    let raw_path = write_named_input_srt(&dir, "raw.srt", "1\n00:00:00,000 --> 00:00:01,000\none");
    let final_path = write_named_input_srt(&dir, "final.srt", "not srt");
    let terms_path = write_session_terms(&dir, "Kafka\n");
    let report_path = dir.join("evaluation-report.json");

    let output = run_evaluate(&[
        "evaluate",
        raw_path.to_str().expect("utf8 raw path"),
        final_path.to_str().expect("utf8 final path"),
        terms_path.to_str().expect("utf8 terms path"),
        report_path.to_str().expect("utf8 report path"),
    ]);

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    assert!(stderr.contains("failed to parse final SRT"));
    assert!(!report_path.exists());
}

#[test]
fn evaluate_rejects_malformed_session_terms() {
    let dir = temp_dir("evaluate-malformed-terms");
    let raw_path = write_named_input_srt(&dir, "raw.srt", "1\n00:00:00,000 --> 00:00:01,000\none");
    let final_path =
        write_named_input_srt(&dir, "final.srt", "1\n00:00:00,000 --> 00:00:01,000\none");
    let terms_path = dir.join("terms.txt");
    std::fs::write(&terms_path, "bad | alias:\n").expect("write terms");
    let report_path = dir.join("evaluation-report.json");

    let output = run_evaluate(&[
        "evaluate",
        raw_path.to_str().expect("utf8 raw path"),
        final_path.to_str().expect("utf8 final path"),
        terms_path.to_str().expect("utf8 terms path"),
        report_path.to_str().expect("utf8 report path"),
    ]);

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    assert!(stderr.contains("session terms"));
    assert!(!report_path.exists());
}

#[test]
fn evaluate_rejects_raw_validation_issue() {
    let dir = temp_dir("evaluate-raw-validation");
    let raw_path = write_named_input_srt(
        &dir,
        "raw.srt",
        "1\n00:00:03,000 --> 00:00:02,500\nreversed",
    );
    let final_path =
        write_named_input_srt(&dir, "final.srt", "1\n00:00:00,000 --> 00:00:01,000\none");
    let terms_path = write_session_terms(&dir, "Kafka\n");
    let report_path = dir.join("evaluation-report.json");

    let output = run_evaluate(&[
        "evaluate",
        raw_path.to_str().expect("utf8 raw path"),
        final_path.to_str().expect("utf8 final path"),
        terms_path.to_str().expect("utf8 terms path"),
        report_path.to_str().expect("utf8 report path"),
    ]);

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    assert!(stderr.contains("raw SRT has validation issues; evaluation refused"));
    assert!(!report_path.exists());
}

#[test]
fn evaluate_rejects_final_validation_issue() {
    let dir = temp_dir("evaluate-final-validation");
    let raw_path = write_named_input_srt(&dir, "raw.srt", "1\n00:00:00,000 --> 00:00:01,000\none");
    let final_path = write_named_input_srt(
        &dir,
        "final.srt",
        "1\n00:00:03,000 --> 00:00:02,500\nreversed",
    );
    let terms_path = write_session_terms(&dir, "Kafka\n");
    let report_path = dir.join("evaluation-report.json");

    let output = run_evaluate(&[
        "evaluate",
        raw_path.to_str().expect("utf8 raw path"),
        final_path.to_str().expect("utf8 final path"),
        terms_path.to_str().expect("utf8 terms path"),
        report_path.to_str().expect("utf8 report path"),
    ]);

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    assert!(stderr.contains("final SRT has validation issues; evaluation refused"));
    assert!(!report_path.exists());
}

#[test]
fn evaluate_rejects_index_mismatch_without_report() {
    let dir = temp_dir("evaluate-index-mismatch");
    let raw_path = write_named_input_srt(&dir, "raw.srt", "1\n00:00:00,000 --> 00:00:01,000\none");
    let final_path =
        write_named_input_srt(&dir, "final.srt", "2\n00:00:00,000 --> 00:00:01,000\none");
    let terms_path = write_session_terms(&dir, "Kafka\n");
    let report_path = dir.join("evaluation-report.json");

    let output = run_evaluate(&[
        "evaluate",
        raw_path.to_str().expect("utf8 raw path"),
        final_path.to_str().expect("utf8 final path"),
        terms_path.to_str().expect("utf8 terms path"),
        report_path.to_str().expect("utf8 report path"),
    ]);

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    assert!(stderr.contains("cue index mismatch"));
    assert!(!report_path.exists());
}

#[test]
fn evaluate_rejects_start_timing_mismatch_without_report() {
    let dir = temp_dir("evaluate-start-mismatch");
    let raw_path = write_named_input_srt(&dir, "raw.srt", "1\n00:00:00,000 --> 00:00:01,000\none");
    let final_path =
        write_named_input_srt(&dir, "final.srt", "1\n00:00:00,100 --> 00:00:01,000\none");
    let terms_path = write_session_terms(&dir, "Kafka\n");
    let report_path = dir.join("evaluation-report.json");

    let output = run_evaluate(&[
        "evaluate",
        raw_path.to_str().expect("utf8 raw path"),
        final_path.to_str().expect("utf8 final path"),
        terms_path.to_str().expect("utf8 terms path"),
        report_path.to_str().expect("utf8 report path"),
    ]);

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    assert!(stderr.contains("start timing mismatch"));
    assert!(!report_path.exists());
}

#[test]
fn evaluate_rejects_end_timing_mismatch_without_report() {
    let dir = temp_dir("evaluate-end-mismatch");
    let raw_path = write_named_input_srt(&dir, "raw.srt", "1\n00:00:00,000 --> 00:00:01,000\none");
    let final_path =
        write_named_input_srt(&dir, "final.srt", "1\n00:00:00,000 --> 00:00:01,100\none");
    let terms_path = write_session_terms(&dir, "Kafka\n");
    let report_path = dir.join("evaluation-report.json");

    let output = run_evaluate(&[
        "evaluate",
        raw_path.to_str().expect("utf8 raw path"),
        final_path.to_str().expect("utf8 final path"),
        terms_path.to_str().expect("utf8 terms path"),
        report_path.to_str().expect("utf8 report path"),
    ]);

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    assert!(stderr.contains("end timing mismatch"));
    assert!(!report_path.exists());
}

#[test]
fn evaluate_structural_refusal_precedes_work_budget_without_report() {
    let dir = temp_dir("evaluate-structural-before-budget");
    let raw_text = "a".repeat(2001);
    let final_text = "b".repeat(2000);
    let raw_path = write_named_input_srt(
        &dir,
        "raw.srt",
        &format!(
            "1\n00:00:00,000 --> 00:00:01,000\n{raw_text}\n\n2\n00:00:01,000 --> 00:00:02,000\nx"
        ),
    );
    let final_path = write_named_input_srt(
        &dir,
        "final.srt",
        &format!("1\n00:00:00,000 --> 00:00:01,000\n{final_text}"),
    );
    let terms_path = write_session_terms(&dir, "Kafka\n");
    let report_path = dir.join("evaluation-report.json");

    let output = run_evaluate(&[
        "evaluate",
        raw_path.to_str().expect("utf8 raw path"),
        final_path.to_str().expect("utf8 final path"),
        terms_path.to_str().expect("utf8 terms path"),
        report_path.to_str().expect("utf8 report path"),
    ]);

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    assert!(stderr.contains("cue count mismatch"));
    assert!(!stderr.contains("local diff work budget exceeded"));
    assert!(!report_path.exists());
}

#[test]
fn evaluate_refuses_output_path_equal_to_final_input_and_preserves_final_bytes() {
    let dir = temp_dir("evaluate-output-equals-final");
    let raw_path = write_named_input_srt(&dir, "raw.srt", "1\n00:00:00,000 --> 00:00:01,000\none");
    let final_path =
        write_named_input_srt(&dir, "final.srt", "1\n00:00:00,000 --> 00:00:01,000\none");
    let terms_path = write_session_terms(&dir, "Kafka\n");
    let final_bytes = std::fs::read(&final_path).expect("read final bytes");

    let output = run_evaluate(&[
        "evaluate",
        raw_path.to_str().expect("utf8 raw path"),
        final_path.to_str().expect("utf8 final path"),
        terms_path.to_str().expect("utf8 terms path"),
        final_path.to_str().expect("utf8 report path"),
    ]);

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    assert!(stderr.contains("destination must differ from all inputs"));
    assert_eq!(
        std::fs::read(&final_path).expect("read final bytes"),
        final_bytes
    );
}

#[test]
fn evaluate_refuses_output_path_equal_to_terms_input_and_preserves_terms_bytes() {
    let dir = temp_dir("evaluate-output-equals-terms");
    let raw_path = write_named_input_srt(&dir, "raw.srt", "1\n00:00:00,000 --> 00:00:01,000\none");
    let final_path =
        write_named_input_srt(&dir, "final.srt", "1\n00:00:00,000 --> 00:00:01,000\none");
    let terms_path = write_session_terms(&dir, "Kafka\n");
    let terms_bytes = std::fs::read(&terms_path).expect("read terms bytes");

    let output = run_evaluate(&[
        "evaluate",
        raw_path.to_str().expect("utf8 raw path"),
        final_path.to_str().expect("utf8 final path"),
        terms_path.to_str().expect("utf8 terms path"),
        terms_path.to_str().expect("utf8 report path"),
    ]);

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    assert!(stderr.contains("destination must differ from all inputs"));
    assert_eq!(
        std::fs::read(&terms_path).expect("read terms bytes"),
        terms_bytes
    );
}

#[test]
fn evaluate_report_carries_local_calibration_disclaimer() {
    let dir = temp_dir("evaluate-disclaimer");
    let raw_path = write_named_input_srt(&dir, "raw.srt", "1\n00:00:00,000 --> 00:00:01,000\none");
    let final_path =
        write_named_input_srt(&dir, "final.srt", "1\n00:00:00,000 --> 00:00:01,000\none");
    let terms_path = write_session_terms(&dir, "Kafka\n");
    let report_path = dir.join("evaluation-report.json");

    let output = run_evaluate(&[
        "evaluate",
        raw_path.to_str().expect("utf8 raw path"),
        final_path.to_str().expect("utf8 final path"),
        terms_path.to_str().expect("utf8 terms path"),
        report_path.to_str().expect("utf8 report path"),
    ]);

    assert!(output.status.success(), "{output:?}");
    let report = std::fs::read_to_string(&report_path).expect("read report");
    assert!(report.contains("Deterministic local calibration correspondence artifact only"));
    assert!(!report.contains("Material Decision"));
}

#[test]
fn evaluate_is_deterministic_for_identical_inputs() {
    let dir = temp_dir("evaluate-deterministic");
    let raw_path =
        write_named_input_srt(&dir, "raw.srt", "1\n00:00:00,000 --> 00:00:01,000\nKafka");
    let final_path = write_named_input_srt(
        &dir,
        "final.srt",
        "1\n00:00:00,000 --> 00:00:01,000\nApache Kafka",
    );
    let terms_path = write_session_terms(&dir, "Apache Kafka | alias:Kafka\n");
    let report_a = dir.join("evaluation-report-a.json");
    let report_b = dir.join("evaluation-report-b.json");

    let first = run_evaluate(&[
        "evaluate",
        raw_path.to_str().expect("utf8 raw path"),
        final_path.to_str().expect("utf8 final path"),
        terms_path.to_str().expect("utf8 terms path"),
        report_a.to_str().expect("utf8 report path"),
    ]);
    let second = run_evaluate(&[
        "evaluate",
        raw_path.to_str().expect("utf8 raw path"),
        final_path.to_str().expect("utf8 final path"),
        terms_path.to_str().expect("utf8 terms path"),
        report_b.to_str().expect("utf8 report path"),
    ]);

    assert!(first.status.success(), "{first:?}");
    assert!(second.status.success(), "{second:?}");
    let a = std::fs::read_to_string(&report_a).expect("read a");
    let b = std::fs::read_to_string(&report_b).expect("read b");
    assert_eq!(a, b);
}

#[test]
fn evaluate_leaves_seekable_stdin_unread_and_emits_only_success_line() {
    let dir = temp_dir("evaluate-stdin-unread");
    let raw_path = write_named_input_srt(&dir, "raw.srt", "1\n00:00:00,000 --> 00:00:01,000\none");
    let final_path =
        write_named_input_srt(&dir, "final.srt", "1\n00:00:00,000 --> 00:00:01,000\none");
    let terms_path = write_session_terms(&dir, "Kafka\n");
    let report_path = dir.join("evaluation-report.json");
    let stdin_path = dir.join("stdin-payload.txt");
    let payload = b"THIS MUST REMAIN UNREAD\n";

    let output = run_with_seekable_stdin(
        &[
            "evaluate",
            raw_path.to_str().expect("utf8 raw path"),
            final_path.to_str().expect("utf8 final path"),
            terms_path.to_str().expect("utf8 terms path"),
            report_path.to_str().expect("utf8 report path"),
        ],
        &stdin_path,
        payload,
    );

    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    assert_eq!(
        stdout,
        format!(
            "wrote calibration evaluation report: {}\n",
            report_path.display()
        )
    );
    assert!(stderr.is_empty(), "stderr must be empty, got: {stderr:?}");
    assert!(!stdout.contains("decision:"));
    assert!(!stdout.contains("usage:"));
    let report = std::fs::read_to_string(&report_path).expect("read report");
    assert!(report.contains("\"schema_revision\": \"voxproof-calibration-correspondence-v0\""));
    assert!(report.contains("\"review_case_count\": 0"));
}

#[test]
fn compare_contract_unchanged_alongside_evaluate_work() {
    let dir = temp_dir("compare-evaluate-coexistence");
    let raw_srt =
        "1\n00:00:00,000 --> 00:00:01,000\nKafka\n\n2\n00:00:01,000 --> 00:00:02,000\nlast";
    let final_srt =
        "1\n00:00:00,000 --> 00:00:01,000\nApache Kafka\n\n2\n00:00:01,000 --> 00:00:02,000\nlast";
    let raw_path = write_named_input_srt(&dir, "raw.srt", raw_srt);
    let final_path = write_named_input_srt(&dir, "final.srt", final_srt);
    let terms_path = write_session_terms(&dir, "Apache Kafka | alias:Kafka\n");
    let compare_report_path = dir.join("comparison-report.json");
    let evaluate_report_path = dir.join("evaluation-report.json");
    let raw_path_str = raw_path.to_str().expect("utf8 raw path");
    let final_path_str = final_path.to_str().expect("utf8 final path");
    let expected_compare_literal = format!(
        r#"{{
  "schema_revision": "voxproof-calibration-comparison-v0",
  "compatibility_policy_id": "identical-cue-count-index-and-timing-v0",
  "note": "Calibration artifact only. Not canonical Evidence, not ground truth, not precision/recall/correctness, and not a Material Decision.",
  "inputs": {{
    "raw_path": "{raw_path_str}",
    "final_path": "{final_path_str}",
    "raw_revision_id": "rev:sha256-v1:2a94d52a44510636b806d59bc4e9c215a341ee85575a3996d7607269051cc68e",
    "final_revision_id": "rev:sha256-v1:7cec84e3e2fa628481c9b8dee314b0157e94a87a8b3963e6c238932f422f21b0"
  }},
  "summary": {{
    "cue_count": 2,
    "unchanged_count": 1,
    "text_changed_count": 1
  }},
  "cues": [
    {{
      "segment_position": 0,
      "cue_index": 1,
      "start_ms": 0,
      "end_ms": 1000,
      "change_kind": "text_changed",
      "raw_text": "Kafka",
      "final_text": "Apache Kafka"
    }},
    {{
      "segment_position": 1,
      "cue_index": 2,
      "start_ms": 1000,
      "end_ms": 2000,
      "change_kind": "unchanged",
      "raw_text": "last",
      "final_text": "last"
    }}
  ]
}}
"#
    );

    let compare_output = run_compare(&[
        "compare",
        raw_path_str,
        final_path_str,
        compare_report_path
            .to_str()
            .expect("utf8 compare report path"),
    ]);
    assert!(compare_output.status.success(), "{compare_output:?}");
    let compare_stdout = String::from_utf8(compare_output.stdout).expect("utf8 compare stdout");
    let compare_stderr = String::from_utf8(compare_output.stderr).expect("utf8 compare stderr");
    assert_eq!(
        compare_stdout,
        format!(
            "wrote comparison report: {}\n",
            compare_report_path.display()
        )
    );
    assert!(compare_stderr.is_empty(), "compare stderr must be empty");
    let compare_report_bytes =
        std::fs::read(&compare_report_path).expect("read compare report bytes");
    assert_eq!(compare_report_bytes, expected_compare_literal.as_bytes());

    let evaluate_output = run_evaluate(&[
        "evaluate",
        raw_path_str,
        final_path_str,
        terms_path.to_str().expect("utf8 terms path"),
        evaluate_report_path
            .to_str()
            .expect("utf8 evaluate report path"),
    ]);
    assert!(evaluate_output.status.success(), "{evaluate_output:?}");
    let evaluate_report =
        std::fs::read_to_string(&evaluate_report_path).expect("read evaluate report");
    assert!(
        evaluate_report.contains("\"schema_revision\": \"voxproof-calibration-correspondence-v0\"")
    );
    assert!(!evaluate_report.contains("voxproof-calibration-comparison-v0"));
    assert!(
        !compare_report_bytes
            .starts_with(b"{\"schema_revision\": \"voxproof-calibration-correspondence-v0\"")
    );
}
