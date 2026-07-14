use std::io::{self, BufRead, Read, Write};
use std::process::ExitCode;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use vox_proof::candidate::Evidence;
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
    let result = if args.first().map(String::as_str) == Some("review") {
        run_review_from_args(&args)
    } else {
        run_parse_command(&args)
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
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

fn print_review_case<W: Write>(
    transcript: &Transcript,
    review_case: &ReviewCase,
    output: &mut W,
) -> io::Result<()> {
    let candidate = review_case.candidate_span();
    let anchor = candidate.anchor();
    let segment_position = anchor.segment_position();
    let segment = transcript.segments().get(segment_position);
    let matched_text = transcript.resolve(anchor).unwrap_or("<unresolved anchor>");

    writeln!(output)?;
    writeln!(output, "case_id: local:{}", review_case.id().local_index())?;
    writeln!(output, "source_segment_position: {segment_position}")?;
    if let Some(segment) = segment {
        writeln!(output, "cue_index: {}", segment.index())?;
        writeln!(output, "cue_text: {}", segment.text())?;
    }
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
    "usage:\n  vox-proof [input.srt]\n  vox-proof review <input.srt> <session-terms.txt> <reviewed-output.srt> <decision-log.txt> <session-summary.txt>"
}
