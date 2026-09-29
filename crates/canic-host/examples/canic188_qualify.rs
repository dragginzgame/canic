//! Local-only qualification of the frozen CANIC-188 Root repair and restoration.
//!
//! Reads retained downstream evidence; all canister effects target an explicit PocketIC server.

use candid::{CandidType, Principal};
use canic_core::dto::{
    error::Error,
    pool_import::{PoolImportIdentity, PoolImportStatus},
};
use canic_host::fleet_ensure::{
    model::{DesiredCanisterKind, DesiredFleet},
    ops::{CanicInitRequest, compile_arguments},
};
use ic_testkit::pic::{PocketIc, PocketIcBuilder, PocketIcBuilderExt, PocketIcStartupConfig};
use serde::Deserialize;
use sha2_host::{Digest, Sha256};
use std::{error::Error as StdError, fs, path::Path, time::Duration};

#[derive(CandidType)]
enum StatusRequest {
    PoolImport(PoolImportIdentity),
}

#[derive(CandidType, Deserialize)]
enum StatusResponse {
    PoolImport(Box<PoolImportStatus>),
}

fn status(
    pic: &PocketIc,
    root: Principal,
    operator: Principal,
    identity: PoolImportIdentity,
) -> PoolImportStatus {
    let bytes = pic
        .query_call(
            root,
            operator,
            "canic_root_status",
            candid::encode_one(StatusRequest::PoolImport(identity)).unwrap(),
        )
        .unwrap();
    let response: Result<StatusResponse, Error> = candid::decode_one(&bytes).unwrap();
    let StatusResponse::PoolImport(status) = response.unwrap();
    *status
}

fn main() -> Result<(), Box<dyn StdError>> {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    if arguments.len() != 2 {
        return Err(
            "usage: canic188_qualify READ_ONLY_TOKO_WORKSPACE CANIC_REPAIR_DIRECTORY".into(),
        );
    }
    let workspace = Path::new(&arguments[0]);
    let repair = Path::new(&arguments[1]);
    let root = Principal::from_text("2ydug-eaaaa-aaaab-qhfca-cai")?;
    let operator =
        Principal::from_text("yafbw-zwrsx-ivoo6-m5alg-hwsf5-f4zfo-wqzsh-yueqw-kfixx-osvof-aqe")?;
    let retained: PoolImportStatus = candid::decode_one(&fs::read(repair.join("status.bin"))?)?;
    let identity = PoolImportIdentity {
        sequence: retained.reservation.sequence,
        plan_sha256: retained.reservation.plan_sha256,
    };
    let initialization = initializer(workspace)?;
    canic_host::canister_build::validate_wasm_candid_endpoints(
        &repair.join("root.wasm"),
        &initialization.original_did,
    )?;
    let server: url::Url = std::env::var("CANIC_POCKET_IC_SERVER_URL")?.parse()?;
    if server.scheme() != "http" || server.host_str() != Some("127.0.0.1") {
        return Err("qualification requires the owned loopback PocketIC server".into());
    }
    let pic = PocketIcBuilder::new().with_application_subnet().try_build(
        PocketIcStartupConfig::connect(server.as_str(), Duration::from_secs(30)),
    )?;
    pic.create_canister_with_id(None, None, root)
        .map_err(std::io::Error::other)?;
    pic.add_cycles(root, 300_700_000_000_000);
    pic.set_controllers(root, None, vec![operator]).unwrap();
    pic.install_canister(
        root,
        fs::read(repair.join("fixture-root.wasm"))?,
        initialization.argument,
        Some(operator),
    );
    let bytes = pic
        .update_call(
            root,
            operator,
            "test_seed_canic188",
            candid::encode_args(())?,
        )
        .unwrap();
    candid::decode_one::<Result<(), Error>>(&bytes)?.unwrap();
    qualify(
        &pic,
        root,
        operator,
        identity,
        &retained,
        repair,
        initialization.original_wasm,
    )
}

