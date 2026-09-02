use super::*;
use crate::canister_build::cache::{canister_build_target_root, configure_canister_cargo_command};
use crate::test_support::{start_pocket_ic, temp_dir};
use candid::{CandidType, Deserialize, Nat, Principal, decode_one, encode_one};
use canic_core::ids::{BuildNetwork, ReleaseBuildId, ReleaseBuildNonce};
use ic_testkit::pic::PocketIcBuilder;
use std::path::Path;

#[test]
fn generated_helper_lock_is_an_exact_workspace_lock_projection() {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let metadata = cargo_metadata(&workspace_root, true).expect("read exact workspace graph");
    let canic_package = resolved_canic_package(&metadata).expect("resolve Canic package");
    let dependencies = resolved_wrapper_dependencies(&metadata, canic_package)
        .expect("resolve exact helper dependencies");
    let root = temp_dir("canic-pool-ledger-recovery-lock");
    let wrapper_root = root.join("wrapper");
    fs::create_dir_all(wrapper_root.join("src")).expect("create generated helper fixture");
    fs::write(
        wrapper_root.join("Cargo.toml"),
        render_helper_manifest(&dependencies),
    )
    .expect("write generated helper manifest");
    fs::write(wrapper_root.join("src/lib.rs"), "pub fn helper() {}\n")
        .expect("write generated helper source");
    let context = WorkspaceBuildContext {
        role: POOL_LEDGER_RECOVERY_ROLE.to_string(),
        profile: CanisterBuildProfile::Fast,
        environment: "local".to_string(),
        build_network: BuildNetwork::Local,
        workspace_root: workspace_root.clone(),
        icp_root: root.clone(),
        config_path: workspace_root.join("canic.toml"),
        local_replica: None,
        refresh_canonical_infrastructure_did: false,
        release_build_id: None,
    };
    let workspace_lock = workspace_root.join("Cargo.lock");
    generate_verified_helper_lock(&context, &wrapper_root, &workspace_lock, &dependencies)
        .expect("project and validate the workspace-locked helper graph");

    let helper_lock = wrapper_root.join("Cargo.lock");
    let helper: toml::Value =
        toml::from_str(&fs::read_to_string(helper_lock).expect("read generated helper lock"))
            .expect("decode generated helper lock");
    assert!(helper["package"].as_array().is_some_and(|packages| {
        packages.iter().any(|package| {
            package.get("name").and_then(toml::Value::as_str)
                == Some(GENERATED_WRAPPER_PACKAGE_NAME)
        })
    }));
    fs::remove_dir_all(root).expect("remove generated helper lock fixture");
}

#[test]
fn workspace_seeded_lock_preserves_an_older_compatible_transitive_identity_without_cargo_cache() {
    let root = temp_dir("canic-pool-ledger-recovery-older-transitive");
    let workspace_lock = root.join("workspace.lock");
    let helper_lock = root.join("helper.lock");
    fs::create_dir_all(&root).expect("create older transitive fixture");
    fs::write(
        &workspace_lock,
        r#"version = 4

[[package]]
name = "candid"
version = "0.10.35"

[[package]]
name = "crypto-common"
version = "0.1.6"

[[package]]
name = "ic-cdk"
version = "0.20.2"
dependencies = ["smallvec 1.15.2"]

[[package]]
name = "serde"
version = "1.0.229"

[[package]]
name = "smallvec"
version = "1.15.2"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "8ed6a63f02c8539c91a8685a86f4099661ba3da017932f6ebbea6de3f0fa7c90"
"#,
    )
    .expect("write older workspace lock fixture");
    let dependencies = crate::bootstrap_store::GeneratedWrapperDependencies {
        canic_version: "0.110.1".to_string(),
        candid_version: "0.10.35".to_string(),
        crypto_common_version: "0.1.6".to_string(),
        ic_cdk_version: "0.20.2".to_string(),
        serde_version: "1.0.229".to_string(),
    };
    write_workspace_seeded_helper_lock(&workspace_lock, &helper_lock, &dependencies)
        .expect("write the helper lock without invoking Cargo resolution");
    let helper = fs::read_to_string(&helper_lock).expect("read projected helper lock");
    assert!(helper.contains("version = \"1.15.2\""));
    assert!(!helper.contains("1.16.0"));
    assert!(helper.contains(GENERATED_WRAPPER_PACKAGE_NAME));
    fs::remove_dir_all(root).expect("remove older transitive fixture");
}

