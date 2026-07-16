use std::io::{self, BufRead, Read, Write};
use std::process::ExitCode;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use serde::Serialize;
use vox_proof::calibration::{
    build_comparison_report, render_comparison_report, write_comparison_report_exclusive,
};
use vox_proof::candidate::Evidence;
use vox_proof::experimental_ranking::{
    ExperimentalContextRanker, ExperimentalRankingResult, ExternalCommandRanker,
    rank_experimental_candidates,
};
use vox_proof::experimental_retrieval::{
    ExperimentalCandidateReport, ExperimentalLatinSpanEligibilityProfile,
    ExperimentalPinyinEligibilityProfile, ExperimentalRetrievalConfig,
    retrieve_experimental_candidates,
};
use vox_proof::pipeline::run_term_review;
use vox_proof::review::{CorrectionDecision, ReviewCase, ReviewLedger};
use vox_proof::reviewed_output::derive_reviewed_srt;
use vox_proof::session_log::render_decision_log;
use vox_proof::session_summary::{
    CompletedSession, SessionInputPaths, SessionOutputPaths, SessionTiming,
    collect_session_summary, render_session_summary,
};
use vox_proof::session_terms::parse_session_terms;
use vox_proof::srt::parse_srt;
use vox_proof::transcript::Transcript;

