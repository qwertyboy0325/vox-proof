use std::fmt;

use serde::{Deserialize, Serialize};

use crate::run_manifest::{RunIdError, validate_opaque_identifier};

const SHA256_DIGEST_PREFIX: &str = "sha256:";

/// Portable opaque identity for one human-final reference revision.
/// No cryptographic uniqueness claim is made for generated values.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ReferenceRevisionId(String);

/// Typed SHA-256 digest of cue source text bytes. Syntax validation only.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CueSourceTextDigest(String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationBasis {
    AudioListened,
    TranscriptContextOnly,
    MixedSources,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReferenceReviewerIdentityClass {
    OwnerBlindReviewer,
    AuthorizedDomainReviewer,
    SyntheticFixtureGenerator,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReferenceIdentityIdError {
    Empty,
    TooLong { len: usize, max: usize },
    InvalidCharacter { character: char },
    PathLikeContent,
    AbsolutePathLike,
    RelativePathLike,
    HomeDirectoryFragment,
    GenerationUnavailable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CueSourceTextDigestError {
    MissingPrefix,
    InvalidLength,
    InvalidHexCharacter { character: char },
    UppercaseHexNotCanonical,
}

impl ReferenceRevisionId {
    pub fn new(value: impl Into<String>) -> Result<Self, ReferenceIdentityIdError> {
        let value = value.into();
        validate_identity_value(&value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn generate() -> Result<Self, ReferenceIdentityIdError> {
        use std::time::{SystemTime, UNIX_EPOCH};

        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| ReferenceIdentityIdError::GenerationUnavailable)?
            .as_nanos();
        Self::new(format!("ref-rev-{nanos:x}"))
    }
}

impl CueSourceTextDigest {
    pub fn new(value: impl Into<String>) -> Result<Self, CueSourceTextDigestError> {
        let value = value.into();
        validate_cue_source_text_digest(&value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ReferenceRevisionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl fmt::Display for CueSourceTextDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

pub(crate) fn validate_identity_value(value: &str) -> Result<(), ReferenceIdentityIdError> {
    validate_opaque_identifier(value).map_err(map_run_id_error)
}

pub fn validate_cue_source_text_digest(value: &str) -> Result<(), CueSourceTextDigestError> {
    if !value.starts_with(SHA256_DIGEST_PREFIX) {
        return Err(CueSourceTextDigestError::MissingPrefix);
    }

    let digest = &value[SHA256_DIGEST_PREFIX.len()..];
    if digest.len() != 64 {
        return Err(CueSourceTextDigestError::InvalidLength);
    }

    for character in digest.chars() {
        if character.is_ascii_digit() || matches!(character, 'a'..='f') {
            continue;
        }
        if character.is_ascii_hexdigit() {
            return Err(CueSourceTextDigestError::UppercaseHexNotCanonical);
        }
        return Err(CueSourceTextDigestError::InvalidHexCharacter { character });
    }

    Ok(())
}

fn map_run_id_error(error: RunIdError) -> ReferenceIdentityIdError {
    match error {
        RunIdError::Empty => ReferenceIdentityIdError::Empty,
        RunIdError::TooLong { len, max } => ReferenceIdentityIdError::TooLong { len, max },
        RunIdError::InvalidCharacter { character } => {
            ReferenceIdentityIdError::InvalidCharacter { character }
        }
        RunIdError::PathLikeContent => ReferenceIdentityIdError::PathLikeContent,
        RunIdError::AbsolutePathLike => ReferenceIdentityIdError::AbsolutePathLike,
        RunIdError::RelativePathLike => ReferenceIdentityIdError::RelativePathLike,
        RunIdError::HomeDirectoryFragment => ReferenceIdentityIdError::HomeDirectoryFragment,
        RunIdError::GenerationUnavailable => ReferenceIdentityIdError::GenerationUnavailable,
    }
}
