use crate::transcript::{Segment, Transcript};

#[derive(Debug, PartialEq, Eq)]
pub enum ParseError {
    MalformedIndex { block: usize, found: String },
    MissingTiming { block: usize },
    MalformedTiming { block: usize, found: String },
}

pub fn parse_srt(input: &str) -> Result<Transcript, ParseError> {
    let mut segments = Vec::new();

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

        segments.push(Segment {
            index,
            start_ms,
            end_ms,
            text,
        });
    }

    Ok(Transcript::from_segments(segments))
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
    let hours = parts.next()?;
    let minutes = parts.next()?;
    let seconds = parts.next()?;
    if parts.next().is_some() {
        return None;
    }

    if !is_ascii_digits_at_least(hours, 2)
        || !is_ascii_digits_exactly(minutes, 2)
        || !is_ascii_digits_exactly(seconds, 2)
    {
        return None;
    }

    let hours: u64 = hours.parse().ok()?;
    let minutes: u64 = minutes.parse().ok()?;
    let seconds: u64 = seconds.parse().ok()?;
    if minutes > 59 || seconds > 59 {
        return None;
    }

    hours
        .checked_mul(60)?
        .checked_add(minutes)?
        .checked_mul(60)?
        .checked_add(seconds)?
        .checked_mul(1000)?
        .checked_add(millis)
}

fn is_ascii_digits_at_least(value: &str, min_len: usize) -> bool {
    value.len() >= min_len && value.bytes().all(|b| b.is_ascii_digit())
}

fn is_ascii_digits_exactly(value: &str, len: usize) -> bool {
    value.len() == len && value.bytes().all(|b| b.is_ascii_digit())
}