fn main() -> ExitCode {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let result = match args.first().map(String::as_str) {
        Some("review") => run_review_from_args(&args),
        Some("review-experiment") => run_experiment_from_args(&args),
        Some("compare") => run_compare_from_args(&args),
        _ => run_parse_command(&args),
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

#[derive(Serialize)]
struct ExperimentalRunSidecar {
    schema_revision: &'static str,
    session_description: String,
    ranker_mode: String,
    pinyin_eligibility_profile: ExperimentalPinyinEligibilityProfile,
    latin_span_eligibility_profile: ExperimentalLatinSpanEligibilityProfile,
    reports: Vec<ExperimentalCandidateReport>,
    rankings: Vec<ExperimentalRankingResult>,
    manual_correction_markers: Vec<ExperimentalManualCorrectionMarker>,
    note: &'static str,
}

#[derive(Serialize)]
struct ExperimentalManualCorrectionMarker {
    marker: &'static str,
    candidate_id: String,
    source_surface: String,
    canonical_term: String,
    guidance: &'static str,
}

fn run_parse_command(args: &[String]) -> Result<(), String> {
    let input = match read_parse_input(args) {
        Ok(text) => text,
        Err(error) => return Err(format!("failed to read input: {error}")),
    };

    let transcript = match parse_srt(&input) {
        Ok(transcript) => transcript,
        Err(error) => return Err(format!("failed to parse SRT: {error:?}")),
    };

    let stdout = io::stdout();
    let mut output = stdout.lock();
    print_parse_summary(&transcript, &mut output).map_err(|error| error.to_string())
}

fn run_experiment_from_args(args: &[String]) -> Result<(), String> {
    if args.len() != 9 {
        return Err(experiment_usage().to_string());
    }
    let ranker = ranker_from_mode(&args[4])?;
    let retrieval_config = experimental_retrieval_config_from_environment()?;
    let stdin = io::stdin();
    let stdout = io::stdout();
    run_experiment_command(
        &args[1],
        &args[2],
        &args[3],
        &args[4],
        ranker,
        retrieval_config,
        &args[5],
        &args[6],
        &args[7],
        &args[8],
        stdin.lock(),
        stdout.lock(),
    )
}

#[allow(clippy::too_many_arguments)]
fn run_experiment_command<R: BufRead, W: Write>(
    input_path: &str,
    session_terms_path: &str,
    description_path: &str,
    ranker_mode: &str,
    ranker: ExperimentalContextRanker,
    retrieval_config: ExperimentalRetrievalConfig,
    experimental_report_path: &str,
    reviewed_output_path: &str,
    decision_log_path: &str,
    session_summary_path: &str,
    mut input: R,
    mut output: W,
) -> Result<(), String> {
    let session_start = SystemTime::now();
    let session_timer = Instant::now();
    let input_srt = std::fs::read_to_string(input_path)
        .map_err(|error| format!("failed to read input SRT: {error}"))?;
    let session_terms_text = std::fs::read_to_string(session_terms_path)
        .map_err(|error| format!("failed to read session terms: {error}"))?;
    let session_description = std::fs::read_to_string(description_path)
        .map_err(|error| format!("failed to read experimental session description: {error}"))?;
    let session_terms =
        parse_session_terms(&session_terms_text).map_err(|error| error.to_string())?;
    let transcript =
        parse_srt(&input_srt).map_err(|error| format!("failed to parse SRT: {error:?}"))?;

    print_parse_summary(&transcript, &mut output).map_err(|error| error.to_string())?;
    writeln!(output, "experimental ranker mode: {ranker_mode}")
        .map_err(|error| error.to_string())?;
    writeln!(
        output,
        "experimental pinyin eligibility profile: {:?}",
        retrieval_config.pinyin_eligibility_profile
    )
    .map_err(|error| error.to_string())?;
    writeln!(
        output,
        "experimental Latin span eligibility profile: {:?}",
        retrieval_config.latin_span_eligibility_profile
    )
    .map_err(|error| error.to_string())?;
    writeln!(
        output,
        "experimental non-exact suggestions are not review cases and cannot change reviewed SRT"
    )
    .map_err(|error| error.to_string())?;

    let reports = retrieve_experimental_candidates(&transcript, &session_terms, &retrieval_config);
    let (rankings, manual_correction_markers) = review_experimental_reports(
        &transcript,
        &reports,
        &ranker,
        &session_description,
        &mut input,
        &mut output,
    )?;

    // The exact path remains independently authoritative and intentionally
    // receives no experimental reports, selections, or ranker output.
    let review_cases = run_term_review(&transcript, &session_terms)
        .map_err(|error| format!("failed to run session-term review: {error:?}"))?;
    let ledger = if review_cases.is_empty() {
        writeln!(output, "no exact review cases found").map_err(|error| error.to_string())?;
        ReviewLedger::new()
    } else {
        review_cases_interactively(&transcript, &review_cases, &mut input, &mut output)?
    };

    let reviewed_srt = derive_reviewed_srt(&transcript, &review_cases, &ledger)
        .map_err(|error| format!("failed to derive reviewed SRT: {error:?}"))?;
    let decision_log = render_decision_log(&ledger);
    let session_end = SystemTime::now();
    let summary = collect_session_summary(CompletedSession {
        transcript: &transcript,
        review_cases: &review_cases,
        ledger: &ledger,
        session_term_entries: session_terms.len(),
        inputs: SessionInputPaths {
            input_srt: input_path.to_string(),
            session_terms: session_terms_path.to_string(),
        },
        timing: SessionTiming {
            start_unix_ms: unix_time_ms(session_start)?,
            end_unix_ms: unix_time_ms(session_end)?,
            elapsed_ms: session_timer.elapsed().as_millis(),
        },
        outputs: SessionOutputPaths {
            reviewed_srt: reviewed_output_path.to_string(),
            decision_log: decision_log_path.to_string(),
            session_summary: session_summary_path.to_string(),
        },
    });
    let sidecar = ExperimentalRunSidecar {
        schema_revision: "experimental-contextual-resolution-sidecar-v3",
        session_description,
        ranker_mode: ranker_mode.to_string(),
        pinyin_eligibility_profile: retrieval_config.pinyin_eligibility_profile,
        latin_span_eligibility_profile: retrieval_config.latin_span_eligibility_profile,
        reports,
        rankings,
        manual_correction_markers,
        note: "Experimental only: these markers are not ReviewLedger decisions, candidates, or materialized edits.",
    };
    let sidecar_json = serde_json::to_string_pretty(&sidecar)
        .map_err(|error| format!("failed to render experimental report: {error}"))?;

    std::fs::write(experimental_report_path, sidecar_json)
        .map_err(|error| format!("failed to write experimental report: {error}"))?;
    std::fs::write(reviewed_output_path, reviewed_srt)
        .map_err(|error| format!("failed to write reviewed SRT: {error}"))?;
    std::fs::write(decision_log_path, decision_log).map_err(|error| {
        format!("failed to write decision log: {error}; reviewed SRT may already exist; session output is incomplete")
    })?;
    std::fs::write(session_summary_path, render_session_summary(&summary)).map_err(|error| {
        format!("failed to write session summary: {error}; reviewed SRT and decision log may already exist; session output is incomplete")
    })?;
    writeln!(
        output,
        "wrote experimental report: {experimental_report_path}"
    )
    .map_err(|error| error.to_string())?;
    writeln!(output, "wrote reviewed SRT: {reviewed_output_path}")
        .map_err(|error| error.to_string())?;
    Ok(())
}

fn experimental_retrieval_config_from_environment() -> Result<ExperimentalRetrievalConfig, String> {
    Ok(ExperimentalRetrievalConfig {
        pinyin_eligibility_profile: experimental_pinyin_profile_from_environment()?,
        latin_span_eligibility_profile: experimental_latin_profile_from_environment()?,
        ..ExperimentalRetrievalConfig::default()
    })
}

fn experimental_pinyin_profile_from_environment()
-> Result<ExperimentalPinyinEligibilityProfile, String> {
    match std::env::var("VOX_PROOF_EXPERIMENT_PINYIN_PROFILE") {
        Ok(value) if value == "unfiltered-baseline-v1" => {
            Ok(ExperimentalPinyinEligibilityProfile::UnfilteredBaselineV1)
        }
        Ok(value) if value == "suppress-short-han-to-short-uppercase-acronym-v1" => {
            Ok(ExperimentalPinyinEligibilityProfile::SuppressShortHanToShortUppercaseAcronymV1)
        }
        Ok(value) => Err(format!(
            "unknown VOX_PROOF_EXPERIMENT_PINYIN_PROFILE '{value}'; expected unfiltered-baseline-v1 or suppress-short-han-to-short-uppercase-acronym-v1"
        )),
        Err(std::env::VarError::NotPresent) => {
            Ok(ExperimentalPinyinEligibilityProfile::SuppressShortHanToShortUppercaseAcronymV1)
        }
        Err(std::env::VarError::NotUnicode(_)) => {
            Err("VOX_PROOF_EXPERIMENT_PINYIN_PROFILE contains a non-Unicode value".to_string())
        }
    }
}

fn experimental_latin_profile_from_environment()
-> Result<ExperimentalLatinSpanEligibilityProfile, String> {
    match std::env::var("VOX_PROOF_EXPERIMENT_LATIN_SPAN_PROFILE") {
        Ok(value) if value == "unfiltered-baseline-v1" => {
            Ok(ExperimentalLatinSpanEligibilityProfile::UnfilteredBaselineV1)
        }
        Ok(value) if value == "suppress-target-embedded-in-larger-window-v1" => {
            Ok(ExperimentalLatinSpanEligibilityProfile::SuppressTargetEmbeddedInLargerWindowV1)
        }
        Ok(value) => Err(format!(
            "unknown VOX_PROOF_EXPERIMENT_LATIN_SPAN_PROFILE '{value}'; expected unfiltered-baseline-v1 or suppress-target-embedded-in-larger-window-v1"
        )),
        Err(std::env::VarError::NotPresent) => {
            Ok(ExperimentalLatinSpanEligibilityProfile::SuppressTargetEmbeddedInLargerWindowV1)
        }
        Err(std::env::VarError::NotUnicode(_)) => {
            Err("VOX_PROOF_EXPERIMENT_LATIN_SPAN_PROFILE contains a non-Unicode value".to_string())
        }
    }
}

fn ranker_from_mode(mode: &str) -> Result<ExperimentalContextRanker, String> {
    match mode {
        "rules-only" => Ok(ExperimentalContextRanker::RulesOnly),
        "fake" => Ok(ExperimentalContextRanker::DeterministicFake),
        "external-command" => {
            let program = std::env::var("VOX_PROOF_EXPERIMENT_COMMAND").map_err(|_| {
                "external-command requires VOX_PROOF_EXPERIMENT_COMMAND; no credentials are read".to_string()
            })?;
            let timeout_ms = std::env::var("VOX_PROOF_EXPERIMENT_TIMEOUT_MS")
                .ok()
                .and_then(|value| value.parse().ok())
                .unwrap_or(1_000);
            Ok(ExperimentalContextRanker::ExternalCommand(
                ExternalCommandRanker {
                    program,
                    arguments: Vec::new(),
                    timeout_ms,
                    max_request_bytes: 256 * 1024,
                    max_output_bytes: 16 * 1024,
                },
            ))
        }
        _ => Err("ranker mode must be rules-only, fake, or external-command".to_string()),
    }
}

fn review_experimental_reports<R: BufRead, W: Write>(
    transcript: &Transcript,
    reports: &[ExperimentalCandidateReport],
    ranker: &ExperimentalContextRanker,
    session_description: &str,
    input: &mut R,
    output: &mut W,
) -> Result<
    (
        Vec<ExperimentalRankingResult>,
        Vec<ExperimentalManualCorrectionMarker>,
    ),
    String,
> {
    let mut groups = std::collections::BTreeMap::<
        (usize, usize, usize),
        Vec<&ExperimentalCandidateReport>,
    >::new();
    for report in reports {
        groups
            .entry((
                report.source_anchor.segment_position,
                report.source_anchor.start_byte,
                report.source_anchor.end_byte,
            ))
            .or_default()
            .push(report);
    }
    let mut rankings = Vec::new();
    let mut markers = Vec::new();
    for group in groups.into_values() {
        let first = group[0];
        let nearby_context = transcript
            .segments()
            .get(first.source_anchor.segment_position)
            .map(|segment| segment.text())
            .unwrap_or("");
        let ranking = rank_experimental_candidates(
            ranker,
            session_description,
            nearby_context,
            &first.source_surface,
            &group
                .iter()
                .map(|report| (*report).clone())
                .collect::<Vec<_>>(),
        );
        writeln!(output, "\nexperimental source: {}", first.source_surface)
            .map_err(|error| error.to_string())?;
        writeln!(output, "nearby context: {nearby_context}").map_err(|error| error.to_string())?;
        for report in &group {
            writeln!(
                output,
                "  {}: {} via {:?}; distance={}; ratio={}/{}",
                report.candidate_id,
                report.canonical_term,
                report.producer,
                report.distance,
                report.ratio_numerator,
                report.ratio_denominator
            )
            .map_err(|error| error.to_string())?;
        }
        writeln!(
            output,
            "ranking: {:?}; disposition={:?}; selected={:?}; fallback={:?}; requires_review={}",
            ranking.assessment,
            ranking.disposition,
            ranking.selected_candidate_id,
            ranking.failure,
            ranking.requires_review
        )
        .map_err(|error| error.to_string())?;
        write!(output, "Experimental selection [s <candidate-id>/n]: ")
            .map_err(|error| error.to_string())?;
        output.flush().map_err(|error| error.to_string())?;
        let mut line = String::new();
        input
            .read_line(&mut line)
            .map_err(|error| format!("failed to read experimental selection: {error}"))?;
        if let Some(id) = line.trim().strip_prefix("s ") {
            let selected = group
                .iter()
                .find(|report| report.candidate_id == id)
                .ok_or("experimental selection must name a displayed candidate ID")?;
            markers.push(ExperimentalManualCorrectionMarker {
                marker: "manual_correction_requested",
                candidate_id: selected.candidate_id.clone(),
                source_surface: selected.source_surface.clone(),
                canonical_term: selected.canonical_term.clone(),
                guidance: "Reviewed SRT remains unchanged. Add this source form as an explicit session alias for the canonical term, then rerun the exact review flow.",
            });
            writeln!(
                output,
                "recorded experimental manual-correction marker; reviewed SRT remains unchanged. Add `alias:{}` to the existing `{}` session-term entry and rerun.",
                selected.source_surface, selected.canonical_term
            )
            .map_err(|error| error.to_string())?;
        } else if line.trim() != "n" {
            return Err("experimental selection must be s <candidate-id> or n".to_string());
        }
        rankings.push(ranking);
    }
    Ok((rankings, markers))
}

fn run_compare_from_args(args: &[String]) -> Result<(), String> {
    if args.len() != 4 {
        return Err(compare_usage().to_string());
    }

    run_compare_command(&args[1], &args[2], &args[3])
}

fn run_compare_command(raw_path: &str, final_path: &str, report_path: &str) -> Result<(), String> {
    let raw_srt = std::fs::read_to_string(raw_path)
        .map_err(|error| format!("failed to read raw input: {error}"))?;
    let final_srt = std::fs::read_to_string(final_path)
        .map_err(|error| format!("failed to read final input: {error}"))?;

    let raw_transcript =
        parse_srt(&raw_srt).map_err(|error| format!("failed to parse raw SRT: {error:?}"))?;
    let final_transcript =
        parse_srt(&final_srt).map_err(|error| format!("failed to parse final SRT: {error:?}"))?;

    let report = build_comparison_report(&raw_transcript, &final_transcript, raw_path, final_path)
        .map_err(|refusal| refusal.message())?;

    let json = render_comparison_report(&report)
        .map_err(|error| format!("failed to serialize comparison report: {error}"))?;

    write_comparison_report_exclusive(report_path, &json)?;

    let stdout = io::stdout();
    let mut output = stdout.lock();
    writeln!(output, "wrote comparison report: {report_path}").map_err(|error| error.to_string())
}

fn run_review_from_args(args: &[String]) -> Result<(), String> {
    if args.len() != 6 {
        return Err(usage().to_string());
    }

    let stdin = io::stdin();
    let stdout = io::stdout();
    run_review_command(
        &args[1],
        &args[2],
        &args[3],
        &args[4],
        &args[5],
        stdin.lock(),
        stdout.lock(),
    )
}

fn run_review_command<R: BufRead, W: Write>(
    input_path: &str,
    session_terms_path: &str,
    reviewed_output_path: &str,
    decision_log_path: &str,
    session_summary_path: &str,
    input: R,
    mut output: W,
) -> Result<(), String> {
    let session_start = SystemTime::now();
    let session_timer = Instant::now();

    let input_srt = std::fs::read_to_string(input_path)
        .map_err(|error| format!("failed to read input SRT: {error}"))?;
    let session_terms_text = std::fs::read_to_string(session_terms_path)
        .map_err(|error| format!("failed to read session terms: {error}"))?;
    let session_terms =
        parse_session_terms(&session_terms_text).map_err(|error| error.to_string())?;
    let transcript =
        parse_srt(&input_srt).map_err(|error| format!("failed to parse SRT: {error:?}"))?;

    print_parse_summary(&transcript, &mut output).map_err(|error| error.to_string())?;
    writeln!(
        output,
        "loaded {} session term entries from {session_terms_path}",
        session_terms.len()
    )
    .map_err(|error| error.to_string())?;

    let review_cases = run_term_review(&transcript, &session_terms)
        .map_err(|error| format!("failed to run session-term review: {error:?}"))?;
    let ledger = if review_cases.is_empty() {
        writeln!(output, "no review cases found").map_err(|error| error.to_string())?;
        ReviewLedger::new()
    } else {
        review_cases_interactively(&transcript, &review_cases, input, &mut output)?
    };

    let reviewed_srt = derive_reviewed_srt(&transcript, &review_cases, &ledger)
        .map_err(|error| format!("failed to derive reviewed SRT: {error:?}"))?;
    let decision_log = render_decision_log(&ledger);
    let session_end = SystemTime::now();
    let summary = collect_session_summary(CompletedSession {
        transcript: &transcript,
        review_cases: &review_cases,
        ledger: &ledger,
        session_term_entries: session_terms.len(),
        inputs: SessionInputPaths {
            input_srt: input_path.to_string(),
            session_terms: session_terms_path.to_string(),
        },
        timing: SessionTiming {
            start_unix_ms: unix_time_ms(session_start)?,
            end_unix_ms: unix_time_ms(session_end)?,
            elapsed_ms: session_timer.elapsed().as_millis(),
        },
        outputs: SessionOutputPaths {
            reviewed_srt: reviewed_output_path.to_string(),
            decision_log: decision_log_path.to_string(),
            session_summary: session_summary_path.to_string(),
        },
    });
    let session_summary = render_session_summary(&summary);

    std::fs::write(reviewed_output_path, reviewed_srt)
        .map_err(|error| format!("failed to write reviewed SRT: {error}"))?;
    std::fs::write(decision_log_path, decision_log).map_err(|error| {
        format!(
            "failed to write decision log: {error}; reviewed SRT may already exist; session output is incomplete"
        )
    })?;
    std::fs::write(session_summary_path, session_summary).map_err(|error| {
        format!(
            "failed to write session summary: {error}; reviewed SRT and decision log may already exist; session output is incomplete"
        )
    })?;

    writeln!(output, "wrote reviewed SRT: {reviewed_output_path}")
        .map_err(|error| error.to_string())?;
    writeln!(output, "wrote decision log: {decision_log_path}")
        .map_err(|error| error.to_string())?;
    writeln!(output, "wrote session summary: {session_summary_path}")
        .map_err(|error| error.to_string())?;

    Ok(())
}

fn review_cases_interactively<R: BufRead, W: Write>(
    transcript: &Transcript,
    review_cases: &[ReviewCase],
    mut input: R,
    output: &mut W,
) -> Result<ReviewLedger, String> {
    let mut ledger = ReviewLedger::new();

    for review_case in review_cases {
        print_review_case(transcript, review_case, output).map_err(|error| error.to_string())?;

        loop {
            write!(output, "Decision [a <index>/r/d/m]: ").map_err(|error| error.to_string())?;
            output.flush().map_err(|error| error.to_string())?;

            let mut line = String::new();
            let bytes_read = input
                .read_line(&mut line)
                .map_err(|error| format!("failed to read decision input: {error}"))?;
            if bytes_read == 0 {
                return Err(format!(
                    "no decision provided for case local:{}",
                    review_case.id().local_index()
                ));
            }

            let decision = match parse_decision_input(line.trim()) {
                Ok(decision) => decision,
                Err(error) => {
                    writeln!(output, "invalid decision: {error}")
                        .map_err(|write_error| write_error.to_string())?;
                    continue;
                }
            };

            match ledger.record_decision(review_case, transcript.revision_id(), decision) {
                Ok(()) => break,
                Err(error) => {
                    writeln!(output, "invalid decision: {error:?}")
                        .map_err(|write_error| write_error.to_string())?;
                }
            }
        }
    }

    Ok(ledger)
}

fn parse_decision_input(input: &str) -> Result<CorrectionDecision, &'static str> {
    let mut parts = input.split_whitespace();
    let Some(command) = parts.next() else {
        return Err("enter a decision");
    };

    match command {
        "a" => {
            let index = parts
                .next()
                .ok_or("accept requires an alternative index")?
                .parse::<usize>()
                .map_err(|_| "alternative index must be a non-negative integer")?;
            if parts.next().is_some() {
                return Err("accept takes exactly one alternative index");
            }
            Ok(CorrectionDecision::AcceptAlternative {
                alternative_index: index,
            })
        }
        "r" => no_extra_parts(parts, CorrectionDecision::Reject),
        "d" => no_extra_parts(parts, CorrectionDecision::Defer),
        "m" => no_extra_parts(parts, CorrectionDecision::NeedsManualCorrection),
        _ => Err("expected a <index>, r, d, or m"),
    }
}

fn no_extra_parts<'a>(
    mut parts: impl Iterator<Item = &'a str>,
    decision: CorrectionDecision,
) -> Result<CorrectionDecision, &'static str> {
    if parts.next().is_some() {
        Err("decision takes no extra input")
    } else {
        Ok(decision)
    }
}