#[test]
fn workspace_seeded_lock_reports_every_missing_direct_identity_together() {
    let root = temp_dir("canic-pool-ledger-recovery-missing-identities");
    fs::create_dir_all(&root).expect("create missing-identity fixture");
    let workspace_lock = root.join("workspace.lock");
    let helper_lock = root.join("helper.lock");
    fs::write(
        &workspace_lock,
        "version = 4\n\n[[package]]\nname = \"serde\"\nversion = \"1.0.229\"\n",
    )
    .expect("write incomplete workspace lock fixture");
    let dependencies = crate::bootstrap_store::GeneratedWrapperDependencies {
        canic_version: "0.110.1".to_string(),
        candid_version: "0.10.35".to_string(),
        crypto_common_version: "0.1.6".to_string(),
        ic_cdk_version: "0.20.2".to_string(),
        serde_version: "1.0.229".to_string(),
    };
    let error = write_workspace_seeded_helper_lock(&workspace_lock, &helper_lock, &dependencies)
        .expect_err("every missing direct dependency must reject before Cargo starts");
    let message = error.to_string();
    for missing in ["candid 0.10.35", "crypto-common 0.1.6", "ic-cdk 0.20.2"] {
        assert!(message.contains(missing), "missing report row: {missing}");
    }
    assert!(!helper_lock.exists());
    fs::remove_dir_all(root).expect("remove missing-identity fixture");
}

#[test]
fn helper_metadata_reports_every_identity_outside_the_workspace_lock() {
    let root = temp_dir("canic-pool-ledger-recovery-unlocked-identities");
    fs::create_dir_all(&root).expect("create unlocked-identity fixture");
    let workspace_lock = root.join("workspace.lock");
    fs::write(
        &workspace_lock,
        r#"version = 4

[[package]]
name = "serde"
version = "1.0.229"
source = "registry+https://github.com/rust-lang/crates.io-index"
"#,
    )
    .expect("write workspace lock fixture");
    let helper_metadata = br#"{
        "packages": [
            {
                "name": "canic-generated-pool-ledger-recovery",
                "version": "0.0.0",
                "source": null,
                "manifest_path": "/fixture/Cargo.toml"
            },
            {
                "name": "unexpected-z",
                "version": "2.0.0",
                "source": "registry+https://github.com/rust-lang/crates.io-index",
                "manifest_path": "/cache/unexpected-z/Cargo.toml"
            },
            {
                "name": "unexpected-a",
                "version": "1.0.0",
                "source": "registry+https://github.com/rust-lang/crates.io-index",
                "manifest_path": "/cache/unexpected-a/Cargo.toml"
            }
        ]
    }"#;
    let error = require_helper_metadata_within_workspace(&workspace_lock, helper_metadata)
        .expect_err("every unlocked helper identity must be rejected together");
    let message = error.to_string();
    let first = message
        .find("unexpected-a 1.0.0")
        .expect("first unexpected identity");
    let second = message
        .find("unexpected-z 2.0.0")
        .expect("second unexpected identity");
    assert!(
        first < second,
        "mismatch report must be complete and stable"
    );
    fs::remove_dir_all(root).expect("remove unlocked-identity fixture");
}

