use sha2::{Digest, Sha256};

use crate::anchor::TranscriptRevisionId;
use crate::candidate::SessionTermEntry;
use crate::transcript::Transcript;

/// The effective inputs and configuration under which analysis was
/// performed. These identities bind behavior-affecting inputs without
/// turning the snapshot into a scheduler, registry, or persistence schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SessionTermsIdentity([u8; 32]);

impl SessionTermsIdentity {
    pub fn from_entries(entries: &[SessionTermEntry]) -> Self {
        const DOMAIN_SEPARATOR: &[u8] = b"voxproof-session-terms-identity-v1";

        let mut hasher = Sha256::new();
        hasher.update(DOMAIN_SEPARATOR);
        hash_len(&mut hasher, entries.len());

        for entry in entries {
            hash_string(&mut hasher, &entry.canonical_term);

            hash_len(&mut hasher, entry.aliases.len());
            for alias in &entry.aliases {
                hash_string(&mut hasher, alias);
            }

            hash_len(&mut hasher, entry.observed_error_forms.len());
            for observed_error_form in &entry.observed_error_forms {
                hash_string(&mut hasher, observed_error_form);
            }
        }

        Self(hasher.finalize().into())
    }
}

fn hash_len(hasher: &mut Sha256, len: usize) {
    hasher.update((len as u64).to_le_bytes());
}

