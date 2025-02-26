//! Tests related to stable memory.
//! These tests make sure that II can be recovered from a stable memory backup.

use candid::Principal;
use canister_tests::api::internet_identity as api;
use canister_tests::flows;
use canister_tests::framework::*;
use internet_identity_interface::internet_identity::types::*;
use pocket_ic::common::rest::BlobCompression;
use pocket_ic::CallError;
use pocket_ic::ErrorCode::CanisterCalledTrap;
use rand::Rng;
use regex::Regex;
use serde_bytes::ByteBuf;
use std::path::PathBuf;

#[cfg(test)]
mod test_setup_helpers;

/// Known devices that exist in the genesis memory backups.
fn known_devices() -> [DeviceData; 6] {
    const CREDENTIAL_ID_1: &str = "63b8afb386dd757dfa5ba9550bca66936717766f395bafad9052a384edc446b11228bcb9cb684980bb5a81270b31d4b9561787296d40204d31e96c1b386b4984";
    const PUB_KEY_1: &str = "305e300c060a2b0601040183b8430101034e00a5010203262001215820ee6f212d1b94fcc014f050b087f06ad34157ff53c19981e3976842b1644b0a1c2258200d6bc5ee077bd2300b3c86df87aa5fdf90d256d0131efbe44424330de8b00471";
    const CREDENTIAL_ID_2: &str = "01bf2325d975f7b24c2d4bb6fef94a2e6dbbb35f490689f460a36f0f96717ac5487ad63899efd59fd01ef38aab8a228badaa1b94cd819572695c446e2c379792af7f";
    const PUB_KEY_2: &str = "305e300c060a2b0601040183b8430101034e00a5010203262001215820724d6d2ae54244650134e475aaced8d82d45520ba672d9892c8de34d2a40e6f3225820e1f89fbff2e05a3ef1a35c1d9bb2de8b5e8856fd710b1a534a0841835cb793aa";
    const CREDENTIAL_ID_3: &str = "2ceb7800078c607e94f8e9432fb1cd9e033081a7";
    const PUB_KEY_3: &str = "305e300c060a2b0601040183b8430101034e00a50102032620012158208ca9cb400318172cb199b3df2f7c601f02fc72be73282ebc88ab0fb5562aae40225820f53cae416256e1ea0592f781f506c8cdd9fa67dbb329c5fca469ac6b5868b4cd";
    const CREDENTIAL_ID_4: &str = "0192eea062df84cde762eff346aaa3a7fb44f1aa19d888ae407295b77c4c754b755b2b7b90d9174c0cf41d3eb3928f1eb310e3b3a4bc00445179df0f84b7f8b1db";
    const PUB_KEY_4: &str = "305e300c060a2b0601040183b8430101034e00a5010203262001215820c8423e7f1df8dc91f599dd3683f37541514341643e916b0a77e935da1a7e5ff42258204f5d37a73d6e1b1ac6ebd0d7739ebf477a8f88ed6992cb36b6c481efee01b462";
    const PUB_KEY_5: &str =
        "302a300506032b6570032100f1ba3b80ce24f382fa32fd07233ceb8e305d57dafe6ad3d1c00e401315692631";
    const PUB_KEY_6: &str = "305e300c060a2b0601040183b8430101034e00a50102032620012158206c52bead5df52c208a9b1c7be0a60847573e5be4ac4fe08ea48036d0ba1d2acf225820b33daeb83bc9c77d8ad762fd68e3eab08684e463c49351b3ab2a14a400138387";

    let device1 = DeviceData {
        pubkey: ByteBuf::from(hex::decode(PUB_KEY_1).unwrap()),
        alias: "Desktop".to_string(),
        credential_id: Some(ByteBuf::from(hex::decode(CREDENTIAL_ID_1).unwrap())),
        ..DeviceData::auth_test_device()
    };
    let device2 = DeviceData {
        pubkey: ByteBuf::from(hex::decode(PUB_KEY_2).unwrap()),
        alias: "andrew-mbp".to_string(),
        credential_id: Some(ByteBuf::from(hex::decode(CREDENTIAL_ID_2).unwrap())),
        ..DeviceData::auth_test_device()
    };
    let device3 = DeviceData {
        pubkey: ByteBuf::from(hex::decode(PUB_KEY_3).unwrap()),
        alias: "andrew phone chrome".to_string(),
        credential_id: Some(ByteBuf::from(hex::decode(CREDENTIAL_ID_3).unwrap())),
        ..DeviceData::auth_test_device()
    };
    let device4 = DeviceData {
        pubkey: ByteBuf::from(hex::decode(PUB_KEY_4).unwrap()),
        alias: "Pixel".to_string(),
        credential_id: Some(ByteBuf::from(hex::decode(CREDENTIAL_ID_4).unwrap())),
        ..DeviceData::auth_test_device()
    };
    let device5 = DeviceData {
        pubkey: ByteBuf::from(hex::decode(PUB_KEY_5).unwrap()),
        alias: "dfx".to_string(),
        ..DeviceData::auth_test_device()
    };
    let device6 = DeviceData {
        pubkey: ByteBuf::from(hex::decode(PUB_KEY_6).unwrap()),
        alias: "testkey".to_string(),
        ..DeviceData::auth_test_device()
    };
    [device1, device2, device3, device4, device5, device6]
}

