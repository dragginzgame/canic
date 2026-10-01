//! Production management routing and lost-response reconciliation against PocketIC.

use super::*;
use crate::{
    fleet_ensure::model::EffectState,
    test_support::{start_pocket_ic, synthetic_identity_pem},
};
use ic_agent::{Identity, identity::BasicIdentity};
use ic_testkit::pocket_ic::{CanisterSettings, PocketIcBuilder};
use std::{fs, sync::Mutex};

#[test]
#[ignore = "the workspace runner supplies one shared PocketIC server and serial execution"]
#[expect(
    clippy::too_many_lines,
    reason = "one management journey binds refusal, retained intent, lost response and conserved custody"
)]
fn governed_pocketic_snapshot_removal_recovers_lost_reply() {
    let mut fixture = ProtocolOwnersFixture::new();
    let pem = synthetic_identity_pem(7);
    let operator = BasicIdentity::from_pem(&pem).unwrap().sender().unwrap();
    fs::write(fixture.root.join("identity.pem"), pem).unwrap();
    fs::write(fixture.root.join("principal"), operator.to_text()).unwrap();
    fs::write(
        fixture.root.join("icp"),
        crate::test_support::tool_script(
            r#"#!/bin/sh
set -eu
case " $* " in
  *" --version "*) echo 'icp @ICP_VERSION@';;
  *" identity export "*) cat identity.pem;;
  *" identity principal "*) cat principal;;
  *) exit 2;;
esac
"#,
        ),
    )
    .unwrap();
    let mut pic = start_pocket_ic(
        PocketIcBuilder::new()
            .with_nns_subnet()
            .with_application_subnet(),
    );
    pic.set_time(SystemTime::now().into());
    let canister = pic.create_canister_with_settings(Some(operator), None);
    pic.install_canister(
        canister,
        b"\0asm\x01\0\0\0".to_vec(),
        vec![],
        Some(operator),
    );
    pic.stop_canister(canister, Some(operator)).unwrap();
    let snapshot = pic
        .take_canister_snapshot(canister, Some(operator), None)
        .unwrap();
    let before = pic.canister_status(canister, Some(operator)).unwrap();
    let subnet = pic.get_subnet(canister).unwrap();
    let url = pic.make_live(None);
    let requests = Arc::new(Mutex::new(Vec::new()));
    let captured = Arc::clone(&requests);
    fixture.platform.desired.operator = operator.to_text();
    fixture.platform = fixture
        .platform
        .with_identity(Some("snapshot-fixture"))
        .with_local_replica(LocalReplicaTarget {
            environment: "local".into(),
            root_key: hex_bytes(pic.root_key().unwrap()),
            url: url.to_string(),
        })
        .with_request_timing_handler(move |request| {
            captured.lock().unwrap().push(request);
        });
    let action = EnsureAction::DeleteSnapshot {
        name: "root".into(),
        principal: canister.to_text(),
        snapshot_id: hex_bytes(&snapshot.id),
        expected_snapshots: vec![hex_bytes(&snapshot.id)],
        expected_module_sha256: before.module_hash.clone().map(hex_bytes),
    };
    let record = EffectRecord {
        maintenance_attempts: 0,
        publication_attempts: 0,
        action_sha256: crate::fleet_ensure::ops::action_sha256(&action),
        created_principal: None,
        destination_post_cycles: None,
        destination_pre_cycles: None,
        post_cycles: None,
        pre_cycles: None,
        pre_canister_version: None,
        progress_identity: None,
        receipt: None,
        state: EffectState::Intent,
    };
    let intent_path = fixture.root.join("snapshot-intent.json");
    crate::fleet_ensure::ops::write_current(&intent_path, &record).unwrap();
    assert!(
        !fixture
            .platform
            .observe_effect("release", &action, &record, &fixture.state)
            .unwrap()
            .applied
    );
    pic.start_canister(canister, Some(operator)).unwrap();
    assert!(matches!(
        fixture
            .platform
            .apply("release", &action, &record, &fixture.state),
        Err(IcpEnsurePlatformError::SnapshotRemoval(_))
    ));
    pic.stop_canister(canister, Some(operator)).unwrap();
    pic.update_canister_settings(
        canister,
        Some(operator),
        CanisterSettings {
            controllers: Some(vec![operator, Principal::from_slice(&[9])]),
            ..CanisterSettings::default()
        },
    )
    .unwrap();
    assert!(matches!(
        fixture
            .platform
            .apply("release", &action, &record, &fixture.state),
        Err(IcpEnsurePlatformError::SnapshotRemoval(_))
    ));
    pic.update_canister_settings(
        canister,
        Some(operator),
        CanisterSettings {
            controllers: Some(vec![operator]),
            ..CanisterSettings::default()
        },
    )
    .unwrap();
    let mut changed = action.clone();
    let EnsureAction::DeleteSnapshot {
        expected_snapshots, ..
    } = &mut changed
    else {
        unreachable!()
    };
    expected_snapshots.push("ff".into());
    expected_snapshots.sort();
    assert!(matches!(
        fixture
            .platform
            .apply("release", &changed, &record, &fixture.state),
        Err(IcpEnsurePlatformError::SnapshotRemoval(_))
    ));
    // Discard the successful reply: the retained journal remains at Intent.
    fixture
        .platform
        .apply("release", &action, &record, &fixture.state)
        .unwrap();
    let record: EffectRecord = crate::fleet_ensure::ops::read_current(&intent_path)
        .unwrap()
        .unwrap();
    assert_eq!(record.state, EffectState::Intent);
    assert!(
        fixture
            .platform
            .observe_effect("release", &action, &record, &fixture.state)
            .unwrap()
            .applied
    );
    fixture
        .platform
        .apply("release", &action, &record, &fixture.state)
        .unwrap();
    assert_eq!(
        requests
            .lock()
            .unwrap()
            .iter()
            .filter(
                |request| request.method.as_deref() == Some("delete_canister_snapshot")
                    && request.succeeded == Some(true)
            )
            .count(),
        1
    );
    assert!(
        pic.list_canister_snapshots(canister, Some(operator))
            .unwrap()
            .is_empty()
    );
    let after = pic.canister_status(canister, Some(operator)).unwrap();
    assert_eq!(after.module_hash, before.module_hash);
    assert_eq!(after.settings.controllers, [operator]);
    assert_eq!(pic.get_subnet(canister), Some(subnet));
    assert!(after.cycles.0 <= before.cycles.0);
    assert!(before.cycles.0 - after.cycles.0 < candid::Nat::from(100_000_000_000_u64).0);
    fs::remove_dir_all(fixture.root).unwrap();
}
