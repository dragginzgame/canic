//! Real query execution proves exact arguments and refusal without substituting review balances.

use super::*;
use canic_contracts::ids::CanonicalNetworkId;
use ic_agent::identity::BasicIdentity;
use ic_testkit::pocket_ic::PocketIcBuilder;
use sha2_host::{Digest, Sha256};
use std::{fs, process::Command, time::SystemTime};

fn wire_fixture() -> Vec<u8> {
    let directory = crate::test_support::temp_dir("release-accounts-wire");
    fs::create_dir_all(&directory).unwrap();
    let source = directory.join("fixture.rs");
    let output = directory.join("fixture.wasm");
    fs::write(&source, include_str!("../fixture/mod.rs")).unwrap();
    let result = Command::new("rustc")
        .args([
            "--edition=2024",
            "--crate-type=cdylib",
            "--target=wasm32-unknown-unknown",
            "-O",
        ])
        .arg(&source)
        .arg("-o")
        .arg(&output)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let bytes = fs::read(output).unwrap();
    fs::remove_dir_all(directory).unwrap();
    bytes
}

fn ledger_data(account: &FleetReleaseAccountRecord, reply: Vec<u8>) -> Vec<u8> {
    let argument = candid::encode_one(Account {
        owner: account.owner,
        subaccount: account.subaccount.filter(|value| *value != [0; 32]),
    })
    .unwrap();
    let mut bytes = u32::try_from(argument.len())
        .unwrap()
        .to_le_bytes()
        .to_vec();
    bytes.extend(argument);
    bytes.extend(reply);
    bytes
}

#[test]
#[ignore = "the workspace runner supplies one shared PocketIC server and serial execution"]
#[expect(
    clippy::too_many_lines,
    reason = "one Ledger query journey covers exact accounts, live balances and independent refusals"
)]
fn governed_pocketic_release_accounts_read_exact_balances() {
    let mut pic = crate::test_support::start_pocket_ic(
        PocketIcBuilder::new()
            .with_nns_subnet()
            .with_application_subnet(),
    );
    pic.set_time(SystemTime::now().into());
    let agent = Agent::builder()
        .with_url(pic.make_live(None))
        .with_identity(BasicIdentity::from_raw_key(&[29; 32]))
        .with_max_response_body_size(RESPONSE_BYTES)
        .build()
        .unwrap();
    agent.set_root_key(pic.root_key().unwrap());
    let operator = agent.get_principal().unwrap();
    let ledger = pic.create_canister_with_settings(Some(operator), None);
    let (mut review, _) = fixture();
    // A single known owner keeps the exact-account fixture intentionally small.
    review.sources.truncate(1);
    review.accounts.truncate(1);
    review.authority.operator = operator;
    review.authority.cycles_ledger = ledger;
    review.authority.network_root_key_sha256 = Sha256::digest(agent.read_root_key()).into();
    review.authority.fleet.fleet.canonical_network_id =
        CanonicalNetworkId::from_der_root_trust_anchor(&agent.read_root_key()).unwrap();
    review.accounts[0].ledger = ledger;
    review.accounts[0].subaccount = Some([0; 32]);
    pic.install_canister(
        ledger,
        wire_fixture(),
        ledger_data(
            &review.accounts[0],
            candid::encode_one(Nat::from(0_u8)).unwrap(),
        ),
        Some(operator),
    );
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let collect =
        |review: &FleetReleaseReviewRecord| runtime.block_on(collect_with_agent(&agent, review));
    let result = collect(&review).unwrap();
    assert_eq!(result[0].observed_balance, 0);
    assert_eq!(result[0].subaccount, None);
    assert_ne!(
        result[0].observed_balance,
        review.accounts[0].observed_balance
    );

    for value in [1, u128::MAX] {
        pic.update_call(
            ledger,
            operator,
            "replace",
            ledger_data(
                &review.accounts[0],
                candid::encode_one(Nat::from(value)).unwrap(),
            ),
        )
        .unwrap();
        assert_eq!(collect(&review).unwrap()[0].observed_balance, value);
    }
    let mut changed = review.clone();
    changed.authority.network_root_key_sha256 = [0; 32];
    assert!(matches!(
        collect(&changed),
        Err(ReleaseAccountError::Authentication(
            ReleaseObservationError::Authority
        ))
    ));
    changed = review.clone();
    changed.authority.operator = Principal::anonymous();
    assert!(matches!(
        collect(&changed),
        Err(ReleaseAccountError::Authentication(
            ReleaseObservationError::Authority
        ))
    ));
    changed = review.clone();
    changed.accounts.push(changed.accounts[0].clone());
    assert!(matches!(
        collect(&changed),
        Err(ReleaseAccountError::Inventory(FleetReleaseError::Accounts))
    ));

    // Add a distinct subaccount. Its wrong wire argument is rejected; no partial list escapes.
    changed = review.clone();
    let mut subaccount = changed.accounts[0].clone();
    subaccount.subaccount = Some([7; 32]);
    changed.accounts.push(subaccount);
    assert!(matches!(
        collect(&changed),
        Err(ReleaseAccountError::Observation {
            stage: ReleaseAccountStage::Query,
            ..
        })
    ));
    // The same owner can also hold an explicitly selected nondefault account on another Ledger.
    let other = pic.create_canister_with_settings(Some(operator), None);
    changed.accounts[1].ledger = other;
    pic.install_canister(
        other,
        wire_fixture(),
        ledger_data(
            &changed.accounts[1],
            candid::encode_one(Nat::from(42_u8)).unwrap(),
        ),
        Some(operator),
    );
    let result = collect(&changed).unwrap();
    assert_eq!(result[1].observed_balance, 42);
    assert_eq!(result[1].subaccount, Some([7; 32]));

    for (bytes, expected) in [
        (b"DIDL".to_vec(), ReleaseAccountStage::Decode),
        (
            candid::encode_one(Nat::from(u128::MAX) + Nat::from(1_u8)).unwrap(),
            ReleaseAccountStage::Overflow,
        ),
    ] {
        pic.update_call(
            other,
            operator,
            "replace",
            ledger_data(&changed.accounts[1], bytes),
        )
        .unwrap();
        assert!(
            matches!(collect(&changed), Err(ReleaseAccountError::Observation { stage, .. }) if stage == expected)
        );
    }
}