/// Tests that II will issue the same principals after stable memory restore.
#[test]
fn should_issue_same_principal_after_restoring_backup() -> Result<(), CallError> {
    const PUBLIC_KEY: &str = "305e300c060a2b0601040183b8430101034e00a50102032620012158206c52bead5df52c208a9b1c7be0a60847573e5be4ac4fe08ea48036d0ba1d2acf225820b33daeb83bc9c77d8ad762fd68e3eab08684e463c49351b3ab2a14a400138387";
    const DELEGATION_PRINCIPAL: &str = "303c300c060a2b0601040183b8430102032c000a000000000000000001013a8926914dd1c836ec67ba66ac6425c21dffd3ca5c5968855f87780a1ec57985";
    let principal = Principal::self_authenticating(hex::decode(PUBLIC_KEY).unwrap());

    let env = env();
    let canister_id = install_ii_canister(&env, EMPTY_WASM.clone());

    restore_compressed_stable_memory(
        &env,
        canister_id,
        "stable_memory/genesis-layout-migrated-to-v7.bin.gz",
    );
    upgrade_ii_canister(&env, canister_id, II_WASM.clone());

    let (user_key, _) = api::prepare_delegation(
        &env,
        canister_id,
        principal,
        10_030,
        "example.com",
        &ByteBuf::from("dummykey"),
        None,
    )?;

    // check that we get the same user key; this proves that the salt was recovered from the backup
    assert_eq!(
        user_key.clone().into_vec(),
        hex::decode(DELEGATION_PRINCIPAL).unwrap()
    );

    let principal = api::get_principal(&env, canister_id, principal, 10_030, "example.com")?;
    assert_eq!(Principal::self_authenticating(user_key), principal);
    Ok(())
}

/// Tests that anchors can still be modified after stable memory restore.
#[test]
fn should_modify_devices_after_restoring_backup() -> Result<(), CallError> {
    let [_, _, _, _, device5, device6] = known_devices();
    let env = env();
    let canister_id = install_ii_canister(&env, EMPTY_WASM.clone());

    restore_compressed_stable_memory(
        &env,
        canister_id,
        "stable_memory/genesis-layout-migrated-to-v7.bin.gz",
    );
    upgrade_ii_canister(&env, canister_id, II_WASM.clone());

    let devices =
        api::get_anchor_info(&env, canister_id, device6.principal(), 10_030)?.into_device_data();

    assert_eq!(devices.len(), 2);
    api::remove(
        &env,
        canister_id,
        device6.principal(),
        10_030,
        &device5.pubkey,
    )?;

    let devices =
        api::get_anchor_info(&env, canister_id, device6.principal(), 10_030)?.into_device_data();
    assert_eq!(devices.len(), 1);
    Ok(())
}