fn hash_string(hasher: &mut Sha256, value: &str) {
    hash_len(hasher, value.len());
    hasher.update(value.as_bytes());
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DetectorIdentity {
    id: &'static str,
    version: &'static str,
}

impl DetectorIdentity {
    pub(crate) const fn new(id: &'static str, version: &'static str) -> Self {
        Self { id, version }
    }

    pub const fn id(self) -> &'static str {
        self.id
    }

    pub const fn version(self) -> &'static str {
        self.version
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CanonicalDetectorSetIdentity {
    detectors: &'static [DetectorIdentity],
}

impl CanonicalDetectorSetIdentity {
    pub(crate) const fn new(detectors: &'static [DetectorIdentity]) -> Self {
        Self { detectors }
    }

    pub const fn detectors(self) -> &'static [DetectorIdentity] {
        self.detectors
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DetectorConfigIdentity {
    id: &'static str,
    version: &'static str,
}

impl DetectorConfigIdentity {
    pub(crate) const fn new(id: &'static str, version: &'static str) -> Self {
        Self { id, version }
    }

    pub const fn id(self) -> &'static str {
        self.id
    }

    pub const fn version(self) -> &'static str {
        self.version
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AlgorithmIdentity {
    id: &'static str,
    version: &'static str,
}

impl AlgorithmIdentity {
    pub(crate) const fn new(id: &'static str, version: &'static str) -> Self {
        Self { id, version }
    }

    pub const fn id(self) -> &'static str {
        self.id
    }

    pub const fn version(self) -> &'static str {
        self.version
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AnalysisConfigurationIdentity {
    detector_set: CanonicalDetectorSetIdentity,
    detector_config: DetectorConfigIdentity,
    algorithm: AlgorithmIdentity,
}

impl AnalysisConfigurationIdentity {
    pub(crate) const fn new(
        detector_set: CanonicalDetectorSetIdentity,
        detector_config: DetectorConfigIdentity,
        algorithm: AlgorithmIdentity,
    ) -> Self {
        Self {
            detector_set,
            detector_config,
            algorithm,
        }
    }

    pub const fn detector_set(self) -> CanonicalDetectorSetIdentity {
        self.detector_set
    }

    pub const fn detector_config(self) -> DetectorConfigIdentity {
        self.detector_config
    }

    pub const fn algorithm(self) -> AlgorithmIdentity {
        self.algorithm
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AnalysisSnapshot {
    source_revision: TranscriptRevisionId,
    session_terms: SessionTermsIdentity,
    configuration: AnalysisConfigurationIdentity,
}

impl AnalysisSnapshot {
    pub fn source_revision(&self) -> TranscriptRevisionId {
        self.source_revision
    }

    pub fn session_terms(&self) -> SessionTermsIdentity {
        self.session_terms
    }

    pub fn configuration(&self) -> AnalysisConfigurationIdentity {
        self.configuration
    }
}

/// One bounded analysis execution over one transcript revision under one
/// effective `AnalysisSnapshot`. `AnalysisRun` is a provenance and
/// reproducibility boundary: it does not schedule, persist, or run
/// detectors itself. Detectors are called with a run and must verify the
/// run's snapshot matches the transcript actually being analyzed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnalysisRun {
    snapshot: AnalysisSnapshot,
}

impl AnalysisRun {
    pub fn for_canonical_session_terms(
        transcript: &Transcript,
        entries: &[SessionTermEntry],
    ) -> Self {
        Self {
            snapshot: AnalysisSnapshot {
                source_revision: transcript.revision_id(),
                session_terms: SessionTermsIdentity::from_entries(entries),
                configuration: crate::candidate::canonical_session_term_analysis_identity(),
            },
        }
    }

    #[cfg(test)]
    pub(crate) fn new(
        transcript: &Transcript,
        entries: &[SessionTermEntry],
        configuration: AnalysisConfigurationIdentity,
    ) -> Self {
        Self {
            snapshot: AnalysisSnapshot {
                source_revision: transcript.revision_id(),
                session_terms: SessionTermsIdentity::from_entries(entries),
                configuration,
            },
        }
    }

    pub fn snapshot(&self) -> AnalysisSnapshot {
        self.snapshot
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::candidate::{
        CANONICAL_SESSION_TERM_ALGORITHM, CANONICAL_SESSION_TERM_ANALYSIS_IDENTITY,
        CANONICAL_SESSION_TERM_DETECTOR_CONFIG, CANONICAL_SESSION_TERM_DETECTOR_SET,
        CANONICAL_SESSION_TERM_DETECTORS, GLOSSARY_DETECTOR, OBSERVED_ERROR_FORM_DETECTOR,
        PHONETIC_DETECTOR,
    };
    use crate::srt::parse_srt;

    fn entry(
        canonical_term: &str,
        aliases: &[&str],
        observed_error_forms: &[&str],
    ) -> SessionTermEntry {
        SessionTermEntry::new(
            canonical_term,
            aliases.iter().map(|value| (*value).to_string()).collect(),
            observed_error_forms
                .iter()
                .map(|value| (*value).to_string())
                .collect(),
        )
    }

    fn transcript(text: &str) -> Transcript {
        parse_srt(&format!("1\n00:00:00,000 --> 00:00:01,000\n{text}")).expect("valid SRT")
    }

    #[test]
    fn equal_effective_inputs_produce_equal_snapshots() {
        let transcript = transcript("Kafka");
        let entries = [entry("Apache Kafka", &["Kafka"], &["卡夫卡"])];

        let first = AnalysisRun::for_canonical_session_terms(&transcript, &entries);
        let second = AnalysisRun::for_canonical_session_terms(&transcript, &entries);

        assert_eq!(first.snapshot(), second.snapshot());
    }

    #[test]
    fn canonical_only_session_term_identity_is_deterministic() {
        let entries = [entry("ASUS", &[], &[])];

        assert_eq!(
            SessionTermsIdentity::from_entries(&entries),
            SessionTermsIdentity::from_entries(&entries)
        );
    }

    #[test]
    fn canonical_only_and_self_alias_have_distinct_session_term_identities() {
        let canonical_only = [entry("ASUS", &[], &[])];
        let self_alias = [entry("ASUS", &["ASUS"], &[])];

        assert_ne!(
            SessionTermsIdentity::from_entries(&canonical_only),
            SessionTermsIdentity::from_entries(&self_alias)
        );
    }

    #[test]
    fn canonical_only_entry_order_changes_session_term_identity() {
        let first = [entry("ASUS", &[], &[]), entry("Microsoft", &[], &[])];
        let second = [first[1].clone(), first[0].clone()];

        assert_ne!(
            SessionTermsIdentity::from_entries(&first),
            SessionTermsIdentity::from_entries(&second)
        );
    }

    #[test]
    fn canonical_only_entries_bind_into_analysis_snapshot() {
        let transcript = transcript("ASIS");
        let entries = [entry("ASUS", &[], &[])];
        let run = AnalysisRun::for_canonical_session_terms(&transcript, &entries);

        assert_eq!(
            run.snapshot().session_terms(),
            SessionTermsIdentity::from_entries(&entries)
        );
    }

    #[test]
    fn transcript_change_changes_snapshot() {
        let entries = [entry("Apache Kafka", &["Kafka"], &[])];

        let first = AnalysisRun::for_canonical_session_terms(&transcript("Kafka"), &entries);
        let second = AnalysisRun::for_canonical_session_terms(&transcript("Postgres"), &entries);

        assert_ne!(first.snapshot(), second.snapshot());
    }

    #[test]
    fn canonical_term_alias_and_error_form_changes_change_session_term_identity() {
        let base = [entry("Apache Kafka", &["Kafka"], &["卡夫卡"])];
        let canonical_changed = [entry("Kafka", &["Kafka"], &["卡夫卡"])];
        let alias_changed = [entry("Apache Kafka", &["Kaf ka"], &["卡夫卡"])];
        let error_changed = [entry("Apache Kafka", &["Kafka"], &["卡夫 卡"])];

        let base_identity = SessionTermsIdentity::from_entries(&base);

        assert_ne!(
            base_identity,
            SessionTermsIdentity::from_entries(&canonical_changed)
        );
        assert_ne!(
            base_identity,
            SessionTermsIdentity::from_entries(&alias_changed)
        );
        assert_ne!(
            base_identity,
            SessionTermsIdentity::from_entries(&error_changed)
        );
    }

    #[test]
    fn entry_order_only_change_changes_session_term_identity() {
        let first = [
            entry("Apache Kafka", &["Kafka"], &[]),
            entry("PostgreSQL", &["Postgres"], &[]),
        ];
        let second = [first[1].clone(), first[0].clone()];

        assert_ne!(
            SessionTermsIdentity::from_entries(&first),
            SessionTermsIdentity::from_entries(&second)
        );
    }

    #[test]
    fn alias_order_only_change_changes_session_term_identity() {
        let first = [entry("PostgreSQL", &["Postgres", "Postgre SQL"], &[])];
        let second = [entry("PostgreSQL", &["Postgre SQL", "Postgres"], &[])];

        assert_ne!(
            SessionTermsIdentity::from_entries(&first),
            SessionTermsIdentity::from_entries(&second)
        );
    }

    #[test]
    fn observed_error_order_only_change_changes_session_term_identity() {
        let first = [entry(
            "PostgreSQL",
            &[],
            &["post grass", "post gray sequel"],
        )];
        let second = [entry(
            "PostgreSQL",
            &[],
            &["post gray sequel", "post grass"],
        )];

        assert_ne!(
            SessionTermsIdentity::from_entries(&first),
            SessionTermsIdentity::from_entries(&second)
        );
    }

    #[test]
    fn session_term_identity_distinguishes_length_prefix_concatenation_ambiguity() {
        let single_alias = [entry("Term", &["ab"], &[])];
        let split_aliases = [entry("Term", &["a", "b"], &[])];

        assert_ne!(
            SessionTermsIdentity::from_entries(&single_alias),
            SessionTermsIdentity::from_entries(&split_aliases)
        );
    }

    #[test]
    fn session_term_identity_binds_entry_and_source_form_order() {
        let first = [
            entry(
                "PostgreSQL",
                &["Postgres", "Postgre SQL"],
                &["post grass", "post gray sequel"],
            ),
            entry("Apache Kafka", &["Kafka", "Kafka API"], &["卡夫卡"]),
        ];
        let second = [
            entry("Apache Kafka", &["Kafka API", "Kafka"], &["卡夫卡"]),
            entry(
                "PostgreSQL",
                &["Postgre SQL", "Postgres"],
                &["post gray sequel", "post grass"],
            ),
        ];

        assert_ne!(
            SessionTermsIdentity::from_entries(&first),
            SessionTermsIdentity::from_entries(&second)
        );
    }

    #[test]
    fn detector_version_change_structurally_changes_detector_set_identity() {
        const CHANGED_GLOSSARY: DetectorIdentity =
            DetectorIdentity::new("glossary-alias-match", "0.2.0");
        const CHANGED_DETECTORS: &[DetectorIdentity] = &[
            CHANGED_GLOSSARY,
            OBSERVED_ERROR_FORM_DETECTOR,
            PHONETIC_DETECTOR,
        ];
        const CHANGED_SET: CanonicalDetectorSetIdentity =
            CanonicalDetectorSetIdentity::new(CHANGED_DETECTORS);

        assert_ne!(CANONICAL_SESSION_TERM_DETECTOR_SET, CHANGED_SET);
        assert_eq!(
            CANONICAL_SESSION_TERM_DETECTOR_SET.detectors(),
            CANONICAL_SESSION_TERM_DETECTORS
        );
    }

    #[test]
    fn canonical_session_term_detector_set_order_is_deterministic() {
        assert_eq!(
            CANONICAL_SESSION_TERM_DETECTOR_SET.detectors(),
            &[
                GLOSSARY_DETECTOR,
                OBSERVED_ERROR_FORM_DETECTOR,
                PHONETIC_DETECTOR
            ]
        );
    }

    #[test]
    fn detector_set_config_and_algorithm_changes_change_snapshot() {
        const CHANGED_GLOSSARY: DetectorIdentity =
            DetectorIdentity::new("glossary-alias-match", "0.2.0");
        const CHANGED_DETECTORS: &[DetectorIdentity] = &[CHANGED_GLOSSARY];
        const CHANGED_DETECTOR_SET: CanonicalDetectorSetIdentity =
            CanonicalDetectorSetIdentity::new(CHANGED_DETECTORS);

        let transcript = transcript("Kafka");
        let entries = [entry("Apache Kafka", &["Kafka"], &[])];
        let base = AnalysisRun::for_canonical_session_terms(&transcript, &entries).snapshot();

        let detector_set_changed = AnalysisConfigurationIdentity::new(
            CHANGED_DETECTOR_SET,
            CANONICAL_SESSION_TERM_DETECTOR_CONFIG,
            CANONICAL_SESSION_TERM_ALGORITHM,
        );
        let config_changed = AnalysisConfigurationIdentity::new(
            CANONICAL_SESSION_TERM_DETECTOR_SET,
            DetectorConfigIdentity::new("exact-case-sensitive-cue-local", "0.2.0"),
            CANONICAL_SESSION_TERM_ALGORITHM,
        );
        let algorithm_changed = AnalysisConfigurationIdentity::new(
            CANONICAL_SESSION_TERM_DETECTOR_SET,
            CANONICAL_SESSION_TERM_DETECTOR_CONFIG,
            AlgorithmIdentity::new("rust-str-match-indices", "2"),
        );

        assert_ne!(
            base,
            AnalysisRun::new(&transcript, &entries, detector_set_changed).snapshot()
        );
        assert_ne!(
            base,
            AnalysisRun::new(&transcript, &entries, config_changed).snapshot()
        );
        assert_ne!(
            base,
            AnalysisRun::new(&transcript, &entries, algorithm_changed).snapshot()
        );
    }

    #[test]
    fn for_canonical_session_terms_binds_owned_canonical_profile() {
        let transcript = transcript("Kafka");
        let entries = [entry("Apache Kafka", &["Kafka"], &[])];

        let run = AnalysisRun::for_canonical_session_terms(&transcript, &entries);

        assert_eq!(
            run.snapshot().configuration(),
            CANONICAL_SESSION_TERM_ANALYSIS_IDENTITY
        );
    }
}
