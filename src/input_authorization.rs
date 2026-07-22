use std::fmt;

use serde::{Deserialize, Serialize};

use crate::run_manifest::{
    InputClass, InputIdentityReference, InputIdentityValidationError, RunId, RunIdError,
    validate_input_identity_reference, validate_opaque_identifier,
};

pub const INPUT_AUTHORIZATION_SCHEMA: &str = "voxproof-input-authorization-v1";
pub const INPUT_AUTHORIZATION_SCOPE_POLICY: &str = "voxproof-local-evaluation-authorization-v1";

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct InputAuthorizationId(String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InputAuthorizationBasis {
    SelfOwned,
    ExplicitPermission,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InputAuthorizationState {
    Confirmed,
    Withdrawn,
    Invalidated,
}

/// Caller or owner assertion that local evaluation is authorized for one run and
/// input revision. This artifact does not independently prove ownership,
/// contract validity, copyright status, consent, or legal sufficiency. It
/// grants no implied training, public-distribution, or publication rights.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InputAuthorization {
    pub schema_revision: String,
    pub authorization_id: InputAuthorizationId,
    pub run_id: RunId,
    pub input_identity: InputIdentityReference,
    pub input_class: InputClass,
    pub authorization_basis: InputAuthorizationBasis,
    pub scope_policy_revision: String,
    pub state: InputAuthorizationState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputAuthorizationIdError {
    Empty,
    TooLong { len: usize, max: usize },
    InvalidCharacter { character: char },
    PathLikeContent,
    AbsolutePathLike,
    RelativePathLike,
    HomeDirectoryFragment,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputAuthorizationValidationError {
    MissingSchemaRevision,
    UnsupportedSchemaRevision {
        found: String,
        expected: String,
    },
    UnsupportedScopePolicy {
        found: String,
        expected: String,
    },
    InvalidAuthorizationId(InputAuthorizationIdError),
    InvalidRunId(RunIdError),
    InvalidInputIdentity(InputIdentityValidationError),
    UnsupportedInputClass {
        input_class: InputClass,
    },
    BasisInputClassMismatch {
        input_class: InputClass,
        authorization_basis: InputAuthorizationBasis,
    },
}

impl InputAuthorizationId {
    pub fn new(value: impl Into<String>) -> Result<Self, InputAuthorizationIdError> {
        let value = value.into();
        validate_authorization_id_value(&value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for InputAuthorizationId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl InputAuthorization {
    pub fn validate(&self) -> Result<(), InputAuthorizationValidationError> {
        if self.schema_revision.is_empty() {
            return Err(InputAuthorizationValidationError::MissingSchemaRevision);
        }

        if self.schema_revision != INPUT_AUTHORIZATION_SCHEMA {
            return Err(
                InputAuthorizationValidationError::UnsupportedSchemaRevision {
                    found: self.schema_revision.clone(),
                    expected: INPUT_AUTHORIZATION_SCHEMA.to_string(),
                },
            );
        }

        if self.scope_policy_revision != INPUT_AUTHORIZATION_SCOPE_POLICY {
            return Err(InputAuthorizationValidationError::UnsupportedScopePolicy {
                found: self.scope_policy_revision.clone(),
                expected: INPUT_AUTHORIZATION_SCOPE_POLICY.to_string(),
            });
        }

        validate_authorization_id_value(self.authorization_id.as_str())
            .map_err(InputAuthorizationValidationError::InvalidAuthorizationId)?;

        validate_opaque_identifier(self.run_id.as_str())
            .map_err(InputAuthorizationValidationError::InvalidRunId)?;

        validate_input_identity_reference(&self.input_identity)
            .map_err(InputAuthorizationValidationError::InvalidInputIdentity)?;

        validate_input_class_and_basis(self.input_class, self.authorization_basis)?;

        Ok(())
    }
}

pub fn validate_input_class_and_basis(
    input_class: InputClass,
    authorization_basis: InputAuthorizationBasis,
) -> Result<(), InputAuthorizationValidationError> {
    match input_class {
        InputClass::SelfOwnedReal => {
            if authorization_basis != InputAuthorizationBasis::SelfOwned {
                return Err(InputAuthorizationValidationError::BasisInputClassMismatch {
                    input_class,
                    authorization_basis,
                });
            }
        }
        InputClass::ExplicitPermissionReal => {
            if authorization_basis != InputAuthorizationBasis::ExplicitPermission {
                return Err(InputAuthorizationValidationError::BasisInputClassMismatch {
                    input_class,
                    authorization_basis,
                });
            }
        }
        InputClass::SyntheticProtocolFixture => {
            return Err(InputAuthorizationValidationError::UnsupportedInputClass { input_class });
        }
    }

    Ok(())
}

fn validate_authorization_id_value(value: &str) -> Result<(), InputAuthorizationIdError> {
    validate_opaque_identifier(value).map_err(map_run_id_error)
}

fn map_run_id_error(error: RunIdError) -> InputAuthorizationIdError {
    match error {
        RunIdError::Empty => InputAuthorizationIdError::Empty,
        RunIdError::TooLong { len, max } => InputAuthorizationIdError::TooLong { len, max },
        RunIdError::InvalidCharacter { character } => {
            InputAuthorizationIdError::InvalidCharacter { character }
        }
        RunIdError::PathLikeContent => InputAuthorizationIdError::PathLikeContent,
        RunIdError::AbsolutePathLike => InputAuthorizationIdError::AbsolutePathLike,
        RunIdError::RelativePathLike => InputAuthorizationIdError::RelativePathLike,
        RunIdError::HomeDirectoryFragment => InputAuthorizationIdError::HomeDirectoryFragment,
        RunIdError::GenerationUnavailable => InputAuthorizationIdError::Empty,
    }
}

pub fn input_authorization_from_json(json: &str) -> Result<InputAuthorization, serde_json::Error> {
    serde_json::from_str(json)
}

pub fn input_authorization_to_json(
    authorization: &InputAuthorization,
) -> Result<String, serde_json::Error> {
    serde_json::to_string(authorization)
}