/// Verifies that an anchor with two recovery phrases can still use both.
/// This anchor is recovered from stable memory because the current version of II does not allow to create such anchors.
#[test]
fn should_not_break_on_multiple_legacy_recovery_phrases() -> Result<(), CallError> {
    let env = env();
    let canister_id = install_ii_canister(&env, EMPTY_WASM.clone());
    let frontend_hostname = "frontend_hostname";
    let session_key = ByteBuf::from("session_key");

    restore_compressed_stable_memory(
        &env,
        canister_id,
        "stable_memory/multiple-recovery-phrases-v7.bin.gz",
    );
    upgrade_ii_canister(&env, canister_id, II_WASM.clone());

    api::prepare_delegation(
        &env,
        canister_id,
        principal_recovery_1(),
        10_000,
        frontend_hostname,
        &session_key,
        None,
    )?;
    api::prepare_delegation(
        &env,
        canister_id,
        principal_recovery_2(),
        10_000,
        frontend_hostname,
        &session_key,
        None,
    )?;
    Ok(())
}

/// Verifies that an existing account with two recovery phrases can only make changes after deleting one.
/// This anchor is recovered from stable memory because the current version of II does not allow to create such anchors.
#[test]
fn should_allow_modification_after_deleting_second_recovery_phrase() -> Result<(), CallError> {
    let env = env();
    let canister_id = install_ii_canister(&env, EMPTY_WASM.clone());

    restore_compressed_stable_memory(
        &env,
        canister_id,
        "stable_memory/multiple-recovery-phrases-v7.bin.gz",
    );
    upgrade_ii_canister(&env, canister_id, II_WASM.clone());

    let mut recovery_1 = recovery_device_data_1();
    recovery_1.alias = "new alias".to_string();
    let result = api::update(
        &env,
        canister_id,
        principal_1(),
        10_000,
        &recovery_1.pubkey,
        &recovery_1,
    );
    expect_user_error_with_message(
        result,
        CanisterCalledTrap,
        Regex::new("There is already a recovery phrase and only one is allowed\\.").unwrap(),
    );

    api::remove(
        &env,
        canister_id,
        principal_1(),
        10_000,
        &recovery_device_data_2().pubkey,
    )?;

    // successful after removing the other one
    api::update(
        &env,
        canister_id,
        principal_1(),
        10_000,
        &recovery_1.pubkey,
        &recovery_1,
    )?;
    Ok(())
}

#[test]
fn should_read_persistent_state_v7() -> Result<(), CallError> {
    let env = env();
    let canister_id = install_ii_canister(&env, EMPTY_WASM.clone());

    restore_compressed_stable_memory(
        &env,
        canister_id,
        "stable_memory/persistent_state_no_archive_v7.bin.gz",
    );
    upgrade_ii_canister(&env, canister_id, II_WASM.clone());

    let devices = api::get_anchor_info(&env, canister_id, principal_1(), 127)?.into_device_data();
    assert_eq!(devices.len(), 7);

    let stats = api::stats(&env, canister_id)?;
    assert!(stats.archive_info.archive_canister.is_none());
    assert!(stats.archive_info.archive_config.is_none());
    assert_eq!(7, stats.storage_layout_version);
    Ok(())
}