fn write_nearby_source_context<W: Write>(
    transcript: &Transcript,
    segment_position: usize,
    output: &mut W,
) -> io::Result<()> {
    let segments = transcript.segments();
    writeln!(
        output,
        "nearby_context_note: presentation only; not evidence, not ranking input, and not used for materialization"
    )?;

    if segment_position == 0 {
        writeln!(output, "previous_cue_index: (none)")?;
        writeln!(output, "previous_cue_text: (none)")?;
    } else if let Some(previous) = segments.get(segment_position - 1) {
        writeln!(output, "previous_cue_index: {}", previous.index())?;
        writeln!(output, "previous_cue_text: {}", previous.text())?;
    } else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "previous cue at segment position {} is unavailable in transcript with {} segment(s)",
                segment_position - 1,
                segments.len()
            ),
        ));
    }

    let current = segments.get(segment_position).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "review anchor segment position {segment_position} is outside transcript bounds (segment count: {})",
                segments.len()
            ),
        )
    })?;
    writeln!(output, "cue_index: {}", current.index())?;
    writeln!(output, "cue_text: {}", current.text())?;

    if let Some(following) = segments.get(segment_position + 1) {
        writeln!(output, "following_cue_index: {}", following.index())?;
        writeln!(output, "following_cue_text: {}", following.text())?;
    } else {
        writeln!(output, "following_cue_index: (none)")?;
        writeln!(output, "following_cue_text: (none)")?;
    }

    Ok(())
}

