use crate::v2_api::authn_method_test_helpers::{
    create_identity_with_authn_method, test_authn_method,
};
use candid::Principal;
use canister_tests::api::internet_identity::api_v2;
use canister_tests::framework::{
    env, expect_user_error_with_message, install_ii_canister, II_WASM,
};
use canister_tests::match_value;
use internet_identity_interface::internet_identity::types::{IdentityInfoResponse, MetadataEntry};
use pocket_ic::CallError;
use pocket_ic::ErrorCode::CanisterCalledTrap;
use regex::Regex;
use std::collections::HashMap;

#[test]
fn should_write_metadata() -> Result<(), CallError> {
    const METADATA_KEY: &str = "some-key";

    let env = env();
    let canister_id = install_ii_canister(&env, II_WASM.clone());
    let authn_method = test_authn_method();
    let identity_number = create_identity_with_authn_method(&env, canister_id, &authn_method);

    match_value!(
        api_v2::identity_info(&env, canister_id, authn_method.principal(), identity_number)?,
        Some(IdentityInfoResponse::Ok(identity_info))
    );
    assert!(identity_info.metadata.is_empty());

    let metadata = HashMap::from_iter(vec![(
        METADATA_KEY.to_string(),
        MetadataEntry::String("some value".to_string()),
    )]);

    api_v2::identity_metadata_replace(
        &env,
        canister_id,
        authn_method.principal(),
        identity_number,
        &metadata,
    )?;

    match_value!(
        api_v2::identity_info(&env, canister_id, authn_method.principal(), identity_number)?,
        Some(IdentityInfoResponse::Ok(identity_info))
    );
    assert_eq!(identity_info.metadata, metadata);
    Ok(())
}

#[test]
fn should_require_authentication_to_replace_identity_metadata() {
    const METADATA_KEY: &str = "some-key";

    let env = env();
    let canister_id = install_ii_canister(&env, II_WASM.clone());
    let authn_method = test_authn_method();
    let identity_number = create_identity_with_authn_method(&env, canister_id, &authn_method);

    let metadata = HashMap::from_iter(vec![(
        METADATA_KEY.to_string(),
        MetadataEntry::String("some value".to_string()),
    )]);

    let result = api_v2::identity_metadata_replace(
        &env,
        canister_id,
        Principal::anonymous(),
        identity_number,
        &metadata,
    );
    expect_user_error_with_message(
        result,
        CanisterCalledTrap,
        Regex::new("[a-z\\d-]+ could not be authenticated.").unwrap(),
    );
}

#[test]
fn should_not_write_too_large_identity_metadata_map() -> Result<(), CallError> {
    const METADATA_KEY: &str = "some-key";

    let env = env();
    let canister_id = install_ii_canister(&env, II_WASM.clone());
    let authn_method = test_authn_method();
    let identity_number = create_identity_with_authn_method(&env, canister_id, &authn_method);

    match_value!(
        api_v2::identity_info(&env, canister_id, authn_method.principal(), identity_number)?,
        Some(IdentityInfoResponse::Ok(identity_info))
    );
    assert!(identity_info.metadata.is_empty());

    let metadata = HashMap::from_iter(vec![(
        METADATA_KEY.to_string(),
        MetadataEntry::String("a".repeat(3000)),
    )]);

    let result = api_v2::identity_metadata_replace(
        &env,
        canister_id,
        authn_method.principal(),
        identity_number,
        &metadata,
    );
    expect_user_error_with_message(
        result,
        CanisterCalledTrap,
        Regex::new("failed to write identity metadata: Cumulative size of variable sized fields exceeds limit: length \\d+, limit \\d+\\.").unwrap(),
    );
    Ok(())
}