/// The fixture supplies historical records; all installation and restoration effects are real.
fn qualify(
    pic: &PocketIc,
    root: Principal,
    operator: Principal,
    identity: PoolImportIdentity,
    retained: &PoolImportStatus,
    repair: &Path,
    original_wasm: Vec<u8>,
) -> Result<(), Box<dyn StdError>> {
    assert_eq!(status(pic, root, operator, identity), *retained);
    pic.stop_canister(root, Some(operator)).unwrap();
    upgrade_ignoring_reply(pic, root, operator, fs::read(repair.join("root.wasm"))?);
    pic.start_canister(root, Some(operator)).unwrap();
    let recovered = status(pic, root, operator, identity);
    assert_eq!(recovered.reservation, retained.reservation);
    assert_eq!(recovered.progress, retained.progress);
    assert_eq!(recovered.paid_calls, retained.paid_calls);
    assert!(recovered.reserved_debit_cycles < retained.reserved_debit_cycles);
    assert_eq!(
        recovered.reserved_debit_cycles,
        retained.reservation.observed_root_cycles - recovered.last_root_cycles
    );
    pic.stop_canister(root, Some(operator)).unwrap();
    assert!(
        pic.upgrade_canister(
            root,
            fs::read(repair.join("root.wasm"))?,
            candid::encode_args(())?,
            Some(operator)
        )
        .is_err()
    );
    pic.start_canister(root, Some(operator)).unwrap();
    assert_eq!(status(pic, root, operator, identity), recovered);
    pic.stop_canister(root, Some(operator)).unwrap();
    upgrade_ignoring_reply(pic, root, operator, original_wasm);
    pic.start_canister(root, Some(operator)).unwrap();
    assert_eq!(status(pic, root, operator, identity), recovered);
    let final_cycles: u128 = pic
        .canister_status(root, Some(operator))
        .unwrap()
        .cycles
        .0
        .try_into()
        .unwrap();
    assert!(final_cycles >= retained.reservation.minimum_root_cycles);
    assert!(
        retained.reservation.observed_root_cycles - final_cycles
            <= retained.reservation.maximum_root_debit_cycles
    );
    println!(
        "CANIC-188: exact evidence, repair, replay rejection and original-artifact restoration passed on PocketIC"
    );
    Ok(())
}

/// Exact read-only compilation of the retained original Root initializer and artifact.
struct Initializer {
    argument: Vec<u8>,
    original_wasm: Vec<u8>,
    original_did: std::path::PathBuf,
}

fn initializer(workspace: &Path) -> Result<Initializer, Box<dyn StdError>> {
    let state_directory = workspace.join(".canic/fleet-ensure/staging/toko-miner-staging-001");
    let receipt: serde_json::Value = serde_json::from_slice(&fs::read(state_directory.join("infrastructure-bootstrap-publications/b81ef78461cbc585891c5565b13af91d0221a6c151fc2032eee302986a2219f6.json"))?)?;
    let desired: DesiredFleet =
        serde_json::from_value(receipt["plan"]["reviewed_desired"]["desired"].clone())?;
    let principals = desired
        .canisters
        .iter()
        .filter_map(|canister| {
            canister
                .principal
                .as_ref()
                .map(|principal| (canister.name.clone(), principal.clone()))
        })
        .collect();
    let canister = desired
        .canisters
        .iter()
        .find(|canister| canister.kind == DesiredCanisterKind::Root)
        .ok_or("Root absent")?;
    let wasm = canister.wasm.as_deref().ok_or("Root artifact absent")?;
    let argument = compile_arguments(&CanicInitRequest {
        desired: &desired,
        init: canister
            .canic_init
            .as_ref()
            .ok_or("Root initializer absent")?,
        operation_id: receipt["plan"]["operation_id"]
            .as_str()
            .ok_or("operation absent")?,
        principals: &principals,
        root: workspace,
        wasm,
        wasm_sha256: "f019558ffac12a214fb68ca68591618d08bdcee6e16858659df24d1ca50f0da6",
    })?;
    Ok(Initializer {
        argument,
        original_wasm: fs::read(workspace.join(wasm))?,
        original_did: workspace.join(wasm).with_extension("did"),
    })
}

/// Recover a lost install reply from exact observed module identity, without a second install.
fn upgrade_ignoring_reply(pic: &PocketIc, root: Principal, operator: Principal, wasm: Vec<u8>) {
    let expected = Sha256::digest(&wasm).to_vec();
    assert_ne!(
        pic.canister_status(root, Some(operator))
            .unwrap()
            .module_hash,
        Some(expected.clone())
    );
    let _lost_reply =
        pic.upgrade_canister(root, wasm, candid::encode_args(()).unwrap(), Some(operator));
    assert_eq!(
        pic.canister_status(root, Some(operator))
            .unwrap()
            .module_hash,
        Some(expected)
    );
}