#[test]
fn should_grow_persistent_state_v7_by_bucket_size() -> Result<(), CallError> {
    let env = env();
    let canister_id = install_ii_canister(&env, EMPTY_WASM.clone());

    // A memory bucket has 128 pages, each 64kB.  A single anchor occupies 4kB, hence we have
    // 16 anchors per page, and 2048 per bucket.
    // The stored persistent state has 2047 anchors starting from anchor_number = 1000.
    // It has only 2047 (and not 2048) anchors to leave space for storing `PersistentState`
    // structure during the pre-upgrade hook (otherwise a second bucket gets allocated
    // for `PersistentState` although all the 2048 anchors fit into the first bucket).
    let first_anchor_number: AnchorNumber = 1000;
    let anchor_count = 2047;
    restore_compressed_stable_memory(
        &env,
        canister_id,
        "stable_memory/persistent_state_full_bucket_v7_2047_ids.bin.gz",
    );
    upgrade_ii_canister(&env, canister_id, II_WASM.clone());
    let stats = api::stats(&env, canister_id)?;
    assert_eq!(7, stats.storage_layout_version);

    // Check the number of allocated memory pages before expansion.
    let metrics = get_metrics(&env, canister_id);
    let (stable_memory_pages, _) = parse_metric(&metrics, "internet_identity_stable_memory_pages");
    assert_eq!(stable_memory_pages, 130f64);

    // Verify a random existing anchor.
    let random_anchor_offset = rand::thread_rng().gen_range(0..anchor_count);
    let random_anchor = first_anchor_number + random_anchor_offset;
    let principal = test_setup_helpers::principal(random_anchor_offset as usize);
    let devices =
        api::get_anchor_info(&env, canister_id, principal, random_anchor)?.into_device_data();
    assert_eq!(devices.len(), 2);
    let device = &devices[0];
    assert_eq!(format!("device #{}", random_anchor_offset), device.alias);

    // Add a new anchor -- this DOES NOT trigger an allocation of a new managed memory bucket.
    // as we're filling up the first bucket to 2048 anchors.
    let anchor_offset = anchor_count;
    let next_anchor: AnchorNumber = first_anchor_number + anchor_offset;
    let new_anchor_number = flows::register_anchor_with(
        &env,
        canister_id,
        test_setup_helpers::principal(anchor_offset as usize),
        &test_setup_helpers::sample_unique_device(anchor_offset as usize),
    );
    assert_eq!(next_anchor, new_anchor_number);

    // Verify the newly added anchor.
    let principal = test_setup_helpers::principal(anchor_offset as usize);
    let devices =
        api::get_anchor_info(&env, canister_id, principal, next_anchor)?.into_device_data();
    assert_eq!(devices.len(), 1);
    let device = &devices[0];
    assert_eq!(format!("device #{}", anchor_offset), device.alias);

    // Verify the number of allocated memory pages didn't grow yet.
    let metrics = get_metrics(&env, canister_id);
    let (stable_memory_pages, _) = parse_metric(&metrics, "internet_identity_stable_memory_pages");
    assert_eq!(stable_memory_pages, 130f64);

    // Add another anchor -- this DOES trigger an allocation of a new managed memory bucket.
    let anchor_offset = anchor_count + 1;
    let next_anchor: AnchorNumber = first_anchor_number + anchor_offset;
    let new_anchor_number = flows::register_anchor_with(
        &env,
        canister_id,
        test_setup_helpers::principal(anchor_offset as usize),
        &test_setup_helpers::sample_unique_device(anchor_offset as usize),
    );
    assert_eq!(next_anchor, new_anchor_number);

    // Verify the newly added anchor.
    let principal = test_setup_helpers::principal(anchor_offset as usize);
    let devices =
        api::get_anchor_info(&env, canister_id, principal, next_anchor)?.into_device_data();
    assert_eq!(devices.len(), 1);
    let device = &devices[0];
    assert_eq!(format!("device #{}", anchor_offset), device.alias);

    // Verify the number of allocated memory pages grew as expected (i.e. by 128 pages).
    assert_eq!(next_anchor, new_anchor_number);
    let metrics = get_metrics(&env, canister_id);
    let (stable_memory_pages, _) = parse_metric(&metrics, "internet_identity_stable_memory_pages");
    assert_eq!(stable_memory_pages, 258f64);

    // Verify another random existing anchor (after addition of a new one).
    let random_anchor_offset = rand::thread_rng().gen_range(0..anchor_count);
    let random_anchor = first_anchor_number + random_anchor_offset;
    let principal = test_setup_helpers::principal(random_anchor_offset as usize);
    let devices =
        api::get_anchor_info(&env, canister_id, principal, random_anchor)?.into_device_data();
    assert_eq!(devices.len(), 2);
    let device = &devices[0];
    assert_eq!(format!("device #{}", random_anchor_offset), device.alias);

    Ok(())
}