#[derive(CandidType)]
struct LedgerInit {
    canister_ids: Vec<Principal>,
    expected_root: Principal,
    expected_subnet: Principal,
    initial_balances: Option<Vec<LedgerBalance>>,
    pending_first_index: Option<u64>,
    withdrawal_fee: Option<Nat>,
}

#[derive(CandidType)]
struct LedgerBalance {
    balance: Nat,
    owner: Principal,
}

#[derive(CandidType)]
struct LedgerAccount {
    owner: Principal,
    subaccount: Option<[u8; 32]>,
}

#[derive(CandidType)]
struct HelperInit {
    amount: Nat,
    canister: Principal,
    created_at_time_ns: u64,
    cycles_ledger: Principal,
    operation_id: [u8; 32],
    root: Principal,
}

#[derive(CandidType, Debug, Deserialize, Eq, PartialEq)]
struct HelperReceipt {
    amount: Nat,
    block_index: Nat,
    canister: Principal,
    operation_id: [u8; 32],
}

#[test]
#[ignore = "the workspace runner supplies one shared PocketIC server and serial execution"]
fn governed_pocketic_generated_pool_ledger_recovery_helper_converts_one_account_exactly_once() {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let icp_root = temp_dir("canic-pool-ledger-recovery-build");
    let release_build_id =
        ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes([39; 32]));
    let context = WorkspaceBuildContext {
        role: POOL_LEDGER_RECOVERY_ROLE.to_string(),
        profile: CanisterBuildProfile::Fast,
        environment: "local".to_string(),
        build_network: BuildNetwork::Local,
        workspace_root: workspace_root.clone(),
        icp_root: icp_root.clone(),
        config_path: workspace_root.join("canic.toml"),
        local_replica: None,
        refresh_canonical_infrastructure_did: false,
        release_build_id: Some(release_build_id),
    };

    let output = build_pool_ledger_recovery_artifact(&context)
        .expect("build the generated pool Ledger recovery helper");
    let candid = fs::read_to_string(&output.did_path).expect("read generated helper Candid");

    assert_eq!(output.protocol_role.as_str(), POOL_LEDGER_RECOVERY_ROLE);
    assert!(output.wasm_gz_path.is_file());
    assert!(candid.contains("authority"));
    assert!(candid.contains("recover"));
    assert!(candid.contains("operation_id"));

    let ledger_wasm = build_cycles_ledger_stub(&workspace_root);
    let helper_wasm = fs::read(&output.wasm_path).expect("read generated recovery helper");
    assert_release_build_identity(&helper_wasm, release_build_id);
    let pic = start_pocket_ic(PocketIcBuilder::new().with_application_subnet());
    let subnet = *pic
        .topology()
        .get_app_subnets()
        .first()
        .expect("PocketIC application Subnet");
    let root = Principal::from_slice(&[31; 29]);
    let pool = pic.create_canister_on_subnet(None, None, subnet);
    pic.set_controllers(pool, None, vec![root])
        .expect("set exact pool controller");
    let initial_native_cycles = 2_000_000_000_000_u128;
    let current_native_cycles = pic.cycle_balance(pool);
    if current_native_cycles < initial_native_cycles {
        pic.add_cycles(pool, initial_native_cycles - current_native_cycles);
    }
    let ledger = pic.create_canister_on_subnet(None, None, subnet);
    pic.add_cycles(ledger, 10_000_000_000_000);
    let fee = 100_000_000_u128;
    let withdrawal = 3_000_000_000_000_u128;
    pic.install_canister(
        ledger,
        ledger_wasm,
        encode_one(LedgerInit {
            canister_ids: vec![pool],
            expected_root: root,
            expected_subnet: subnet,
            initial_balances: Some(vec![LedgerBalance {
                balance: Nat::from(withdrawal + fee),
                owner: pool,
            }]),
            pending_first_index: None,
            withdrawal_fee: Some(Nat::from(fee)),
        })
        .expect("encode Ledger init"),
        None,
    );
    pic.install_canister(
        pool,
        helper_wasm,
        encode_one(HelperInit {
            amount: Nat::from(withdrawal),
            canister: pool,
            created_at_time_ns: 1,
            cycles_ledger: ledger,
            operation_id: [32; 32],
            root,
        })
        .expect("encode helper init"),
        Some(root),
    );

    let before = pic.cycle_balance(pool);
    let first = recover(&pic, pool, root);
    assert_eq!(first.amount, withdrawal);
    assert_eq!(first.canister, pool);
    assert_eq!(first.operation_id, [32; 32]);
    let after_first = pic.cycle_balance(pool);
    assert!(after_first >= before + withdrawal - 100_000_000_000);
    assert_eq!(ledger_balance(&pic, ledger, pool), 0);
    assert_eq!(withdrawal_count(&pic, ledger), 1);

    let replay = recover(&pic, pool, root);
    assert_eq!(replay, first);
    assert_eq!(ledger_balance(&pic, ledger, pool), 0);
    assert_eq!(withdrawal_count(&pic, ledger), 1);
    pic.uninstall_canister(pool, Some(root))
        .expect("uninstall temporary recovery helper");
    let status = pic
        .canister_status(pool, Some(root))
        .expect("query recovered pool status");
    assert!(status.module_hash.is_none());
    assert!(pic.cycle_balance(pool) >= before + withdrawal - 200_000_000_000);

    fs::remove_dir_all(icp_root).expect("clean generated helper test root");
}

