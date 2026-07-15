//! Experiment-only contextual ranking for bounded retrieval reports.
//!
//! Rankings are sidecar facts only. They never create review cases, ledger
//! events, canonical alternatives, or transcript edits.
//! Subprocess I/O workers report through channels and are waited only for a
//! bounded grace period. A worker may remain detached temporarily if a
//! descendant pathologically retains an inherited pipe.

use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc::{self, Receiver, TryRecvError},
};
use std::thread;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::experimental_retrieval::ExperimentalCandidateReport;

pub const RANKING_REQUEST_REVISION: &str = "experimental-context-rank-request-v1";
const IO_POLL_INTERVAL: Duration = Duration::from_millis(5);
const IO_COMPLETION_GRACE: Duration = Duration::from_millis(100);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExperimentalContextRanker {
    RulesOnly,
    DeterministicFake,
    ExternalCommand(ExternalCommandRanker),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalCommandRanker {
    pub program: String,
    pub arguments: Vec<String>,
    pub timeout_ms: u64,
    pub max_request_bytes: usize,
    pub max_output_bytes: usize,
}

struct CapturedStream {
    bytes: Vec<u8>,
    exceeded_limit: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExperimentalRankingRequest {
    pub schema_revision: String,
    pub session_description: String,
    pub nearby_context: String,
    pub source_surface: String,
    pub candidates: Vec<RankingCandidate>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RankingCandidate {
    pub candidate_id: String,
    pub canonical_term: String,
    pub producer: String,
    pub source_representation: String,
    pub target_representation: String,
    pub distance: usize,
    pub ratio_numerator: usize,
    pub ratio_denominator: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExperimentalRankingResult {
    pub provider_identity: String,
    pub request_revision: String,
    pub disposition: ExperimentalRankingDisposition,
    pub selected_candidate_id: Option<String>,
    pub assessment: Option<ExperimentalAssessment>,
    pub reason_codes: Vec<ExperimentalReasonCode>,
    pub requires_review: bool,
    pub display_explanation: Option<String>,
    pub failure: Option<ExperimentalRankingFailure>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExperimentalRankingDisposition {
    NotRun,
    Succeeded,
    FellBackAfterFailure,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExperimentalAssessment {
    Strong,
    Plausible,
    Ambiguous,
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExperimentalReasonCode {
    SessionDescriptionTermOverlap,
    NearbyContextTermOverlap,
    LowestDeterministicDistance,
    MultipleCandidates,
    NoCandidateSupported,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExperimentalRankingFailure {
    RequestTooLarge,
    SpawnFailed,
    StdinWriteFailed,
    StreamReadFailed,
    IoCompletionTimedOut,
    TimedOut,
    NonZeroExit,
    OutputTooLarge,
    MalformedResponse,
    InvalidResponse,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExternalRankingResponse {
    schema_revision: String,
    selected_candidate_id: Option<String>,
    assessment: ExperimentalAssessment,
    reason_codes: Vec<ExperimentalReasonCode>,
    requires_review: bool,
    #[serde(default)]
    display_explanation: Option<String>,
}

pub fn rank_experimental_candidates(
    ranker: &ExperimentalContextRanker,
    session_description: &str,
    nearby_context: &str,
    source_surface: &str,
    candidates: &[ExperimentalCandidateReport],
) -> ExperimentalRankingResult {
    match ranker {
        ExperimentalContextRanker::RulesOnly => rules_only_result(),
        ExperimentalContextRanker::DeterministicFake => {
            deterministic_fake(session_description, nearby_context, candidates)
        }
        ExperimentalContextRanker::ExternalCommand(config) => external_command(
            config,
            session_description,
            nearby_context,
            source_surface,
            candidates,
        ),
    }
}

fn rules_only_result() -> ExperimentalRankingResult {
    ExperimentalRankingResult {
        provider_identity: "rules-only".to_string(),
        request_revision: RANKING_REQUEST_REVISION.to_string(),
        disposition: ExperimentalRankingDisposition::NotRun,
        selected_candidate_id: None,
        assessment: None,
        reason_codes: Vec::new(),
        requires_review: true,
        display_explanation: None,
        failure: None,
    }
}

fn fallback(
    provider_identity: &str,
    failure: ExperimentalRankingFailure,
) -> ExperimentalRankingResult {
    ExperimentalRankingResult {
        provider_identity: provider_identity.to_string(),
        request_revision: RANKING_REQUEST_REVISION.to_string(),
        disposition: ExperimentalRankingDisposition::FellBackAfterFailure,
        selected_candidate_id: None,
        assessment: None,
        reason_codes: Vec::new(),
        requires_review: true,
        display_explanation: None,
        failure: Some(failure),
    }
}

fn deterministic_fake(
    session_description: &str,
    nearby_context: &str,
    candidates: &[ExperimentalCandidateReport],
) -> ExperimentalRankingResult {
    let mut sorted = candidates.iter().collect::<Vec<_>>();
    sorted.sort_by_key(|candidate| {
        (
            usize::MAX - context_overlap(session_description, &candidate.canonical_term),
            usize::MAX - context_overlap(nearby_context, &candidate.canonical_term),
            candidate.distance,
            &candidate.candidate_id,
        )
    });
    let selected = sorted.first();
    let description_overlap = selected
        .map(|candidate| context_overlap(session_description, &candidate.canonical_term))
        .unwrap_or(0);
    let context_overlap = selected
        .map(|candidate| context_overlap(nearby_context, &candidate.canonical_term))
        .unwrap_or(0);
    let mut reason_codes = Vec::new();
    if description_overlap > 0 {
        reason_codes.push(ExperimentalReasonCode::SessionDescriptionTermOverlap);
    }
    if context_overlap > 0 {
        reason_codes.push(ExperimentalReasonCode::NearbyContextTermOverlap);
    }
    let minimum_distance = candidates.iter().map(|candidate| candidate.distance).min();
    if selected.is_some_and(|candidate| Some(candidate.distance) == minimum_distance) {
        reason_codes.push(ExperimentalReasonCode::LowestDeterministicDistance);
    }
    if candidates.len() > 1 {
        reason_codes.push(ExperimentalReasonCode::MultipleCandidates);
    }
    if reason_codes.is_empty() {
        reason_codes.push(ExperimentalReasonCode::NoCandidateSupported);
    }

    let assessment = match (
        selected.is_some(),
        description_overlap + context_overlap,
        candidates.len(),
    ) {
        (false, _, _) => ExperimentalAssessment::Unsupported,
        (true, value, _) if value > 0 => ExperimentalAssessment::Strong,
        (true, _, count) if count > 1 => ExperimentalAssessment::Ambiguous,
        _ => ExperimentalAssessment::Plausible,
    };
    let selected_candidate_id = if assessment == ExperimentalAssessment::Ambiguous {
        None
    } else {
        selected.map(|candidate| candidate.candidate_id.clone())
    };
    if selected_candidate_id.is_none() {
        reason_codes.retain(|code| *code != ExperimentalReasonCode::LowestDeterministicDistance);
    }

    ExperimentalRankingResult {
        provider_identity: "deterministic-fake".to_string(),
        request_revision: RANKING_REQUEST_REVISION.to_string(),
        disposition: ExperimentalRankingDisposition::Succeeded,
        selected_candidate_id,
        assessment: Some(assessment),
        reason_codes,
        requires_review: true,
        display_explanation: Some("deterministic fake ranking; requires human review".to_string()),
        failure: None,
    }
}

fn external_command(
    config: &ExternalCommandRanker,
    session_description: &str,
    nearby_context: &str,
    source_surface: &str,
    candidates: &[ExperimentalCandidateReport],
) -> ExperimentalRankingResult {
    let request = ExperimentalRankingRequest {
        schema_revision: RANKING_REQUEST_REVISION.to_string(),
        session_description: session_description.to_string(),
        nearby_context: nearby_context.to_string(),
        source_surface: source_surface.to_string(),
        candidates: candidates.iter().map(ranking_candidate).collect(),
    };
    let Ok(mut request_json) = serde_json::to_vec(&request) else {
        return fallback(
            &config.program,
            ExperimentalRankingFailure::MalformedResponse,
        );
    };
    request_json.push(b'\n');
    if request_json.len() > config.max_request_bytes {
        return fallback(&config.program, ExperimentalRankingFailure::RequestTooLarge);
    }
    let mut child = match Command::new(&config.program)
        .args(&config.arguments)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(_) => return fallback(&config.program, ExperimentalRankingFailure::SpawnFailed),
    };
    let deadline = Instant::now() + Duration::from_millis(config.timeout_ms);

    let Some(stdout) = child.stdout.take() else {
        terminate_direct_child(&mut child);
        return fallback(
            &config.program,
            ExperimentalRankingFailure::StreamReadFailed,
        );
    };
    let stdout_reader = drain_stream(stdout, config.max_output_bytes);
    let Some(stderr) = child.stderr.take() else {
        terminate_direct_child(&mut child);
        return fallback(
            &config.program,
            ExperimentalRankingFailure::StreamReadFailed,
        );
    };
    let stderr_reader = drain_stream(stderr, config.max_output_bytes);
    let Some(stdin) = child.stdin.take() else {
        terminate_direct_child(&mut child);
        return fallback(
            &config.program,
            ExperimentalRankingFailure::StdinWriteFailed,
        );
    };
    let workers = WorkerReceivers {
        stdin: write_request(stdin, request_json),
        stdout: stdout_reader.receiver,
        stderr: stderr_reader.receiver,
    };
    let mut results = WorkerResults::default();

    loop {
        workers.poll(&mut results);
        if stdout_reader.exceeded.load(Ordering::Relaxed)
            || stderr_reader.exceeded.load(Ordering::Relaxed)
        {
            terminate_direct_child(&mut child);
            let _ = workers.wait_for_all(&mut results, Instant::now() + IO_COMPLETION_GRACE);
            return fallback(&config.program, ExperimentalRankingFailure::OutputTooLarge);
        }
        if let Some(failure) = results.worker_failure() {
            terminate_direct_child(&mut child);
            let _ = workers.wait_for_all(&mut results, Instant::now() + IO_COMPLETION_GRACE);
            return fallback(&config.program, failure);
        }

        match child.try_wait() {
            Ok(Some(status)) => {
                if !workers.wait_for_all(&mut results, Instant::now() + IO_COMPLETION_GRACE) {
                    return fallback(
                        &config.program,
                        ExperimentalRankingFailure::IoCompletionTimedOut,
                    );
                }
                if let Some(failure) = results.worker_failure() {
                    return fallback(&config.program, failure);
                }
                let Some((stdout, stderr)) = results.into_streams() else {
                    return fallback(
                        &config.program,
                        ExperimentalRankingFailure::IoCompletionTimedOut,
                    );
                };
                if !status.success() {
                    return fallback(&config.program, ExperimentalRankingFailure::NonZeroExit);
                }
                if stdout.exceeded_limit || stderr.exceeded_limit {
                    return fallback(&config.program, ExperimentalRankingFailure::OutputTooLarge);
                }
                return validate_external_response(&config.program, &stdout.bytes, candidates);
            }
            Ok(None) if Instant::now() >= deadline => {
                terminate_direct_child(&mut child);
                let _ = workers.wait_for_all(&mut results, Instant::now() + IO_COMPLETION_GRACE);
                return fallback(&config.program, ExperimentalRankingFailure::TimedOut);
            }
            Ok(None) => thread::sleep(IO_POLL_INTERVAL),
            Err(_) => {
                terminate_direct_child(&mut child);
                let _ = workers.wait_for_all(&mut results, Instant::now() + IO_COMPLETION_GRACE);
                return fallback(
                    &config.program,
                    ExperimentalRankingFailure::MalformedResponse,
                );
            }
        }
    }
}

struct StreamReader {
    exceeded: Arc<AtomicBool>,
    receiver: Receiver<std::io::Result<CapturedStream>>,
}

fn drain_stream<R: Read + Send + 'static>(mut stream: R, limit: usize) -> StreamReader {
    let exceeded = Arc::new(AtomicBool::new(false));
    let exceeded_for_thread = Arc::clone(&exceeded);
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let mut bytes = Vec::new();
        let mut buffer = [0_u8; 4096];
        let result = (|| {
            loop {
                let count = stream.read(&mut buffer)?;
                if count == 0 {
                    break;
                }
                let remaining = limit.saturating_sub(bytes.len());
                let stored = remaining.min(count);
                bytes.extend_from_slice(&buffer[..stored]);
                if stored < count {
                    exceeded_for_thread.store(true, Ordering::Relaxed);
                }
            }
            Ok(CapturedStream {
                bytes,
                exceeded_limit: exceeded_for_thread.load(Ordering::Relaxed),
            })
        })();
        let _ = sender.send(result);
    });
    StreamReader { exceeded, receiver }
}

fn write_request<W: Write + Send + 'static>(
    mut stdin: W,
    request: Vec<u8>,
) -> Receiver<std::io::Result<()>> {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let _ = sender.send(stdin.write_all(&request));
    });
    receiver
}

struct WorkerReceivers {
    stdin: Receiver<std::io::Result<()>>,
    stdout: Receiver<std::io::Result<CapturedStream>>,
    stderr: Receiver<std::io::Result<CapturedStream>>,
}

#[derive(Default)]
struct WorkerResults {
    stdin: Option<std::io::Result<()>>,
    stdout: Option<std::io::Result<CapturedStream>>,
    stderr: Option<std::io::Result<CapturedStream>>,
}

impl WorkerReceivers {
    fn poll(&self, results: &mut WorkerResults) {
        poll_receiver(&self.stdin, &mut results.stdin);
        poll_receiver(&self.stdout, &mut results.stdout);
        poll_receiver(&self.stderr, &mut results.stderr);
    }

    fn wait_for_all(&self, results: &mut WorkerResults, deadline: Instant) -> bool {
        loop {
            self.poll(results);
            if results.is_complete() {
                return true;
            }
            if Instant::now() >= deadline {
                return false;
            }
            thread::sleep(IO_POLL_INTERVAL);
        }
    }
}

impl WorkerResults {
    fn is_complete(&self) -> bool {
        self.stdin.is_some() && self.stdout.is_some() && self.stderr.is_some()
    }

    fn worker_failure(&self) -> Option<ExperimentalRankingFailure> {
        if self.stdin.as_ref().is_some_and(Result::is_err) {
            return Some(ExperimentalRankingFailure::StdinWriteFailed);
        }
        if self.stdout.as_ref().is_some_and(Result::is_err)
            || self.stderr.as_ref().is_some_and(Result::is_err)
        {
            return Some(ExperimentalRankingFailure::StreamReadFailed);
        }
        None
    }

    fn into_streams(self) -> Option<(CapturedStream, CapturedStream)> {
        Some((self.stdout?.ok()?, self.stderr?.ok()?))
    }
}

fn poll_receiver<T>(
    receiver: &Receiver<std::io::Result<T>>,
    result: &mut Option<std::io::Result<T>>,
) {
    if result.is_some() {
        return;
    }
    match receiver.try_recv() {
        Ok(value) => *result = Some(value),
        Err(TryRecvError::Empty) => {}
        Err(TryRecvError::Disconnected) => {
            *result = Some(Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "subprocess I/O worker disconnected",
            )));
        }
    }
}

