use vox_proof::input_authorization::{
    INPUT_AUTHORIZATION_SCHEMA, INPUT_AUTHORIZATION_SCOPE_POLICY, InputAuthorization,
    InputAuthorizationBasis, InputAuthorizationId, InputAuthorizationState,
    InputAuthorizationValidationError, input_authorization_from_json, input_authorization_to_json,
    validate_input_class_and_basis,
};
use vox_proof::run_manifest::{InputClass, InputIdentityReference, RunId};

const SAMPLE_REVISION: &str =
    "rev:sha256-v1:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn authorization(
    input_class: InputClass,
    basis: InputAuthorizationBasis,
    state: InputAuthorizationState,
) -> InputAuthorization {
    InputAuthorization {
        schema_revision: INPUT_AUTHORIZATION_SCHEMA.to_string(),
        authorization_id: InputAuthorizationId::new("auth-real-contract-001")
            .expect("authorization id"),
        run_id: RunId::new("run-real-contract-001").expect("run id"),
        input_identity: InputIdentityReference {
            transcript_revision_id: SAMPLE_REVISION.to_string(),
        },
        input_class,
        authorization_basis: basis,
        scope_policy_revision: INPUT_AUTHORIZATION_SCOPE_POLICY.to_string(),
        state,
    }
}

#[test]
fn self_owned_real_confirmed_validates() {
    authorization(
        InputClass::SelfOwnedReal,
        InputAuthorizationBasis::SelfOwned,
        InputAuthorizationState::Confirmed,
    )
    .validate()
    .expect("valid");
}

#[test]
fn explicit_permission_real_confirmed_validates() {
    authorization(
        InputClass::ExplicitPermissionReal,
        InputAuthorizationBasis::ExplicitPermission,
        InputAuthorizationState::Confirmed,
    )
    .validate()
    .expect("valid");
}

#[test]
fn synthetic_protocol_fixture_rejected() {
    assert!(matches!(
        validate_input_class_and_basis(
            InputClass::SyntheticProtocolFixture,
            InputAuthorizationBasis::SelfOwned,
        ),
        Err(InputAuthorizationValidationError::UnsupportedInputClass { .. })
    ));
}

#[test]
fn self_owned_real_with_explicit_permission_basis_rejected() {
    assert!(matches!(
        authorization(
            InputClass::SelfOwnedReal,
            InputAuthorizationBasis::ExplicitPermission,
            InputAuthorizationState::Confirmed,
        )
        .validate(),
        Err(InputAuthorizationValidationError::BasisInputClassMismatch { .. })
    ));
}

#[test]
fn explicit_permission_real_with_self_owned_basis_rejected() {
    assert!(matches!(
        authorization(
            InputClass::ExplicitPermissionReal,
            InputAuthorizationBasis::SelfOwned,
            InputAuthorizationState::Confirmed,
        )
        .validate(),
        Err(InputAuthorizationValidationError::BasisInputClassMismatch { .. })
    ));
}

#[test]
fn withdrawn_structurally_valid_but_not_confirmed() {
    authorization(
        InputClass::SelfOwnedReal,
        InputAuthorizationBasis::SelfOwned,
        InputAuthorizationState::Withdrawn,
    )
    .validate()
    .expect("structurally valid");
}

#[test]
fn invalidated_structurally_valid_but_not_confirmed() {
    authorization(
        InputClass::SelfOwnedReal,
        InputAuthorizationBasis::SelfOwned,
        InputAuthorizationState::Invalidated,
    )
    .validate()
    .expect("structurally valid");
}

#[test]
fn unsupported_schema_rejected() {
    let mut auth = authorization(
        InputClass::SelfOwnedReal,
        InputAuthorizationBasis::SelfOwned,
        InputAuthorizationState::Confirmed,
    );
    auth.schema_revision = "voxproof-input-authorization-v0".to_string();

    assert!(matches!(
        auth.validate(),
        Err(InputAuthorizationValidationError::UnsupportedSchemaRevision { .. })
    ));
}

#[test]
fn unsupported_scope_policy_rejected() {
    let mut auth = authorization(
        InputClass::SelfOwnedReal,
        InputAuthorizationBasis::SelfOwned,
        InputAuthorizationState::Confirmed,
    );
    auth.scope_policy_revision = "voxproof-other-scope-v1".to_string();

    assert!(matches!(
        auth.validate(),
        Err(InputAuthorizationValidationError::UnsupportedScopePolicy { .. })
    ));
}

#[test]
fn path_like_authorization_id_rejected() {
    assert!(InputAuthorizationId::new("../auth-id").is_err());
}

#[test]
fn unknown_json_field_rejected() {
    let json = format!(
        r#"{{
  "schema_revision": "{INPUT_AUTHORIZATION_SCHEMA}",
  "authorization_id": "auth-real-contract-001",
  "run_id": "run-real-contract-001",
  "input_identity": {{ "transcript_revision_id": "{SAMPLE_REVISION}" }},
  "input_class": "self_owned_real",
  "authorization_basis": "self_owned",
  "scope_policy_revision": "{INPUT_AUTHORIZATION_SCOPE_POLICY}",
  "state": "confirmed",
  "permission_email": "forbidden@example.com"
}}"#
    );

    let error = input_authorization_from_json(&json).expect_err("must fail");
    assert!(error.to_string().contains("unknown field"));
}

#[test]
fn compact_json_round_trip_exact_equality() {
    let auth = authorization(
        InputClass::ExplicitPermissionReal,
        InputAuthorizationBasis::ExplicitPermission,
        InputAuthorizationState::Confirmed,
    );
    let json = input_authorization_to_json(&auth).expect("serialize");
    let restored = input_authorization_from_json(&json).expect("deserialize");
    assert_eq!(restored, auth);
}
