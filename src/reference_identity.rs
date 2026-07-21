use std::fmt;

use serde::{Deserialize, Serialize};

use crate::run_manifest::{RunIdError, validate_opaque_identifier};

/// Portable opaque identity for one human-final reference revision.
/// No cryptographic uniqueness claim is made for generated values.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ReferenceRevisionId(String);

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

impl fmt::Display for ReferenceRevisionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

pub(crate) fn validate_identity_value(value: &str) -> Result<(), ReferenceIdentityIdError> {
    validate_opaque_identifier(value).map_err(map_run_id_error)
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