fn terminate_direct_child(child: &mut std::process::Child) {
    let _ = child.kill();
    let _ = child.wait();
}

fn validate_external_response(
    provider_identity: &str,
    stdout: &[u8],
    candidates: &[ExperimentalCandidateReport],
) -> ExperimentalRankingResult {
    let Ok(response) = serde_json::from_slice::<ExternalRankingResponse>(stdout) else {
        return fallback(
            provider_identity,
            ExperimentalRankingFailure::MalformedResponse,
        );
    };
    let valid_id = response.selected_candidate_id.as_ref().is_none_or(|id| {
        candidates
            .iter()
            .any(|candidate| candidate.candidate_id == *id)
    });
    let valid_shape = match response.assessment {
        ExperimentalAssessment::Strong | ExperimentalAssessment::Plausible => {
            response.selected_candidate_id.is_some()
        }
        ExperimentalAssessment::Unsupported => response.selected_candidate_id.is_none(),
        ExperimentalAssessment::Ambiguous => {
            response.selected_candidate_id.is_none()
                && response
                    .reason_codes
                    .contains(&ExperimentalReasonCode::MultipleCandidates)
        }
    };
    if response.schema_revision != RANKING_REQUEST_REVISION
        || !response.requires_review
        || !valid_id
        || !valid_shape
        || response.reason_codes.is_empty()
    {
        return fallback(
            provider_identity,
            ExperimentalRankingFailure::InvalidResponse,
        );
    }
    ExperimentalRankingResult {
        provider_identity: provider_identity.to_string(),
        request_revision: response.schema_revision,
        disposition: ExperimentalRankingDisposition::Succeeded,
        selected_candidate_id: response.selected_candidate_id,
        assessment: Some(response.assessment),
        reason_codes: response.reason_codes,
        requires_review: response.requires_review,
        display_explanation: response.display_explanation,
        failure: None,
    }
}

