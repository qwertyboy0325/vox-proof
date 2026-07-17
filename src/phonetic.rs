use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use rphonetic::DoubleMetaphone;

use crate::analysis::AnalysisRun;
use crate::anchor::SourceAnchor;
use crate::candidate::{
    AsciiLatinPhoneticRepresentation, CANONICAL_SESSION_TERM_ALGORITHM,
    CANONICAL_SESSION_TERM_DETECTOR_CONFIG, CandidateAlternative, CandidateSpan, DetectionError,
    DetectionKind, DetectorProvenance, Evidence, PHONETIC_DETECTOR, PhoneticComparisonFacts,
    PhoneticSimilarityEvidence, PhoneticTargetKind, SessionTermEntry, validate_detection_inputs,
};
use crate::transcript::Transcript;

pub(crate) const MIN_TOKEN_LEN: usize = 2;
pub(crate) const MAX_TOKEN_LEN: usize = 32;
pub(crate) const MAX_WINDOW_TOKENS: usize = 3;
pub(crate) const MIN_NORMALIZED_LEN: usize = 3;
pub(crate) const MAX_NORMALIZED_LEN: usize = 64;
pub(crate) const MAX_ANCHOR_BYTES: usize = 96;
pub(crate) const MIN_QUALIFYING_PERMILLE: usize = 500;