fn print_review_case<W: Write>(
    transcript: &Transcript,
    review_case: &ReviewCase,
    output: &mut W,
) -> io::Result<()> {
    let candidate = review_case.candidate_span();
    let anchor = candidate.anchor();
    let segment_position = anchor.segment_position();
    let matched_text = transcript.resolve(anchor).unwrap_or("<unresolved anchor>");

    writeln!(output)?;
    writeln!(output, "case_id: local:{}", review_case.id().local_index())?;
    writeln!(output, "source_segment_position: {segment_position}")?;
    write_nearby_source_context(transcript, segment_position, output)?;
    writeln!(output, "matched_text: {matched_text}")?;

    match candidate.evidence() {
        Evidence::GlossaryAlias(evidence) => {
            writeln!(
                output,
                "evidence: glossary alias '{}' for '{}'",
                evidence.matched_form, evidence.entry.canonical_term
            )?;
        }
        Evidence::ObservedErrorForm(evidence) => {
            writeln!(
                output,
                "evidence: observed error form '{}' for '{}'",
                evidence.matched_form, evidence.entry.canonical_term
            )?;
        }
    }

    writeln!(output, "alternatives:")?;
    for (index, alternative) in candidate.alternatives().iter().enumerate() {
        writeln!(output, "  {index}: {}", alternative.replacement_text())?;
    }

    Ok(())
}

