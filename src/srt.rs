use crate::transcript::{Segment, Transcript};

#[derive(Debug, PartialEq, Eq)]
pub enum ParseError {
    MalformedIndex { block: usize, found: String },
    MissingTiming { block: usize },
    MalformedTiming { block: usize, found: String },
}

pub fn parse_srt(input: &str) -> Result<Transcript, ParseError> {
    let mut transcript = Transcript::new();

    for (position, raw_block) in split_into_blocks(input).into_iter().enumerate() {
        let block_number = position + 1;

        let mut lines = raw_block.into_iter();
        let index_line = lines.next().expect("blocks are never empty");

        let index: u32 = index_line
            .trim()
            .parse()
            .map_err(|_| ParseError::MalformedIndex {
                block: block_number,
                found: index_line.to_string(),
            })?;

        let timing_line = lines.next().ok_or(ParseError::MissingTiming {
            block: block_number,
        })?;

        let (start_ms, end_ms) =
            parse_timing(timing_line).ok_or_else(|| ParseError::MalformedTiming {
                block: block_number,
                found: timing_line.to_string(),
            })?;

        let text = lines.collect::<Vec<&str>>().join("\n");

        transcript.add_segment(Segment {
            index,
            start_ms,
            end_ms,
            text,
        });
    }

    Ok(transcript)
}

fn split_into_blocks(input: &str) -> Vec<Vec<&str>> {
    let mut blocks = Vec::new();
    let mut current: Vec<&str> = Vec::new();

    for line in input.lines() {
        if line.trim().is_empty() {
            if !current.is_empty() {
                blocks.push(std::mem::take(&mut current));
            }
        } else {
            current.push(line);
        }
    }

    if !current.is_empty() {
        blocks.push(current);
    }

    blocks
}

fn parse_timing(line: &str) -> Option<(u64, u64)> {
    let (start, end) = line.split_once("-->")?;
    let start_ms = parse_timestamp(start.trim())?;
    let end_ms = parse_timestamp(end.trim())?;
    Some((start_ms, end_ms))
}

fn parse_timestamp(value: &str) -> Option<u64> {
    let (clock, millis) = value.split_once(',')?;
    if millis.len() != 3 || !millis.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let millis: u64 = millis.parse().ok()?;

    let mut parts = clock.split(':');
    let hours: u64 = parts.next()?.parse().ok()?;
    let minutes: u64 = parts.next()?.parse().ok()?;
    let seconds: u64 = parts.next()?.parse().ok()?;
    if parts.next().is_some() {
        return None;
    }

    Some(((hours * 60 + minutes) * 60 + seconds) * 1000 + millis)
}