fn assert_release_build_identity(wasm: &[u8], release_build_id: ReleaseBuildId) {
    let identity = release_build_id.to_string();
    assert!(
        wasm.windows(identity.len())
            .any(|window| window == identity.as_bytes())
    );
}

fn build_cycles_ledger_stub(workspace_root: &Path) -> Vec<u8> {
    let mut command = cargo_command();
    command.current_dir(workspace_root).args([
        "build",
        "--locked",
        "--package",
        "cycles_ledger_stub",
        "--profile",
        "fast",
        "--target",
        "wasm32-unknown-unknown",
    ]);
    configure_canister_cargo_command(&mut command, workspace_root);
    let output = command.output().expect("run Cycles Ledger stub build");
    assert!(
        output.status.success(),
        "Cycles Ledger stub build failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    fs::read(
        canister_build_target_root(workspace_root)
            .join("wasm32-unknown-unknown/fast/cycles_ledger_stub.wasm"),
    )
    .expect("read Cycles Ledger stub Wasm")
}

fn recover(pic: &pocket_ic::PocketIc, pool: Principal, root: Principal) -> HelperReceipt {
    let response = pic
        .update_call(
            pool,
            root,
            "recover",
            candid::encode_args(()).expect("encode recovery call"),
        )
        .expect("call recovery helper");
    decode_one::<Result<HelperReceipt, String>>(&response)
        .expect("decode recovery receipt")
        .expect("recovery succeeds")
}

fn ledger_balance(pic: &pocket_ic::PocketIc, ledger: Principal, owner: Principal) -> u128 {
    let response = pic
        .query_call(
            ledger,
            Principal::anonymous(),
            "icrc1_balance_of",
            encode_one(LedgerAccount {
                owner,
                subaccount: None,
            })
            .expect("encode Ledger balance query"),
        )
        .expect("query Ledger balance");
    u128::try_from(
        decode_one::<Nat>(&response)
            .expect("decode Ledger balance")
            .0,
    )
    .expect("Ledger balance fits u128")
}

fn withdrawal_count(pic: &pocket_ic::PocketIc, ledger: Principal) -> u64 {
    let response = pic
        .query_call(
            ledger,
            Principal::anonymous(),
            "withdrawal_count",
            candid::encode_args(()).expect("encode withdrawal count query"),
        )
        .expect("query withdrawal count");
    decode_one(&response).expect("decode withdrawal count")
}