#[derive(Debug, Clone, PartialEq, Eq)]
struct TokenSpan {
    start_byte: usize,
    end_byte: usize,
    surface: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct StructuralSurface {
    tokens: Vec<String>,
    normalized_letters: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SourceWindow {
    anchor: SourceAnchor,
    surface: String,
    tokens: Vec<String>,
    normalized_letters: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PhoneticTarget {
    surface: String,
    tokens: Vec<String>,
    normalized_letters: String,
    kind: PhoneticTargetKind,
    canonical_term: String,
    entry: SessionTermEntry,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct QualifiedMatch {
    target: PhoneticTarget,
    comparison: PhoneticComparisonFacts,
    source_representation: AsciiLatinPhoneticRepresentation,
    target_representation: AsciiLatinPhoneticRepresentation,
}

/// Finds bounded ASCII-Latin phonetic similarity between cue-local source
/// windows and session-term canonical terms or aliases. Observed error forms
/// are never phonetic targets. Multiple qualifying canonical owners for one
/// anchor are suppressed without emitting a candidate.
pub fn detect_ascii_latin_phonetic_matches(
    run: &AnalysisRun,
    transcript: &Transcript,
    entries: &[SessionTermEntry],
) -> Result<Vec<CandidateSpan>, DetectionError> {
    validate_detection_inputs(run, transcript, entries)?;

    let targets = build_phonetic_targets(entries);
    let exact_token_vectors = exact_suppression_token_vectors(&targets);
    let provenance = DetectorProvenance::from_detector_identity(PHONETIC_DETECTOR);
    let mut spans = Vec::new();

    for (position, segment) in transcript.segments().iter().enumerate() {
        for window in source_windows(transcript, position, &segment.text) {
            if exact_token_vectors.contains(&window.tokens) {
                continue;
            }

            let Some(source_representation) = ascii_representation(&window.normalized_letters)
            else {
                continue;
            };

            let mut qualified_by_owner: BTreeMap<String, QualifiedMatch> = BTreeMap::new();

            for target in &targets {
                if window.tokens == target.tokens {
                    continue;
                }

                let Some(target_representation) = ascii_representation(&target.normalized_letters)
                else {
                    continue;
                };

                let Some(comparison) = compare_representations(
                    &window.normalized_letters,
                    &target.normalized_letters,
                    &source_representation,
                    &target_representation,
                ) else {
                    continue;
                };

                let candidate = QualifiedMatch {
                    target: target.clone(),
                    comparison,
                    source_representation: source_representation.clone(),
                    target_representation: target_representation.clone(),
                };

                match qualified_by_owner.get(&target.canonical_term) {
                    None => {
                        qualified_by_owner.insert(target.canonical_term.clone(), candidate);
                    }
                    Some(existing) => {
                        if same_owner_match_preferred(&candidate, existing) {
                            qualified_by_owner.insert(target.canonical_term.clone(), candidate);
                        }
                    }
                }
            }

            if qualified_by_owner.len() != 1 {
                continue;
            }

            let winner = qualified_by_owner
                .into_values()
                .next()
                .expect("exactly one owner");
            let evidence = Evidence::PhoneticSimilarity(PhoneticSimilarityEvidence {
                observed_surface: window.surface.clone(),
                target_surface: winner.target.surface.clone(),
                target_kind: winner.target.kind,
                canonical_term: winner.target.canonical_term.clone(),
                source_representation: winner.source_representation,
                target_representation: winner.target_representation,
                comparison: winner.comparison,
                detector_config: CANONICAL_SESSION_TERM_DETECTOR_CONFIG,
                algorithm: CANONICAL_SESSION_TERM_ALGORITHM,
            });

            spans.push(CandidateSpan::new(
                DetectionKind::PhoneticSimilarity,
                provenance.clone(),
                window.anchor,
                evidence,
                vec![CandidateAlternative::new(
                    winner.target.canonical_term.clone(),
                )],
            ));
        }
    }

    Ok(spans)
}

fn build_phonetic_targets(entries: &[SessionTermEntry]) -> Vec<PhoneticTarget> {
    let mut targets = Vec::new();

    for entry in entries {
        if let Some(target) = phonetic_target_from_surface(
            &entry.canonical_term,
            PhoneticTargetKind::CanonicalTerm,
            entry,
        ) {
            targets.push(target);
        }

        for alias in &entry.aliases {
            if let Some(target) =
                phonetic_target_from_surface(alias, PhoneticTargetKind::Alias, entry)
            {
                targets.push(target);
            }
        }
    }

    collapse_same_owner_duplicates(targets)
}

fn collapse_same_owner_duplicates(targets: Vec<PhoneticTarget>) -> Vec<PhoneticTarget> {
    let mut groups: BTreeMap<(String, String), Vec<PhoneticTarget>> = BTreeMap::new();

    for target in targets {
        groups
            .entry((
                target.canonical_term.clone(),
                target.normalized_letters.clone(),
            ))
            .or_default()
            .push(target);
    }

    let mut collapsed = Vec::new();
    for (_, mut group) in groups {
        group.sort_by(same_owner_duplicate_ordering);
        collapsed.push(
            group
                .into_iter()
                .next()
                .expect("duplicate groups are non-empty"),
        );
    }

    collapsed.sort_by(|left, right| {
        (
            left.canonical_term.as_str(),
            left.normalized_letters.as_str(),
            target_kind_rank(left.kind),
            lowercase_ascii(&left.surface).as_str(),
            left.surface.as_str(),
        )
            .cmp(&(
                right.canonical_term.as_str(),
                right.normalized_letters.as_str(),
                target_kind_rank(right.kind),
                lowercase_ascii(&right.surface).as_str(),
                right.surface.as_str(),
            ))
    });

    collapsed
}

fn same_owner_duplicate_ordering(left: &PhoneticTarget, right: &PhoneticTarget) -> Ordering {
    target_kind_rank(left.kind)
        .cmp(&target_kind_rank(right.kind))
        .then_with(|| lowercase_ascii(&left.surface).cmp(&lowercase_ascii(&right.surface)))
        .then_with(|| left.surface.cmp(&right.surface))
}

fn phonetic_target_from_surface(
    surface: &str,
    kind: PhoneticTargetKind,
    entry: &SessionTermEntry,
) -> Option<PhoneticTarget> {
    let structural = parse_structural_ascii_latin_surface(surface)?;

    Some(PhoneticTarget {
        surface: surface.to_string(),
        tokens: structural.tokens,
        normalized_letters: structural.normalized_letters,
        kind,
        canonical_term: entry.canonical_term.clone(),
        entry: entry.clone(),
    })
}

fn parse_structural_ascii_latin_surface(surface: &str) -> Option<StructuralSurface> {
    if surface.is_empty() || surface.len() > MAX_ANCHOR_BYTES || !surface.is_ascii() {
        return None;
    }

    let bytes = surface.as_bytes();
    if bytes.first().is_some_and(|byte| byte.is_ascii_whitespace())
        || bytes.last().is_some_and(|byte| byte.is_ascii_whitespace())
    {
        return None;
    }

    let mut tokens = Vec::new();
    let mut pos = 0;

    while pos < bytes.len() {
        if !bytes[pos].is_ascii_alphabetic() {
            return None;
        }

        let start = pos;
        while pos < bytes.len() && bytes[pos].is_ascii_alphabetic() {
            pos += 1;
        }

        let token_len = pos - start;
        if !token_length_eligible(token_len) {
            return None;
        }

        tokens.push(lowercase_ascii(&surface[start..pos]));
        if tokens.len() > MAX_WINDOW_TOKENS {
            return None;
        }

        if pos == bytes.len() {
            break;
        }

        if !bytes[pos].is_ascii_whitespace() {
            return None;
        }

        while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
            pos += 1;
        }

        if pos >= bytes.len() {
            return None;
        }
    }

    if tokens.is_empty() {
        return None;
    }

    let normalized_letters = tokens
        .iter()
        .flat_map(|token| token.chars())
        .collect::<String>();
    if !normalized_length_eligible(normalized_letters.len()) {
        return None;
    }

    Some(StructuralSurface {
        tokens,
        normalized_letters,
    })
}

fn exact_suppression_token_vectors(targets: &[PhoneticTarget]) -> BTreeSet<Vec<String>> {
    targets.iter().map(|target| target.tokens.clone()).collect()
}

fn source_windows(
    transcript: &Transcript,
    segment_position: usize,
    text: &str,
) -> Vec<SourceWindow> {
    let tokens = eligible_tokens(text);
    let mut windows = Vec::new();

    for start in 0..tokens.len() {
        for length in 1..=MAX_WINDOW_TOKENS.min(tokens.len() - start) {
            let slice = &tokens[start..start + length];
            if !tokens_separated_only_by_ascii_whitespace(text, slice) {
                continue;
            }

            let start_byte = slice.first().expect("non-empty slice").start_byte;
            let end_byte = slice.last().expect("non-empty slice").end_byte;
            let surface = text[start_byte..end_byte].to_string();

            let Some(structural) = parse_structural_ascii_latin_surface(&surface) else {
                continue;
            };

            let anchor = transcript
                .anchor(segment_position, start_byte, end_byte)
                .expect("ASCII token bounds are valid UTF-8 anchors");

            windows.push(SourceWindow {
                anchor,
                surface,
                tokens: structural.tokens,
                normalized_letters: structural.normalized_letters,
            });
        }
    }

    windows
}

fn eligible_tokens(text: &str) -> Vec<TokenSpan> {
    let mut tokens = Vec::new();
    let mut start = None;

    for (byte, ch) in text.char_indices() {
        if ch.is_ascii_alphabetic() {
            start.get_or_insert(byte);
        } else if let Some(start_byte) = start.take() {
            push_token_if_eligible(text, start_byte, byte, &mut tokens);
        }
    }

    if let Some(start_byte) = start {
        push_token_if_eligible(text, start_byte, text.len(), &mut tokens);
    }

    tokens
}

fn push_token_if_eligible(
    text: &str,
    start_byte: usize,
    end_byte: usize,
    tokens: &mut Vec<TokenSpan>,
) {
    let surface = text[start_byte..end_byte].to_string();
    if token_length_eligible(surface.len()) {
        tokens.push(TokenSpan {
            start_byte,
            end_byte,
            surface,
        });
    }
}

fn tokens_separated_only_by_ascii_whitespace(text: &str, tokens: &[TokenSpan]) -> bool {
    for pair in tokens.windows(2) {
        let between = &text[pair[0].end_byte..pair[1].start_byte];
        if !between
            .as_bytes()
            .iter()
            .all(|byte| byte.is_ascii_whitespace())
        {
            return false;
        }
    }
    true
}

fn lowercase_ascii(value: &str) -> String {
    value.to_ascii_lowercase()
}

fn token_length_eligible(len: usize) -> bool {
    (MIN_TOKEN_LEN..=MAX_TOKEN_LEN).contains(&len)
}

fn normalized_length_eligible(len: usize) -> bool {
    (MIN_NORMALIZED_LEN..=MAX_NORMALIZED_LEN).contains(&len)
}

fn ascii_representation(normalized_letters: &str) -> Option<AsciiLatinPhoneticRepresentation> {
    if normalized_letters.is_empty()
        || !normalized_letters.is_ascii()
        || !normalized_letters
            .bytes()
            .all(|byte| byte.is_ascii_lowercase())
        || !normalized_length_eligible(normalized_letters.len())
    {
        return None;
    }

    let result = DoubleMetaphone::new(None).double_metaphone(normalized_letters);
    Some(AsciiLatinPhoneticRepresentation {
        normalized_letters: normalized_letters.to_string(),
        primary_key: result.primary(),
        alternate_key: result.alternate(),
    })
}

fn compare_representations(
    source_normalized: &str,
    target_normalized: &str,
    source_representation: &AsciiLatinPhoneticRepresentation,
    target_representation: &AsciiLatinPhoneticRepresentation,
) -> Option<PhoneticComparisonFacts> {
    let matched_keys = deterministic_matched_keys(source_representation, target_representation);
    if matched_keys.is_empty() {
        return None;
    }

    let distance = levenshtein_bytes(source_normalized.as_bytes(), target_normalized.as_bytes());
    let denominator = source_normalized.len().max(target_normalized.len());
    let numerator = denominator.saturating_sub(distance);
    let permille = numerator * 1000 / denominator;
    if permille < MIN_QUALIFYING_PERMILLE {
        return None;
    }

    Some(PhoneticComparisonFacts {
        edit_distance: distance,
        ratio_numerator: numerator,
        ratio_denominator: denominator,
        ratio_permille: permille,
        matched_key: matched_keys[0].clone(),
    })
}

pub(crate) fn matched_key_overlap(source_keys: &[&str], target_keys: &[&str]) -> Vec<String> {
    let mut matched = BTreeSet::new();

    for source_key in source_keys {
        for target_key in target_keys {
            if !source_key.is_empty() && source_key == target_key {
                matched.insert((*source_key).to_string());
            }
        }
    }

    matched.into_iter().collect()
}

pub(crate) fn deterministic_matched_keys(
    source: &AsciiLatinPhoneticRepresentation,
    target: &AsciiLatinPhoneticRepresentation,
) -> Vec<String> {
    matched_key_overlap(
        &[&source.primary_key, &source.alternate_key],
        &[&target.primary_key, &target.alternate_key],
    )
}

fn levenshtein_bytes(left: &[u8], right: &[u8]) -> usize {
    let mut previous = (0..=right.len()).collect::<Vec<_>>();
    let mut current = vec![0; right.len() + 1];

    for (left_index, left_value) in left.iter().enumerate() {
        current[0] = left_index + 1;
        for (right_index, right_value) in right.iter().enumerate() {
            let substitution = previous[right_index] + usize::from(left_value != right_value);
            let insertion = current[right_index] + 1;
            let deletion = previous[right_index + 1] + 1;
            current[right_index + 1] = substitution.min(insertion).min(deletion);
        }
        std::mem::swap(&mut previous, &mut current);
    }

    previous[right.len()]
}

fn same_owner_match_ordering(
    value: &QualifiedMatch,
) -> (std::cmp::Reverse<usize>, usize, u8, String, String) {
    (
        std::cmp::Reverse(value.comparison.ratio_permille),
        value.comparison.edit_distance,
        target_kind_rank(value.target.kind),
        lowercase_ascii(&value.target.surface),
        value.target.surface.clone(),
    )
}

fn compare_same_owner_match(left: &QualifiedMatch, right: &QualifiedMatch) -> Ordering {
    same_owner_match_ordering(left).cmp(&same_owner_match_ordering(right))
}

fn same_owner_match_preferred(candidate: &QualifiedMatch, existing: &QualifiedMatch) -> bool {
    compare_same_owner_match(candidate, existing) == Ordering::Less
}

fn target_kind_rank(kind: PhoneticTargetKind) -> u8 {
    match kind {
        PhoneticTargetKind::CanonicalTerm => 0,
        PhoneticTargetKind::Alias => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::{
        AlgorithmIdentity, AnalysisConfigurationIdentity, AnalysisRun,
        CanonicalDetectorSetIdentity, DetectorConfigIdentity,
    };
    use crate::candidate::{
        CANONICAL_SESSION_TERM_ALGORITHM, CANONICAL_SESSION_TERM_ANALYSIS_IDENTITY,
        CANONICAL_SESSION_TERM_DETECTOR_CONFIG, CANONICAL_SESSION_TERM_DETECTOR_SET,
        GLOSSARY_DETECTOR, OBSERVED_ERROR_FORM_DETECTOR, PHONETIC_DETECTOR,
        detect_glossary_matches, detect_observed_error_form_matches,
    };
    use crate::pipeline::run_term_review;
    use crate::srt::parse_srt;

    fn entry(canonical: &str, aliases: &[&str], errors: &[&str]) -> SessionTermEntry {
        SessionTermEntry::new(
            canonical,
            aliases.iter().map(|value| (*value).to_string()).collect(),
            errors.iter().map(|value| (*value).to_string()).collect(),
        )
    }

    fn transcript(text: &str) -> crate::transcript::Transcript {
        parse_srt(&format!("1\n00:00:00,000 --> 00:00:01,000\n{text}")).expect("valid srt")
    }

    fn run(
        transcript: &crate::transcript::Transcript,
        entries: &[SessionTermEntry],
    ) -> AnalysisRun {
        AnalysisRun::for_canonical_session_terms(transcript, entries)
    }

    fn phonetic_spans(
        transcript: &crate::transcript::Transcript,
        entries: &[SessionTermEntry],
    ) -> Vec<CandidateSpan> {
        detect_ascii_latin_phonetic_matches(&run(transcript, entries), transcript, entries)
            .expect("valid configuration")
    }

    fn winner_for(
        transcript: &crate::transcript::Transcript,
        entries: &[SessionTermEntry],
    ) -> CandidateSpan {
        phonetic_spans(transcript, entries)
            .into_iter()
            .next()
            .expect("expected one phonetic span")
    }

    fn phonetic_evidence(span: &CandidateSpan) -> &PhoneticSimilarityEvidence {
        match span.evidence() {
            Evidence::PhoneticSimilarity(evidence) => evidence,
            _ => panic!("expected phonetic evidence"),
        }
    }

    fn qualified_match(
        permille: usize,
        distance: usize,
        kind: PhoneticTargetKind,
        surface: &str,
        canonical_term: &str,
    ) -> QualifiedMatch {
        QualifiedMatch {
            target: PhoneticTarget {
                surface: surface.to_string(),
                tokens: parse_structural_ascii_latin_surface(surface)
                    .expect("valid structural surface")
                    .tokens,
                normalized_letters: parse_structural_ascii_latin_surface(surface)
                    .expect("valid structural surface")
                    .normalized_letters,
                kind,
                canonical_term: canonical_term.to_string(),
                entry: entry(canonical_term, &[], &[]),
            },
            comparison: PhoneticComparisonFacts {
                edit_distance: distance,
                ratio_numerator: 1,
                ratio_denominator: 1,
                ratio_permille: permille,
                matched_key: "KEY".to_string(),
            },
            source_representation: AsciiLatinPhoneticRepresentation {
                normalized_letters: "src".to_string(),
                primary_key: "KEY".to_string(),
                alternate_key: String::new(),
            },
            target_representation: AsciiLatinPhoneticRepresentation {
                normalized_letters: "tgt".to_string(),
                primary_key: "KEY".to_string(),
                alternate_key: String::new(),
            },
        }
    }

    fn assert_anchor_uses_ascii_whitespace_only(text: &str, span: &CandidateSpan) {
        let anchor = span.anchor();
        let segment = &text[anchor.start_byte..anchor.end_byte];
        assert!(
            segment.is_ascii(),
            "anchor surface must remain ASCII-only: {segment:?}"
        );
        for window in segment.as_bytes().windows(2) {
            if window[0].is_ascii_alphabetic() && window[1].is_ascii_alphabetic() {
                continue;
            }
            if window[0].is_ascii_whitespace() || window[1].is_ascii_whitespace() {
                assert!(
                    window[0].is_ascii_whitespace() && window[1].is_ascii_whitespace()
                        || window[0].is_ascii_alphabetic()
                        || window[1].is_ascii_alphabetic(),
                    "non-ASCII separator byte in anchor: {segment:?}"
                );
            }
        }
        assert!(
            !segment
                .as_bytes()
                .iter()
                .any(|byte| !byte.is_ascii_alphabetic() && !byte.is_ascii_whitespace()),
            "anchor contains non-ASCII separator bytes: {segment:?}"
        );
    }

    #[test]
    fn positive_postgre_sequel_matches_postgresql() {
        let entries = [entry("PostgreSQL", &[], &[])];
        let transcript = transcript("Postgre sequel");
        let span = winner_for(&transcript, &entries);
        let evidence = phonetic_evidence(&span);

        assert_eq!(evidence.canonical_term, "PostgreSQL");
        assert_eq!(evidence.comparison.ratio_permille, 769);
        assert_eq!(evidence.comparison.matched_key, "PSTKRSKL");
    }

    #[test]
    fn positive_postgre_sql_matches_postgresql() {
        let entries = [entry("PostgreSQL", &[], &[])];
        let transcript = transcript("Postgre SQL");
        let span = winner_for(&transcript, &entries);
        let evidence = phonetic_evidence(&span);

        assert_eq!(evidence.comparison.ratio_permille, 1000);
        assert_eq!(evidence.comparison.matched_key, "PSTKRSKL");
    }

    #[test]
    fn positive_cuber_netties_matches_kubernetes() {
        let entries = [entry("Kubernetes", &[], &[])];
        let transcript = transcript("cuber netties");
        let span = winner_for(&transcript, &entries);
        let evidence = phonetic_evidence(&span);

        assert_eq!(evidence.comparison.ratio_permille, 750);
        assert_eq!(evidence.comparison.matched_key, "KPRNTS");
    }

    #[test]
    fn positive_sequel_matches_sql() {
        let entries = [entry("SQL", &[], &[])];
        let transcript = transcript("sequel");
        let span = winner_for(&transcript, &entries);
        let evidence = phonetic_evidence(&span);

        assert_eq!(evidence.comparison.ratio_permille, 500);
        assert_eq!(evidence.comparison.matched_key, "SKL");
    }

    #[test]
    fn negative_scale_does_not_match_sql() {
        let entries = [entry("SQL", &[], &[])];
        let transcript = transcript("scale");
        assert!(phonetic_spans(&transcript, &entries).is_empty());
    }

    #[test]
    fn negative_happy_does_not_match_api() {
        let entries = [entry("API", &[], &[])];
        let transcript = transcript("happy");
        assert!(phonetic_spans(&transcript, &entries).is_empty());
    }

    #[test]
    fn negative_postal_does_not_match_postgresql() {
        let entries = [entry("PostgreSQL", &[], &[])];
        let transcript = transcript("postal");
        assert!(phonetic_spans(&transcript, &entries).is_empty());
    }

    #[test]
    fn negative_copper_does_not_match_kubernetes() {
        let entries = [entry("Kubernetes", &[], &[])];
        let transcript = transcript("copper");
        assert!(phonetic_spans(&transcript, &entries).is_empty());
    }

    #[test]
    fn exact_canonical_occurrence_is_suppressed() {
        let entries = [entry("PostgreSQL", &[], &[])];
        let transcript = transcript("PostgreSQL");
        assert!(phonetic_spans(&transcript, &entries).is_empty());
    }

    #[test]
    fn exact_alias_occurrence_is_suppressed() {
        let entries = [entry("PostgreSQL", &["Postgres"], &[])];
        let transcript = transcript("Postgres");
        assert!(phonetic_spans(&transcript, &entries).is_empty());
    }

    #[test]
    fn case_only_exact_suppression() {
        let entries = [entry("PostgreSQL", &[], &[])];
        let transcript = transcript("postgresql");
        assert!(phonetic_spans(&transcript, &entries).is_empty());
    }

    #[test]
    fn ascii_whitespace_only_exact_suppression() {
        let entries = [entry("XY ZW", &["XY ZW"], &[])];
        let transcript = transcript("XY  ZW");
        assert!(phonetic_spans(&transcript, &entries).is_empty());
    }

    #[test]
    fn alias_target_maps_to_canonical_owner() {
        let entries = [entry("PostgreSQL", &["Postgres"], &[])];
        let transcript = transcript("post grass");
        let span = winner_for(&transcript, &entries);
        let evidence = phonetic_evidence(&span);

        assert_eq!(evidence.target_kind, PhoneticTargetKind::Alias);
        assert_eq!(evidence.target_surface, "Postgres");
        assert_eq!(evidence.canonical_term, "PostgreSQL");
        assert_eq!(span.alternatives()[0].replacement_text(), "PostgreSQL");
    }

    #[test]
    fn observed_error_forms_are_never_targets() {
        let entries = [entry("SQL", &[], &["sequel"])];
        let transcript = transcript("sequel");
        let span = winner_for(&transcript, &entries);
        let evidence = phonetic_evidence(&span);

        assert_eq!(evidence.target_kind, PhoneticTargetKind::CanonicalTerm);
        assert_eq!(evidence.target_surface, "SQL");
        assert_eq!(evidence.canonical_term, "SQL");
    }

    #[test]
    fn split_acronym_and_symbol_inputs_are_ineligible() {
        let entries = [entry("SQL", &[], &[])];
        for text in ["S Q L", "C#", "C++", "K8s", "🙂"] {
            let transcript = transcript(text);
            assert!(
                phonetic_spans(&transcript, &entries).is_empty(),
                "expected ineligible source: {text}"
            );
        }
    }

    #[test]
    fn mixed_utf8_ascii_substring_runs_phonetic_path_without_panic() {
        let entries = [entry("PostgreSQL", &[], &[])];
        let text = "資料 Postgre sequel 測試";
        let transcript = transcript(text);
        let spans = phonetic_spans(&transcript, &entries);
        assert_eq!(spans.len(), 1);

        let span = &spans[0];
        let evidence = phonetic_evidence(span);
        let resolved = transcript
            .resolve(span.anchor())
            .expect("anchor resolves against transcript");
        let anchor = span.anchor();

        assert_eq!(resolved, "Postgre sequel");
        assert_eq!(&text[anchor.start_byte..anchor.end_byte], "Postgre sequel");
        assert!(text[..anchor.start_byte].ends_with(' '));
        assert!(text[anchor.end_byte..].starts_with(' '));
        assert!(!text[..anchor.start_byte].is_ascii());
        assert!(!text[anchor.end_byte..].is_ascii());
        assert_eq!(evidence.observed_surface, "Postgre sequel");
        assert_eq!(evidence.target_surface, "PostgreSQL");
        assert_eq!(evidence.comparison.matched_key, "PSTKRSKL");
        assert_anchor_uses_ascii_whitespace_only(text, span);
    }

    #[test]
    fn ambiguity_across_multiple_owners_is_suppressed() {
        let entries = [entry("SQL", &[], &[]), entry("Skull", &[], &[])];
        let transcript = transcript("sequel");
        assert!(phonetic_spans(&transcript, &entries).is_empty());
    }

    #[test]
    fn ascii_space_is_valid_token_separator() {
        let entries = [entry("PostgreSQL", &["Postgre SQL"], &[])];
        let transcript = transcript("postgre sequel");
        assert!(!phonetic_spans(&transcript, &entries).is_empty());
    }

    #[test]
    fn nbsp_is_hard_boundary_for_source_windows() {
        let entries = [entry("PostgreSQL", &[], &[])];
        let transcript = transcript("Postgre\u{00a0}sequel");
        assert!(phonetic_spans(&transcript, &entries).is_empty());
    }

    #[test]
    fn invalid_target_surfaces_are_excluded() {
        let entries = [entry(
            "PostgreSQL",
            &[
                "資料SQL",
                "SQL!",
                "A PostgreSQL",
                "K8s",
                " PostgreSQL",
                "PostgreSQL ",
                "foo\u{00a0}bar",
            ],
            &[],
        )];

        let surfaces = build_phonetic_targets(&entries)
            .into_iter()
            .map(|target| target.surface)
            .collect::<Vec<_>>();

        assert_eq!(surfaces, vec!["PostgreSQL".to_string()]);
    }

    #[test]
    fn same_owner_duplicate_collapse_prefers_canonical_over_alias() {
        let entries = [entry(
            "TestCanonical",
            &["intervening", "testcanonical"],
            &[],
        )];
        let surfaces = build_phonetic_targets(&entries)
            .into_iter()
            .filter(|target| target.normalized_letters == "testcanonical")
            .map(|target| (target.kind, target.surface))
            .collect::<Vec<_>>();

        assert_eq!(
            surfaces,
            vec![(
                PhoneticTargetKind::CanonicalTerm,
                "TestCanonical".to_string()
            )]
        );
    }

    #[test]
    fn same_owner_duplicate_collapse_is_input_order_independent() {
        let first = [
            entry("AlphaTerm", &["alphaterm"], &[]),
            entry("BetaTerm", &["betaterm"], &[]),
        ];
        let second = [first[1].clone(), first[0].clone()];

        assert_eq!(
            build_phonetic_targets(&first),
            build_phonetic_targets(&second)
        );
    }

    #[test]
    fn cross_owner_normalized_duplicate_remains_ambiguous() {
        let entries = [entry("PostgreSQL", &[], &[]), entry("postgresql", &[], &[])];
        let transcript = transcript("postgre sequel");
        assert!(phonetic_spans(&transcript, &entries).is_empty());
    }

    #[test]
    fn same_owner_winner_prefers_higher_permille() {
        let existing = qualified_match(700, 2, PhoneticTargetKind::CanonicalTerm, "Alpha", "Alpha");
        let candidate = qualified_match(800, 2, PhoneticTargetKind::Alias, "Beta", "Alpha");
        assert!(same_owner_match_preferred(&candidate, &existing));
    }

    #[test]
    fn same_owner_winner_prefers_smaller_edit_distance() {
        let existing = qualified_match(800, 4, PhoneticTargetKind::CanonicalTerm, "Alpha", "Alpha");
        let candidate = qualified_match(800, 2, PhoneticTargetKind::Alias, "Beta", "Alpha");
        assert!(same_owner_match_preferred(&candidate, &existing));
    }

    #[test]
    fn same_owner_winner_prefers_canonical_term_kind() {
        let existing = qualified_match(800, 2, PhoneticTargetKind::Alias, "Alpha", "Alpha");
        let candidate =
            qualified_match(800, 2, PhoneticTargetKind::CanonicalTerm, "Alpha", "Alpha");
        assert!(same_owner_match_preferred(&candidate, &existing));
    }

    #[test]
    fn same_owner_winner_prefers_smaller_lowercase_surface() {
        let existing = qualified_match(800, 2, PhoneticTargetKind::Alias, "aaBB", "Alpha");
        let candidate = qualified_match(800, 2, PhoneticTargetKind::Alias, "aaBa", "Alpha");
        assert!(lowercase_ascii("aaBa") < lowercase_ascii("aaBB"));
        assert!("aaBB" < "aaBa");
        assert!(same_owner_match_preferred(&candidate, &existing));
    }

    #[test]
    fn same_owner_winner_prefers_smaller_original_surface() {
        let existing = qualified_match(800, 2, PhoneticTargetKind::Alias, "Betaa", "Alpha");
        let candidate = qualified_match(800, 2, PhoneticTargetKind::Alias, "BetaA", "Alpha");
        assert!(same_owner_match_preferred(&candidate, &existing));
    }

    #[test]
    fn matched_key_is_lexicographically_first_distinct_overlap() {
        let source = AsciiLatinPhoneticRepresentation {
            normalized_letters: "sequel".to_string(),
            primary_key: "SKL".to_string(),
            alternate_key: "SKL".to_string(),
        };
        let target = AsciiLatinPhoneticRepresentation {
            normalized_letters: "sql".to_string(),
            primary_key: "SKL".to_string(),
            alternate_key: "SQL".to_string(),
        };

        assert_eq!(
            deterministic_matched_keys(&source, &target),
            vec!["SKL".to_string()]
        );
    }

    #[test]
    fn multiple_distinct_overlaps_select_lexicographic_first_key() {
        let source = AsciiLatinPhoneticRepresentation {
            normalized_letters: "abc".to_string(),
            primary_key: "B".to_string(),
            alternate_key: "A".to_string(),
        };
        let target = AsciiLatinPhoneticRepresentation {
            normalized_letters: "abd".to_string(),
            primary_key: "A".to_string(),
            alternate_key: "B".to_string(),
        };

        assert_eq!(
            deterministic_matched_keys(&source, &target),
            vec!["A".to_string(), "B".to_string()]
        );
        assert_eq!(
            compare_representations("abc", "abd", &source, &target)
                .expect("keys overlap")
                .matched_key,
            "A"
        );
    }

    #[test]
    fn matched_key_selection_is_independent_of_key_collection_order() {
        let source = AsciiLatinPhoneticRepresentation {
            normalized_letters: "abc".to_string(),
            primary_key: "B".to_string(),
            alternate_key: "A".to_string(),
        };
        let target = AsciiLatinPhoneticRepresentation {
            normalized_letters: "abd".to_string(),
            primary_key: "A".to_string(),
            alternate_key: "B".to_string(),
        };

        let forward = matched_key_overlap(&["B", "A"], &["A", "B"]);
        let reversed = matched_key_overlap(&["A", "B"], &["B", "A"]);
        assert_eq!(forward, reversed);
        assert_eq!(forward, vec!["A".to_string(), "B".to_string()]);

        let forward_evidence =
            compare_representations("abc", "abd", &source, &target).expect("forward evidence");
        let reversed_source = AsciiLatinPhoneticRepresentation {
            primary_key: source.alternate_key.clone(),
            alternate_key: source.primary_key.clone(),
            ..source.clone()
        };
        let reversed_target = AsciiLatinPhoneticRepresentation {
            primary_key: target.alternate_key.clone(),
            alternate_key: target.primary_key.clone(),
            ..target.clone()
        };
        let reversed_evidence =
            compare_representations("abc", "abd", &reversed_source, &reversed_target)
                .expect("reversed evidence");

        assert_eq!(forward_evidence.matched_key, reversed_evidence.matched_key);
        assert_eq!(forward_evidence, reversed_evidence);
    }

    #[test]
    fn ascii_representation_rejects_non_lowercase_ascii() {
        assert!(ascii_representation("Postgre").is_none());
        assert!(ascii_representation("postgre").is_some());
    }

    #[test]
    fn structural_surface_accepts_token_length_two_in_multi_token_surface() {
        assert!(parse_structural_ascii_latin_surface("ab cd").is_some());
    }

    #[test]
    fn structural_surface_enforces_per_token_length_boundaries() {
        assert!(parse_structural_ascii_latin_surface(&"a".repeat(32)).is_some());
        assert!(parse_structural_ascii_latin_surface(&"a".repeat(33)).is_none());
    }

    #[test]
    fn structural_surface_rejects_four_token_windows() {
        assert!(parse_structural_ascii_latin_surface("aa bb cc dd").is_none());
        assert!(parse_structural_ascii_latin_surface("aa bb cc").is_some());
    }

    #[test]
    fn structural_surface_enforces_normalized_length_boundaries() {
        assert!(parse_structural_ascii_latin_surface("ab").is_none());
        assert!(parse_structural_ascii_latin_surface("abc").is_some());

        let normalized_64 = format!("{} {}", "a".repeat(32), "a".repeat(32));
        assert_eq!(normalized_64.len(), 65);
        let parsed_64 = parse_structural_ascii_latin_surface(&normalized_64).expect("64 letters");
        assert_eq!(parsed_64.normalized_letters.len(), 64);
        assert_eq!(parsed_64.tokens.len(), 2);

        let normalized_65 = format!("{} {} {}", "a".repeat(32), "a".repeat(31), "aa");
        assert_eq!(normalized_65.len(), 67);
        assert_eq!(
            normalized_65
                .chars()
                .filter(|ch| ch.is_ascii_alphabetic())
                .count(),
            65
        );
        assert!(parse_structural_ascii_latin_surface(&normalized_65).is_none());
    }

    #[test]
    fn structural_surface_enforces_anchor_byte_boundaries() {
        let within = format!(
            "{}{}{}{}{}",
            "a".repeat(32),
            " ".repeat(16),
            "a".repeat(16),
            " ".repeat(16),
            "a".repeat(16),
        );
        assert_eq!(within.len(), MAX_ANCHOR_BYTES);
        assert!(parse_structural_ascii_latin_surface(&within).is_some());

        let over = format!(
            "{}{}{}{}{}",
            "a".repeat(32),
            " ".repeat(17),
            "a".repeat(16),
            " ".repeat(16),
            "a".repeat(16),
        );
        assert_eq!(over.len(), MAX_ANCHOR_BYTES + 1);
        assert!(parse_structural_ascii_latin_surface(&over).is_none());
    }

    #[test]
    fn score_boundary_retains_exact_500_case() {
        let entries = [entry("SQL", &[], &[])];
        let transcript = transcript("sequel");
        let span = winner_for(&transcript, &entries);
        let evidence = phonetic_evidence(&span);
        assert_eq!(evidence.comparison.ratio_permille, 500);
    }

    #[test]
    fn score_boundary_rejects_closest_below_threshold() {
        let source_norm = format!("{}{}", "a".repeat(31), "b".repeat(32));
        let target_norm = "a".repeat(63);
        assert_eq!(
            levenshtein_bytes(source_norm.as_bytes(), target_norm.as_bytes()),
            32
        );
        assert_eq!(31 * 1000 / 63, 492);

        let source_rep = AsciiLatinPhoneticRepresentation {
            normalized_letters: source_norm.clone(),
            primary_key: "OVL".to_string(),
            alternate_key: String::new(),
        };
        let target_rep = AsciiLatinPhoneticRepresentation {
            normalized_letters: target_norm.clone(),
            primary_key: "OVL".to_string(),
            alternate_key: String::new(),
        };

        assert!(
            compare_representations(&source_norm, &target_norm, &source_rep, &target_rep).is_none()
        );
    }

    #[test]
    fn detection_identity_mismatch_cases_fail_closed() {
        let review_transcript = transcript("Postgre sequel");
        let entries = [entry("PostgreSQL", &[], &[])];

        let cases = [
            (
                "revision",
                AnalysisRun::for_canonical_session_terms(&transcript("other cue"), &entries),
            ),
            (
                "session terms",
                AnalysisRun::new(
                    &review_transcript,
                    &[entry("Other", &[], &[])],
                    CANONICAL_SESSION_TERM_ANALYSIS_IDENTITY,
                ),
            ),
            (
                "detector set",
                AnalysisRun::new(
                    &review_transcript,
                    &entries,
                    AnalysisConfigurationIdentity::new(
                        CanonicalDetectorSetIdentity::new(&[GLOSSARY_DETECTOR]),
                        CANONICAL_SESSION_TERM_DETECTOR_CONFIG,
                        CANONICAL_SESSION_TERM_ALGORITHM,
                    ),
                ),
            ),
            (
                "detector config",
                AnalysisRun::new(
                    &review_transcript,
                    &entries,
                    AnalysisConfigurationIdentity::new(
                        CANONICAL_SESSION_TERM_DETECTOR_SET,
                        DetectorConfigIdentity::new("exact-case-sensitive-cue-local", "0.2.0"),
                        CANONICAL_SESSION_TERM_ALGORITHM,
                    ),
                ),
            ),
            (
                "algorithm",
                AnalysisRun::new(
                    &review_transcript,
                    &entries,
                    AnalysisConfigurationIdentity::new(
                        CANONICAL_SESSION_TERM_DETECTOR_SET,
                        CANONICAL_SESSION_TERM_DETECTOR_CONFIG,
                        AlgorithmIdentity::new("rust-str-match-indices", "2"),
                    ),
                ),
            ),
        ];

        for (label, bad_run) in cases {
            let result =
                detect_ascii_latin_phonetic_matches(&bad_run, &review_transcript, &entries);
            assert!(result.is_err(), "expected {label} mismatch to fail closed");
        }
    }

    #[test]
    fn canonical_review_cases_remain_order_independent() {
        let transcript = transcript("Postgre sequel then Postgres");
        let first = [
            entry("PostgreSQL", &["Postgres"], &[]),
            entry("SQL", &[], &[]),
        ];
        let second = [first[1].clone(), first[0].clone()];

        let first_cases = run_term_review(&transcript, &first).expect("valid");
        let second_cases = run_term_review(&transcript, &second).expect("valid");
        assert_eq!(first_cases, second_cases);
    }

    #[test]
    fn exact_detectors_remain_unchanged_with_phonetic_profile() {
        let transcript = transcript("Postgre SQL then Postgres");
        let entries = [entry("PostgreSQL", &["Postgres"], &["Postgre SQL"])];
        let analysis_run = run(&transcript, &entries);

        let glossary =
            detect_glossary_matches(&analysis_run, &transcript, &entries).expect("valid glossary");
        let observed = detect_observed_error_form_matches(&analysis_run, &transcript, &entries)
            .expect("valid observed");

        assert_eq!(glossary.len(), 1);
        assert_eq!(observed.len(), 1);
        assert_eq!(
            analysis_run.snapshot().configuration(),
            CANONICAL_SESSION_TERM_ANALYSIS_IDENTITY
        );
    }

    #[test]
    fn active_detector_set_includes_phonetic_identity_in_order() {
        use crate::candidate::CANONICAL_SESSION_TERM_DETECTORS;

        assert_eq!(
            CANONICAL_SESSION_TERM_DETECTORS,
            &[
                GLOSSARY_DETECTOR,
                OBSERVED_ERROR_FORM_DETECTOR,
                PHONETIC_DETECTOR
            ]
        );
    }
}