fn print_parse_summary<W: Write>(transcript: &Transcript, output: &mut W) -> io::Result<()> {
    writeln!(output, "parsed {} segment(s)", transcript.segments().len())?;

    let issues = transcript.validation_issues();
    if issues.is_empty() {
        writeln!(output, "no validation issues")?;
    } else {
        writeln!(output, "{} validation issue(s):", issues.len())?;
        for issue in &issues {
            writeln!(
                output,
                "  segment position {} (cue index {}): {:?}",
                issue.segment_position(),
                issue.cue_index(),
                issue.error()
            )?;
        }
    }

    Ok(())
}

fn read_parse_input(args: &[String]) -> Result<String, String> {
    match args {
        [] => {
            let mut buffer = String::new();
            io::stdin()
                .read_to_string(&mut buffer)
                .map_err(|error| error.to_string())?;
            Ok(buffer)
        }
        [path] => std::fs::read_to_string(path).map_err(|error| error.to_string()),
        _ => Err(usage().to_string()),
    }
}

fn unix_time_ms(time: SystemTime) -> Result<u128, String> {
    time.duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .map_err(|error| format!("system clock is before Unix epoch: {error}"))
}

fn usage() -> &'static str {
    "usage:\n  vox-proof [input.srt]\n  vox-proof review <input.srt> <session-terms.txt> <reviewed-output.srt> <decision-log.txt> <session-summary.txt>\n  vox-proof compare <raw-input.srt> <final-input.srt> <comparison-report.json>"
}

fn compare_usage() -> &'static str {
    "usage:\n  vox-proof compare <raw-input.srt> <final-input.srt> <comparison-report.json>"
}

fn experiment_usage() -> &'static str {
    "usage:\n  vox-proof review-experiment <input.srt> <session-terms.txt> <session-description.txt> <rules-only|fake|external-command> <experimental-report.json> <reviewed-output.srt> <decision-log.txt> <session-summary.txt>"
}
