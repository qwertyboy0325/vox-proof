use std::io::Read;
use std::process::ExitCode;

use vox_proof::srt::parse_srt;

fn main() -> ExitCode {
    let input = match read_input() {
        Ok(text) => text,
        Err(error) => {
            eprintln!("failed to read input: {error}");
            return ExitCode::FAILURE;
        }
    };

    let transcript = match parse_srt(&input) {
        Ok(transcript) => transcript,
        Err(error) => {
            eprintln!("failed to parse SRT: {error:?}");
            return ExitCode::FAILURE;
        }
    };

    println!("parsed {} segment(s)", transcript.segments().len());

    let issues = transcript.validation_issues();
    if issues.is_empty() {
        println!("no validation issues");
    } else {
        println!("{} validation issue(s):", issues.len());
        for issue in &issues {
            println!(
                "  segment position {} (cue index {}): {:?}",
                issue.segment_position(),
                issue.cue_index(),
                issue.error()
            );
        }
    }

    ExitCode::SUCCESS
}

fn read_input() -> std::io::Result<String> {
    match std::env::args().nth(1) {
        Some(path) => std::fs::read_to_string(path),
        None => {
            let mut buffer = String::new();
            std::io::stdin().read_to_string(&mut buffer)?;
            Ok(buffer)
        }
    }
}