fn ranking_candidate(candidate: &ExperimentalCandidateReport) -> RankingCandidate {
    RankingCandidate {
        candidate_id: candidate.candidate_id.clone(),
        canonical_term: candidate.canonical_term.clone(),
        producer: format!("{:?}", candidate.producer),
        source_representation: candidate.source_representation.clone(),
        target_representation: candidate.target_representation.clone(),
        distance: candidate.distance,
        ratio_numerator: candidate.ratio_numerator,
        ratio_denominator: candidate.ratio_denominator,
    }
}

fn context_overlap(context: &str, canonical_term: &str) -> usize {
    let context = context.to_ascii_lowercase();
    canonical_term
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|token| token.len() > 1 && context.contains(&token.to_ascii_lowercase()))
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::experimental_retrieval::{
        ExperimentalAnchor, ExperimentalProducer, ExperimentalRepresentation,
    };

    fn candidate(id: &str, term: &str) -> ExperimentalCandidateReport {
        ExperimentalCandidateReport {
            candidate_id: id.to_string(),
            source_anchor: ExperimentalAnchor {
                segment_position: 0,
                start_byte: 0,
                end_byte: 4,
            },
            source_surface: "Postg".to_string(),
            canonical_term: term.to_string(),
            producer: ExperimentalProducer::LatinNormalizedDistance,
            producer_version: "test".to_string(),
            representation: ExperimentalRepresentation::LatinAlphanumericLowercase,
            normalization_variant: "test".to_string(),
            source_representation: "postg".to_string(),
            target_representation: term.to_ascii_lowercase(),
            distance: 1,
            ratio_numerator: 4,
            ratio_denominator: 5,
            skipped_components: Vec::new(),
            pinyin: None,
        }
    }

    #[test]
    fn rules_only_needs_no_credentials_and_has_no_selection() {
        let result = rank_experimental_candidates(
            &ExperimentalContextRanker::RulesOnly,
            "PostgreSQL discussion",
            "Postgre sequel",
            "Postgre sequel",
            &[candidate("a", "PostgreSQL")],
        );
        assert_eq!(result.disposition, ExperimentalRankingDisposition::NotRun);
        assert_eq!(result.failure, None);
        assert_eq!(result.assessment, None);
        assert!(result.reason_codes.is_empty());
        assert_eq!(result.selected_candidate_id, None);
        assert!(result.requires_review);
    }

    #[test]
    fn fake_ranking_is_deterministic_and_remains_non_authoritative() {
        let candidates = vec![candidate("b", "Kafka"), candidate("a", "PostgreSQL")];
        let first = rank_experimental_candidates(
            &ExperimentalContextRanker::DeterministicFake,
            "PostgreSQL infrastructure",
            "Postgre sequel",
            "Postgre sequel",
            &candidates,
        );
        let second = rank_experimental_candidates(
            &ExperimentalContextRanker::DeterministicFake,
            "PostgreSQL infrastructure",
            "Postgre sequel",
            "Postgre sequel",
            &candidates,
        );
        assert_eq!(first, second);
        assert_eq!(first.selected_candidate_id.as_deref(), Some("a"));
        assert!(first.requires_review);
    }

    #[test]
    fn malformed_and_unknown_external_responses_fall_back() {
        let candidates = vec![candidate("a", "PostgreSQL")];
        let malformed = validate_external_response("test", b"not json", &candidates);
        assert_eq!(
            malformed.failure,
            Some(ExperimentalRankingFailure::MalformedResponse)
        );
        let unknown = br#"{"schema_revision":"experimental-context-rank-request-v1","selected_candidate_id":"unknown","assessment":"strong","reason_codes":["lowest_deterministic_distance"],"requires_review":true}"#;
        let result = validate_external_response("test", unknown, &candidates);
        assert_eq!(
            result.failure,
            Some(ExperimentalRankingFailure::InvalidResponse)
        );
        let replacement = br#"{"schema_revision":"experimental-context-rank-request-v1","selected_candidate_id":"a","assessment":"strong","reason_codes":["lowest_deterministic_distance"],"requires_review":true,"replacement":"untrusted text"}"#;
        let result = validate_external_response("test", replacement, &candidates);
        assert_eq!(
            result.failure,
            Some(ExperimentalRankingFailure::MalformedResponse)
        );
    }

    #[test]
    fn unavailable_external_provider_falls_back_without_losing_candidate_boundary() {
        let candidates = vec![candidate("a", "PostgreSQL")];
        let result = rank_experimental_candidates(
            &ExperimentalContextRanker::ExternalCommand(ExternalCommandRanker {
                program: "/definitely/not/a/vox-proof-ranker".to_string(),
                arguments: Vec::new(),
                timeout_ms: 10,
                max_request_bytes: 1024,
                max_output_bytes: 128,
            }),
            "synthetic",
            "Postgre sequel",
            "Postgre sequel",
            &candidates,
        );
        assert_eq!(
            result.failure,
            Some(ExperimentalRankingFailure::SpawnFailed)
        );
        assert!(result.requires_review);
        assert_eq!(result.selected_candidate_id, None);
    }

    #[test]
    fn fake_does_not_claim_lowest_distance_when_overlap_selects_a_worse_candidate() {
        let mut overlapping = candidate("overlap", "Kafka");
        overlapping.distance = 4;
        let mut closer = candidate("closer", "PostgreSQL");
        closer.distance = 1;
        let result = rank_experimental_candidates(
            &ExperimentalContextRanker::DeterministicFake,
            "Kafka discussion",
            "",
            "synthetic",
            &[overlapping, closer],
        );
        assert_eq!(result.selected_candidate_id.as_deref(), Some("overlap"));
        assert!(
            !result
                .reason_codes
                .contains(&ExperimentalReasonCode::LowestDeterministicDistance)
        );
    }

    fn external_config(
        program: &str,
        arguments: &[&str],
        max_output_bytes: usize,
    ) -> ExternalCommandRanker {
        ExternalCommandRanker {
            program: program.to_string(),
            arguments: arguments
                .iter()
                .map(|argument| (*argument).to_string())
                .collect(),
            timeout_ms: 200,
            max_request_bytes: 512 * 1024,
            max_output_bytes,
        }
    }

    #[test]
    fn external_command_accepts_valid_direct_child_output() {
        let candidates = vec![candidate("a", "PostgreSQL")];
        let response = r#"{"schema_revision":"experimental-context-rank-request-v1","selected_candidate_id":"a","assessment":"strong","reason_codes":["lowest_deterministic_distance"],"requires_review":true}"#;
        let program = format!(r#"my $input = <STDIN>; print '{}'"#, response);
        let result = rank_experimental_candidates(
            &ExperimentalContextRanker::ExternalCommand(external_config(
                "/usr/bin/perl",
                &["-e", &program],
                1024,
            )),
            "synthetic",
            "context",
            "source",
            &candidates,
        );
        assert_eq!(result.selected_candidate_id.as_deref(), Some("a"));
        assert_eq!(result.failure, None);
        assert_eq!(
            result.disposition,
            ExperimentalRankingDisposition::Succeeded
        );
    }

    #[test]
    fn external_command_nonzero_timeout_and_oversized_stdout_fall_back() {
        let candidates = vec![candidate("a", "PostgreSQL")];
        let nonzero = rank_experimental_candidates(
            &ExperimentalContextRanker::ExternalCommand(external_config("/usr/bin/false", &[], 64)),
            "synthetic",
            "context",
            "source",
            &candidates,
        );
        assert_eq!(
            nonzero.failure,
            Some(ExperimentalRankingFailure::NonZeroExit)
        );

        let timeout = rank_experimental_candidates(
            &ExperimentalContextRanker::ExternalCommand(ExternalCommandRanker {
                timeout_ms: 20,
                ..external_config("/bin/sleep", &["1"], 64)
            }),
            "synthetic",
            "context",
            "source",
            &candidates,
        );
        assert_eq!(timeout.failure, Some(ExperimentalRankingFailure::TimedOut));

        let oversized = rank_experimental_candidates(
            &ExperimentalContextRanker::ExternalCommand(external_config("/usr/bin/yes", &[], 64)),
            "synthetic",
            "context",
            "source",
            &candidates,
        );
        assert_eq!(
            oversized.failure,
            Some(ExperimentalRankingFailure::OutputTooLarge)
        );
    }

    #[test]
    fn external_command_drains_substantial_stderr_without_deadlock() {
        let candidates = vec![candidate("a", "PostgreSQL")];
        let program = r#"print STDERR "x" x 8192; print '{"schema_revision":"experimental-context-rank-request-v1","selected_candidate_id":"a","assessment":"strong","reason_codes":["lowest_deterministic_distance"],"requires_review":true}'"#;
        let result = rank_experimental_candidates(
            &ExperimentalContextRanker::ExternalCommand(external_config(
                "/usr/bin/perl",
                &["-e", program],
                16 * 1024,
            )),
            "synthetic",
            "context",
            "source",
            &candidates,
        );
        assert_eq!(result.failure, None);
        assert_eq!(result.selected_candidate_id.as_deref(), Some("a"));
    }

    #[test]
    fn child_that_never_reads_large_stdin_times_out() {
        let mut large_candidate = candidate("large", &"x".repeat(200_000));
        large_candidate.target_representation = "x".repeat(200_000);
        let started = Instant::now();
        let result = rank_experimental_candidates(
            &ExperimentalContextRanker::ExternalCommand(ExternalCommandRanker {
                timeout_ms: 25,
                ..external_config("/bin/sleep", &["1"], 1024)
            }),
            "synthetic",
            "context",
            "source",
            &[large_candidate],
        );
        assert_eq!(result.failure, Some(ExperimentalRankingFailure::TimedOut));
        assert!(started.elapsed() < Duration::from_millis(500));
    }

    #[test]
    fn oversized_request_is_rejected_before_spawn() {
        let result = rank_experimental_candidates(
            &ExperimentalContextRanker::ExternalCommand(ExternalCommandRanker {
                max_request_bytes: 1,
                ..external_config("/definitely/not/executed", &[], 1024)
            }),
            "synthetic",
            "context",
            "source",
            &[candidate("a", "PostgreSQL")],
        );
        assert_eq!(
            result.failure,
            Some(ExperimentalRankingFailure::RequestTooLarge)
        );
    }

    #[test]
    fn child_can_emit_substantial_output_before_reading_large_stdin() {
        let mut large_candidate = candidate("large", &"x".repeat(100_000));
        large_candidate.target_representation = "x".repeat(100_000);
        let response = r#"{"schema_revision":"experimental-context-rank-request-v1","selected_candidate_id":"large","assessment":"strong","reason_codes":["lowest_deterministic_distance"],"requires_review":true}"#;
        let program = format!(
            r#"print " " x 65536; print '{}'; STDOUT->flush(); my $input = <STDIN>"#,
            response
        );
        let result = rank_experimental_candidates(
            &ExperimentalContextRanker::ExternalCommand(external_config(
                "/usr/bin/perl",
                &["-e", &program],
                128 * 1024,
            )),
            "synthetic",
            "context",
            "source",
            &[large_candidate],
        );
        assert_eq!(result.failure, None);
        assert_eq!(result.selected_candidate_id.as_deref(), Some("large"));
    }

    #[test]
    fn inherited_descendant_pipe_returns_without_unbounded_reader_wait() {
        let candidates = vec![candidate("a", "PostgreSQL")];
        let program =
            r#"my $pid = fork(); if ($pid == 0) { sleep 1; exit 0; } my $input = <STDIN>; exit 0"#;
        let started = Instant::now();
        let result = rank_experimental_candidates(
            &ExperimentalContextRanker::ExternalCommand(ExternalCommandRanker {
                timeout_ms: 500,
                ..external_config("/usr/bin/perl", &["-e", program], 1024)
            }),
            "synthetic",
            "context",
            "source",
            &candidates,
        );
        assert_eq!(
            result.failure,
            Some(ExperimentalRankingFailure::IoCompletionTimedOut)
        );
        assert!(started.elapsed() < Duration::from_millis(500));
    }

    #[test]
    fn external_response_cross_fields_are_validated() {
        let candidates = vec![candidate("a", "PostgreSQL"), candidate("b", "Kafka")];
        let invalid = [
            r#"{"schema_revision":"experimental-context-rank-request-v1","selected_candidate_id":null,"assessment":"strong","reason_codes":["lowest_deterministic_distance"],"requires_review":true}"#,
            r#"{"schema_revision":"experimental-context-rank-request-v1","selected_candidate_id":null,"assessment":"plausible","reason_codes":["lowest_deterministic_distance"],"requires_review":true}"#,
            r#"{"schema_revision":"experimental-context-rank-request-v1","selected_candidate_id":"a","assessment":"unsupported","reason_codes":["no_candidate_supported"],"requires_review":true}"#,
            r#"{"schema_revision":"experimental-context-rank-request-v1","selected_candidate_id":"a","assessment":"ambiguous","reason_codes":["multiple_candidates"],"requires_review":true}"#,
            r#"{"schema_revision":"experimental-context-rank-request-v1","selected_candidate_id":null,"assessment":"ambiguous","reason_codes":["no_candidate_supported"],"requires_review":true}"#,
        ];
        for response in invalid {
            let result = validate_external_response("test", response.as_bytes(), &candidates);
            assert_eq!(
                result.failure,
                Some(ExperimentalRankingFailure::InvalidResponse)
            );
        }

        let ambiguous = br#"{"schema_revision":"experimental-context-rank-request-v1","selected_candidate_id":null,"assessment":"ambiguous","reason_codes":["multiple_candidates"],"requires_review":true}"#;
        let result = validate_external_response("test", ambiguous, &candidates);
        assert_eq!(result.failure, None);
        assert_eq!(result.assessment, Some(ExperimentalAssessment::Ambiguous));
        assert_eq!(result.selected_candidate_id, None);
    }
}