/// Verifies that a stable memory backup with persistent state containing archive information is restored correctly.
#[test]
fn should_read_persistent_state_with_archive() -> Result<(), CallError> {
    let env = env();
    let canister_id = install_ii_canister(&env, EMPTY_WASM.clone());

    restore_compressed_stable_memory(
        &env,
        canister_id,
        "stable_memory/persistent_state_archive_v7.bin.gz",
    );
    upgrade_ii_canister(&env, canister_id, II_WASM.clone());

    let devices =
        api::get_anchor_info(&env, canister_id, principal_1(), 10_000)?.into_device_data();
    assert_eq!(devices.len(), 1);

    let stats = api::stats(&env, canister_id)?;
    assert_eq!(
        stats.archive_info.archive_canister.unwrap(),
        Principal::from_text("rrkah-fqaaa-aaaaa-aaaaq-cai").unwrap()
    );

    assert_eq!(
        stats
            .archive_info
            .archive_config
            .unwrap()
            .module_hash
            .to_vec(),
        hex::decode("12e2c2bd05dfcd86e3004ecd5f00533e6120e7bcf82bac0753af0a7fe14bfea1").unwrap()
    );
    assert_eq!(stats.storage_layout_version, 7);
    Ok(())
}

/// Tests that II will refuse to install on a stable memory layout that is no longer supported.
#[test]
fn should_trap_on_old_stable_memory() -> Result<(), CallError> {
    let env = env();
    let canister_id = install_ii_canister(&env, EMPTY_WASM.clone());

    let stable_memory_backup =
        std::fs::read(PathBuf::from("stable_memory/genesis-memory-layout.bin")).unwrap();
    env.set_stable_memory(
        canister_id,
        stable_memory_backup,
        BlobCompression::NoCompression,
    );
    let result = upgrade_ii_canister_with_arg(&env, canister_id, II_WASM.clone(), None);
    assert!(result.is_err());
    let err = result.err().unwrap();
    match err {
        CallError::Reject(err) => panic!("unexpected error {err}"),
        CallError::UserError(err) => {
            assert_eq!(err.code, CanisterCalledTrap);
            assert!(err.description.contains("stable memory layout version 1 is no longer supported:\nEither reinstall (wiping stable memory) or migrate using a previous II version"));
        }
    }
    Ok(())
}

/// Tests that II will refuse to upgrade on stable memory without persistent state.
#[test]
fn should_trap_on_missing_persistent_state() -> Result<(), CallError> {
    let env = env();
    let canister_id = install_ii_canister(&env, EMPTY_WASM.clone());
    restore_compressed_stable_memory(
        &env,
        canister_id,
        "stable_memory/no-persistent-state-v7.bin.gz",
    );

    let result = upgrade_ii_canister_with_arg(&env, canister_id, II_WASM.clone(), None);

    assert!(result.is_err());
    let err = result.err().unwrap();
    match err {
        CallError::Reject(err) => panic!("unexpected error {err}"),
        CallError::UserError(err) => {
            assert_eq!(err.code, CanisterCalledTrap);
            assert!(err
                .description
                .contains("failed to recover persistent state! Err: NotFound"));
        }
    }
    Ok(())
}
