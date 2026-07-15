//! Experiment-only bounded retrieval for contextual-resolution studies.
//!
//! These reports are deliberately separate from canonical `Evidence`,
//! `CandidateSpan`, `ReviewCase`, decisions, and reviewed-output derivation.

use pinyin::{Pinyin, ToPinyin, ToPinyinMulti};
use serde::Serialize;

use crate::anchor::SourceAnchor;
use crate::candidate::SessionTermEntry;
use crate::transcript::Transcript;

pub const RETRIEVAL_VERSION: &str = "experimental-retrieval-0.2";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExperimentalPinyinEligibilityProfile {
    UnfilteredBaselineV1,
    SuppressShortHanToShortUppercaseAcronymV1,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExperimentalRetrievalConfig {
    pub max_candidates_per_window: usize,
    pub max_latin_distance: usize,
    pub max_pinyin_distance: usize,
    pub max_pinyin_expansions: usize,
    pub max_latin_window_tokens: usize,
    pub pinyin_eligibility_profile: ExperimentalPinyinEligibilityProfile,
}

impl Default for ExperimentalRetrievalConfig {
    fn default() -> Self {
        Self {
            max_candidates_per_window: 3,
            max_latin_distance: 3,
            max_pinyin_distance: 3,
            max_pinyin_expansions: 32,
            max_latin_window_tokens: 3,
            pinyin_eligibility_profile:
                ExperimentalPinyinEligibilityProfile::SuppressShortHanToShortUppercaseAcronymV1,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExperimentalCandidateReport {
    pub candidate_id: String,
    pub source_anchor: ExperimentalAnchor,
    pub source_surface: String,
    pub canonical_term: String,
    pub producer: ExperimentalProducer,
    pub producer_version: String,
    pub representation: ExperimentalRepresentation,
    pub normalization_variant: String,
    pub source_representation: String,
    pub target_representation: String,
    pub distance: usize,
    pub ratio_numerator: usize,
    pub ratio_denominator: usize,
    pub skipped_components: Vec<String>,
    pub pinyin: Option<PinyinAuxiliaryDetails>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExperimentalAnchor {
    pub segment_position: usize,
    pub start_byte: usize,
    pub end_byte: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExperimentalProducer {
    LatinNormalizedDistance,
    LatinTokenBoundary,
    HanPinyinAuxiliary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExperimentalRepresentation {
    LatinAlphanumericLowercase,
    LatinTokenBoundary,
    HanPinyinToneless,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PinyinAuxiliaryDetails {
    pub primary_reading: String,
    pub alternate_reading_used: bool,
    pub expansion_bounded: bool,
    pub expansion_truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SourceWindow {
    anchor: SourceAnchor,
    surface: String,
    kind: SourceWindowKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SourceWindowKind {
    Latin,
    Han,
}

/// Retrieves only canonical terms supplied in `entries`. All results are
/// deterministic reports, not reviewable alternatives or edit authority.
pub fn retrieve_experimental_candidates(
    transcript: &Transcript,
    entries: &[SessionTermEntry],
    config: &ExperimentalRetrievalConfig,
) -> Vec<ExperimentalCandidateReport> {
    let mut reports = Vec::new();

    for (segment_position, segment) in transcript.segments().iter().enumerate() {
        for window in latin_windows(
            transcript,
            segment_position,
            &segment.text,
            config.max_latin_window_tokens,
        ) {
            reports.extend(latin_reports(&window, entries, config));
        }
        for window in han_windows(transcript, segment_position, &segment.text) {
            reports.extend(pinyin_reports(&window, entries, config));
        }
    }

    reports.sort_by(|left, right| {
        (
            left.source_anchor.segment_position,
            left.source_anchor.start_byte,
            left.source_anchor.end_byte,
            left.producer,
            left.distance,
            &left.canonical_term,
        )
            .cmp(&(
                right.source_anchor.segment_position,
                right.source_anchor.start_byte,
                right.source_anchor.end_byte,
                right.producer,
                right.distance,
                &right.canonical_term,
            ))
    });

    let mut per_window = std::collections::BTreeMap::<(usize, usize, usize), usize>::new();
    reports.retain(|report| {
        let key = (
            report.source_anchor.segment_position,
            report.source_anchor.start_byte,
            report.source_anchor.end_byte,
        );
        let count = per_window.entry(key).or_default();
        if *count >= config.max_candidates_per_window {
            return false;
        }
        *count += 1;
        true
    });

    for (index, report) in reports.iter_mut().enumerate() {
        report.candidate_id = format!("experimental-{index}");
    }
    reports
}

fn latin_windows(
    transcript: &Transcript,
    segment_position: usize,
    text: &str,
    max_tokens: usize,
) -> Vec<SourceWindow> {
    let tokens = ascii_word_ranges(text);
    let mut windows = Vec::new();
    for start in 0..tokens.len() {
        for length in 1..=max_tokens.min(tokens.len() - start) {
            let first = tokens[start];
            let last = tokens[start + length - 1];
            let start_byte = first.0;
            let end_byte = last.1;
            if !is_latin_window_eligible(&text[start_byte..end_byte]) {
                continue;
            }
            let anchor = transcript
                .anchor(segment_position, start_byte, end_byte)
                .expect("ASCII token bounds are valid UTF-8 anchors");
            windows.push(SourceWindow {
                anchor,
                surface: text[start_byte..end_byte].to_string(),
                kind: SourceWindowKind::Latin,
            });
        }
    }
    windows
}

fn han_windows(transcript: &Transcript, segment_position: usize, text: &str) -> Vec<SourceWindow> {
    let mut windows = Vec::new();
    let mut start = None;
    let mut end = 0;
    for (byte, ch) in text.char_indices() {
        if ch.to_pinyin().is_some() {
            start.get_or_insert(byte);
            end = byte + ch.len_utf8();
        } else if let Some(start_byte) = start.take() {
            let anchor = transcript
                .anchor(segment_position, start_byte, end)
                .expect("Han run bounds are valid UTF-8 anchors");
            windows.push(SourceWindow {
                anchor,
                surface: text[start_byte..end].to_string(),
                kind: SourceWindowKind::Han,
            });
        }
    }
    if let Some(start_byte) = start {
        let anchor = transcript
            .anchor(segment_position, start_byte, end)
            .expect("Han run bounds are valid UTF-8 anchors");
        windows.push(SourceWindow {
            anchor,
            surface: text[start_byte..end].to_string(),
            kind: SourceWindowKind::Han,
        });
    }
    windows
}

fn latin_reports(
    window: &SourceWindow,
    entries: &[SessionTermEntry],
    config: &ExperimentalRetrievalConfig,
) -> Vec<ExperimentalCandidateReport> {
    debug_assert_eq!(window.kind, SourceWindowKind::Latin);
    if !is_latin_window_eligible(&window.surface)
        || ascii_word_ranges(&window.surface)
            .iter()
            .any(|(start, end)| end - start < 2)
    {
        return Vec::new();
    }
    let source = normalize_latin(&window.surface);
    if source.len() < 3 {
        return Vec::new();
    }

    entries
        .iter()
        .filter_map(|entry| {
            if !is_ascii_alphabetic_term(&entry.canonical_term) {
                return None;
            }
            let target = normalize_latin(&entry.canonical_term);
            let distance = levenshtein(&source, &target)?;
            if target.len() < 3
                || source == target
                || distance > config.max_latin_distance
                || distance == source.len().max(target.len())
            {
                return None;
            }
            let representation = if window.surface.split_ascii_whitespace().count() > 1 {
                ExperimentalRepresentation::LatinTokenBoundary
            } else {
                ExperimentalRepresentation::LatinAlphanumericLowercase
            };
            Some(report(
                window,
                entry,
                if representation == ExperimentalRepresentation::LatinTokenBoundary {
                    ExperimentalProducer::LatinTokenBoundary
                } else {
                    ExperimentalProducer::LatinNormalizedDistance
                },
                representation,
                "ascii-alphanumeric-lowercase",
                source.clone(),
                target,
                distance,
                Vec::new(),
                None,
            ))
        })
        .collect()
}

fn pinyin_reports(
    window: &SourceWindow,
    entries: &[SessionTermEntry],
    config: &ExperimentalRetrievalConfig,
) -> Vec<ExperimentalCandidateReport> {
    debug_assert_eq!(window.kind, SourceWindowKind::Han);
    let Some(expansion) = pinyin_expansion(&window.surface, config.max_pinyin_expansions) else {
        return Vec::new();
    };

    entries
        .iter()
        .filter_map(|entry| {
            if suppress_pinyin_candidate(window, entry, config.pinyin_eligibility_profile) {
                return None;
            }
            let target = representation_for_pinyin_target(&entry.canonical_term)?;
            let mut best = None;
            for source in &expansion.readings {
                let Some(distance) = levenshtein(source, &target) else {
                    continue;
                };
                if distance > config.max_pinyin_distance {
                    continue;
                }
                let candidate = (distance, source);
                if best.is_none_or(|current: (usize, &String)| candidate.0 < current.0) {
                    best = Some(candidate);
                }
            }
            let (distance, source) = best?;
            if source == &target {
                return None;
            }
            Some(report(
                window,
                entry,
                ExperimentalProducer::HanPinyinAuxiliary,
                ExperimentalRepresentation::HanPinyinToneless,
                "han-pinyin-toneless-with-bounded-heteronyms",
                source.clone(),
                target,
                distance,
                vec!["tone-not-compared".to_string()],
                Some(PinyinAuxiliaryDetails {
                    primary_reading: expansion.primary.clone(),
                    alternate_reading_used: source != &expansion.primary,
                    expansion_bounded: true,
                    expansion_truncated: expansion.truncated,
                }),
            ))
        })
        .collect()
}

fn suppress_pinyin_candidate(
    window: &SourceWindow,
    entry: &SessionTermEntry,
    profile: ExperimentalPinyinEligibilityProfile,
) -> bool {
    if profile == ExperimentalPinyinEligibilityProfile::UnfilteredBaselineV1 {
        return false;
    }

    let source_length = window.surface.chars().count();
    let target = entry.canonical_term.as_str();
    is_all_han_term(&window.surface)
        && (1..=2).contains(&source_length)
        && !target.is_empty()
        && !target.chars().any(char::is_whitespace)
        && target.chars().all(|ch| ch.is_ascii_uppercase())
        && (1..=3).contains(&target.len())
}

#[allow(clippy::too_many_arguments)]
fn report(
    window: &SourceWindow,
    entry: &SessionTermEntry,
    producer: ExperimentalProducer,
    representation: ExperimentalRepresentation,
    normalization_variant: &str,
    source_representation: String,
    target_representation: String,
    distance: usize,
    skipped_components: Vec<String>,
    pinyin: Option<PinyinAuxiliaryDetails>,
) -> ExperimentalCandidateReport {
    let denominator = source_representation
        .chars()
        .count()
        .max(target_representation.chars().count());
    ExperimentalCandidateReport {
        candidate_id: String::new(),
        source_anchor: ExperimentalAnchor {
            segment_position: window.anchor.segment_position(),
            start_byte: window.anchor.start_byte,
            end_byte: window.anchor.end_byte,
        },
        source_surface: window.surface.clone(),
        canonical_term: entry.canonical_term.clone(),
        producer,
        producer_version: RETRIEVAL_VERSION.to_string(),
        representation,
        normalization_variant: normalization_variant.to_string(),
        source_representation,
        target_representation,
        distance,
        ratio_numerator: denominator.saturating_sub(distance),
        ratio_denominator: denominator,
        skipped_components,
        pinyin,
    }
}

fn ascii_word_ranges(text: &str) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    let mut start = None;
    for (byte, ch) in text.char_indices() {
        if ch.is_ascii_alphabetic() {
            start.get_or_insert(byte);
        } else if let Some(start_byte) = start.take() {
            ranges.push((start_byte, byte));
        }
    }
    if let Some(start_byte) = start {
        ranges.push((start_byte, text.len()));
    }
    ranges
}

fn normalize_latin(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_ascii_alphabetic())
        .flat_map(char::to_lowercase)
        .collect()
}

fn is_latin_window_eligible(value: &str) -> bool {
    value
        .chars()
        .all(|ch| ch.is_ascii_alphabetic() || ch.is_ascii_whitespace())
}

fn is_ascii_alphabetic_term(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphabetic() || ch.is_ascii_whitespace())
        && value.chars().any(|ch| ch.is_ascii_alphabetic())
}

fn is_all_han_term(value: &str) -> bool {
    !value.is_empty() && value.chars().all(|ch| ch.to_pinyin().is_some())
}

fn representation_for_pinyin_target(value: &str) -> Option<String> {
    if is_all_han_term(value) {
        return Some(pinyin_expansion(value, 1)?.primary);
    }
    if is_ascii_alphabetic_term(value) {
        return Some(normalize_latin(value));
    }
    None
}

struct PinyinExpansion {
    primary: String,
    readings: Vec<String>,
    truncated: bool,
}

fn pinyin_expansion(value: &str, limit: usize) -> Option<PinyinExpansion> {
    if !is_all_han_term(value) || limit == 0 {
        return None;
    }
    let mut primary = String::new();
    let mut choices = Vec::new();
    for ch in value.chars() {
        let primary_reading = ch.to_pinyin()?.plain().to_string();
        primary.push_str(&primary_reading);
        let alternatives = ch
            .to_pinyin_multi()?
            .into_iter()
            .map(Pinyin::plain)
            .map(str::to_string);
        choices.push(readings_with_primary_first(primary_reading, alternatives));
    }
    let theoretical = choices
        .iter()
        .fold(1usize, |count, choice| count.saturating_mul(choice.len()));
    let mut readings = Vec::new();
    if !choices.is_empty() {
        expand_readings(&choices, 0, String::new(), limit, &mut readings);
    }
    Some(PinyinExpansion {
        primary,
        truncated: readings.len() < theoretical,
        readings,
    })
}

fn expand_readings(
    choices: &[Vec<String>],
    index: usize,
    prefix: String,
    limit: usize,
    output: &mut Vec<String>,
) {
    if output.len() == limit {
        return;
    }
    if index == choices.len() {
        output.push(prefix);
        return;
    }
    for value in &choices[index] {
        let mut next = prefix.clone();
        next.push_str(value);
        expand_readings(choices, index + 1, next, limit, output);
        if output.len() == limit {
            return;
        }
    }
}

fn readings_with_primary_first(
    primary: String,
    alternatives: impl Iterator<Item = String>,
) -> Vec<String> {
    let mut readings = vec![primary.clone()];
    for alternative in alternatives {
        if alternative != primary && !readings.contains(&alternative) {
            readings.push(alternative);
        }
    }
    readings
}

fn levenshtein(left: &str, right: &str) -> Option<usize> {
    if left.is_empty() || right.is_empty() {
        return None;
    }
    let right = right.chars().collect::<Vec<_>>();
    let mut previous = (0..=right.len()).collect::<Vec<_>>();
    let mut current = vec![0; right.len() + 1];
    for (index, left_ch) in left.chars().enumerate() {
        current[0] = index + 1;
        for (right_index, right_ch) in right.iter().enumerate() {
            current[right_index + 1] = (previous[right_index] + usize::from(left_ch != *right_ch))
                .min(current[right_index] + 1)
                .min(previous[right_index + 1] + 1);
        }
        std::mem::swap(&mut previous, &mut current);
    }
    Some(previous[right.len()])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::run_term_review;
    use crate::srt::parse_srt;

    fn entry(canonical: &str) -> SessionTermEntry {
        SessionTermEntry::new(canonical, Vec::new(), Vec::new())
    }

    #[test]
    fn retrieval_is_deterministic_and_is_limited_to_session_terms() {
        let transcript = parse_srt("1\n00:00:00,000 --> 00:00:01,000\nPostgre sequel").unwrap();
        let entries = vec![entry("PostgreSQL"), entry("not-in-session")];
        let config = ExperimentalRetrievalConfig::default();
        let first = retrieve_experimental_candidates(&transcript, &entries, &config);
        let second = retrieve_experimental_candidates(&transcript, &entries, &config);
        assert_eq!(first, second);
        assert!(
            first
                .iter()
                .all(|report| report.canonical_term == "PostgreSQL")
        );
        assert!(
            first
                .iter()
                .all(|report| report.source_anchor.end_byte <= 14)
        );
    }

    #[test]
    fn pinyin_auxiliary_is_bounded_and_keeps_anchor_details() {
        let transcript = parse_srt("1\n00:00:00,000 --> 00:00:01,000\n卡夫卡").unwrap();
        let reports = retrieve_experimental_candidates(
            &transcript,
            &[entry("Kafka")],
            &ExperimentalRetrievalConfig::default(),
        );
        let report = reports
            .iter()
            .find(|report| report.producer == ExperimentalProducer::HanPinyinAuxiliary)
            .unwrap();
        assert_eq!(report.source_surface, "卡夫卡");
        assert_eq!(report.source_anchor.start_byte, 0);
        assert_eq!(report.source_anchor.end_byte, 9);
        assert!(report.pinyin.is_some());
    }

    #[test]
    fn symbols_require_explicit_exact_aliases_not_non_exact_retrieval() {
        let transcript = parse_srt("1\n00:00:00,000 --> 00:00:01,000\nC plus plus").unwrap();
        let reports = retrieve_experimental_candidates(
            &transcript,
            &[entry("C++")],
            &ExperimentalRetrievalConfig::default(),
        );
        assert!(reports.is_empty());
    }

    #[test]
    fn candidate_cap_applies_to_each_source_window() {
        let transcript = parse_srt("1\n00:00:00,000 --> 00:00:01,000\nKafk").unwrap();
        let entries = vec![
            entry("Kafka"),
            entry("Kappa"),
            entry("Kafla"),
            entry("Kafra"),
        ];
        let config = ExperimentalRetrievalConfig {
            max_candidates_per_window: 2,
            ..ExperimentalRetrievalConfig::default()
        };
        assert_eq!(
            retrieve_experimental_candidates(&transcript, &entries, &config).len(),
            2
        );
    }

    #[test]
    fn pinyin_primary_precedes_order_preserved_alternates() {
        let expansion = pinyin_expansion("重", 16).expect("Han has pinyin readings");
        assert_eq!(expansion.primary, "zhong");
        assert_eq!(
            expansion.readings.first().map(String::as_str),
            Some("zhong")
        );
        assert_eq!(expansion.readings, vec!["zhong", "chong", "tong"]);
    }

    #[test]
    fn latin_windows_do_not_cross_incompatible_content() {
        let transcript = parse_srt("1\n00:00:00,000 --> 00:00:01,000\nfoo 卡 bar").unwrap();
        let windows = latin_windows(&transcript, 0, "foo 卡 bar", 3);
        assert_eq!(
            windows
                .iter()
                .map(|window| window.surface.as_str())
                .collect::<Vec<_>>(),
            vec!["foo", "bar"]
        );
    }

    #[test]
    fn non_exact_producers_reject_symbols_digits_and_mixed_pinyin_targets() {
        let transcript = parse_srt("1\n00:00:00,000 --> 00:00:01,000\n卡夫卡").unwrap();
        let reports = retrieve_experimental_candidates(
            &transcript,
            &[
                entry("K8s"),
                entry("C++"),
                entry("C#"),
                entry("澀谷 Sky"),
                entry("Kafka"),
            ],
            &ExperimentalRetrievalConfig::default(),
        );
        assert_eq!(
            reports
                .iter()
                .map(|report| report.canonical_term.as_str())
                .collect::<Vec<_>>(),
            vec!["Kafka"]
        );
    }

    #[test]
    fn unfiltered_profile_retains_short_han_to_short_acronym_baseline() {
        let transcript = parse_srt("1\n00:00:00,000 --> 00:00:01,000\n對").unwrap();
        let config = ExperimentalRetrievalConfig {
            pinyin_eligibility_profile: ExperimentalPinyinEligibilityProfile::UnfilteredBaselineV1,
            ..ExperimentalRetrievalConfig::default()
        };
        let reports = retrieve_experimental_candidates(&transcript, &[entry("AI")], &config);
        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].source_surface, "對");
        assert_eq!(reports[0].canonical_term, "AI");
    }

    #[test]
    fn default_profile_suppresses_short_han_to_short_acronym() {
        let transcript = parse_srt("1\n00:00:00,000 --> 00:00:01,000\n對 嗯 會").unwrap();
        let reports = retrieve_experimental_candidates(
            &transcript,
            &[entry("AI")],
            &ExperimentalRetrievalConfig::default(),
        );
        assert!(reports.is_empty());
    }

    #[test]
    fn default_profile_retains_longer_han_to_latin_transliterations() {
        for (source, target) in [("卡夫卡", "Kafka"), ("阿里佩", "Alipay")] {
            let transcript =
                parse_srt(&format!("1\n00:00:00,000 --> 00:00:01,000\n{source}")).unwrap();
            let reports = retrieve_experimental_candidates(
                &transcript,
                &[entry(target)],
                &ExperimentalRetrievalConfig::default(),
            );
            assert!(
                reports.iter().any(|report| report.canonical_term == target),
                "expected {source} -> {target} to remain eligible"
            );
        }
    }

    #[test]
    fn exact_alias_path_is_unaffected_by_experimental_profile() {
        let transcript = parse_srt("1\n00:00:00,000 --> 00:00:01,000\n對").unwrap();
        let entries = vec![SessionTermEntry::new(
            "AI",
            vec!["對".to_string()],
            Vec::new(),
        )];
        let review_cases = run_term_review(&transcript, &entries).unwrap();
        assert_eq!(review_cases.len(), 1);

        let reports = retrieve_experimental_candidates(
            &transcript,
            &entries,
            &ExperimentalRetrievalConfig::default(),
        );
        assert!(reports.is_empty());
    }

    #[test]
    fn profile_specific_retrieval_is_deterministic() {
        let transcript = parse_srt("1\n00:00:00,000 --> 00:00:01,000\n卡夫卡 對").unwrap();
        let entries = vec![entry("Kafka"), entry("AI")];
        for profile in [
            ExperimentalPinyinEligibilityProfile::UnfilteredBaselineV1,
            ExperimentalPinyinEligibilityProfile::SuppressShortHanToShortUppercaseAcronymV1,
        ] {
            let config = ExperimentalRetrievalConfig {
                pinyin_eligibility_profile: profile,
                ..ExperimentalRetrievalConfig::default()
            };
            assert_eq!(
                retrieve_experimental_candidates(&transcript, &entries, &config),
                retrieve_experimental_candidates(&transcript, &entries, &config)
            );
        }
    }
}
