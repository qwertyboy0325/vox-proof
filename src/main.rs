use std::io::{self, BufRead, Read, Write};
use std::process::ExitCode;

use vox_proof::candidate::{Evidence, GlossaryEntry};
use vox_proof::pipeline::run_glossary_review;
use vox_proof::review::{CorrectionDecision, ReviewCase, ReviewLedger};
use vox_proof::reviewed_output::derive_reviewed_srt;
use vox_proof::session_log::render_decision_log;
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
    if args.len() != 4 {
        return Err(usage().to_string());
    }

    let stdin = io::stdin();
    let stdout = io::stdout();
    run_review_command(&args[1], &args[2], &args[3], stdin.lock(), stdout.lock())
}

fn run_review_command<R: BufRead, W: Write>(
    input_path: &str,
    reviewed_output_path: &str,
    decision_log_path: &str,
    input: R,
    mut output: W,
) -> Result<(), String> {
    let input_srt = std::fs::read_to_string(input_path)
        .map_err(|error| format!("failed to read input SRT: {error}"))?;
    let transcript =
        parse_srt(&input_srt).map_err(|error| format!("failed to parse SRT: {error:?}"))?;

    print_parse_summary(&transcript, &mut output).map_err(|error| error.to_string())?;

    let glossary = demo_glossary();
    writeln!(
        output,
        "using temporary demo glossary: Kafka -> Apache Kafka, Postgres -> PostgreSQL"
    )
    .map_err(|error| error.to_string())?;

    let review_cases = run_glossary_review(&transcript, &glossary)
        .map_err(|error| format!("failed to run glossary review: {error:?}"))?;
    let ledger = if review_cases.is_empty() {
        writeln!(output, "no review cases found").map_err(|error| error.to_string())?;
        ReviewLedger::new()
    } else {
        review_cases_interactively(&transcript, &review_cases, input, &mut output)?
    };

    let reviewed_srt = derive_reviewed_srt(&transcript, &review_cases, &ledger)
        .map_err(|error| format!("failed to derive reviewed SRT: {error:?}"))?;
    let decision_log = render_decision_log(&ledger);

    std::fs::write(reviewed_output_path, reviewed_srt)
        .map_err(|error| format!("failed to write reviewed SRT: {error}"))?;
    std::fs::write(decision_log_path, decision_log)
        .map_err(|error| format!("failed to write decision log: {error}"))?;

    writeln!(output, "wrote reviewed SRT: {reviewed_output_path}")
        .map_err(|error| error.to_string())?;
    writeln!(output, "wrote decision log: {decision_log_path}")
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
        Evidence::Glossary(evidence) => {
            writeln!(
                output,
                "evidence: glossary alias '{}' for '{}'",
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

fn demo_glossary() -> Vec<GlossaryEntry> {
    // Temporary demo glossary for the facilitated v0.1 CLI loop. This is not a
    // product glossary system and intentionally avoids file/config parsing.
    vec![
        GlossaryEntry::new("Apache Kafka", vec!["Kafka".to_string()]),
        GlossaryEntry::new("PostgreSQL", vec!["Postgres".to_string()]),
    ]
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

fn usage() -> &'static str {
    "usage:\n  vox-proof [input.srt]\n  vox-proof review <input.srt> <reviewed-output.srt> <decision-log.txt>"
}
