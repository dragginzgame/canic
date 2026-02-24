// Category C - Artifact test (built wasm; no runtime config).

use candid::{Principal, decode_one, encode_one};
use pocket_ic::PocketIcBuilder;
use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

const INSTALL_CYCLES: u128 = 1_000_000_000_000;
const CANISTERS: [&str; 3] = ["intent_authority", "intent_external", "intent_client"];
const PREBUILT_WASM_DIR_ENV: &str = "CANIC_PREBUILT_WASM_DIR";

#[test]
fn intent_race_capacity_one() {
    let workspace_root = workspace_root();
    println!("intent_race: workspace_root={}", workspace_root.display());
    build_canisters(&workspace_root);

    let authority_wasm = read_wasm(&workspace_root, "intent_authority");
    let external_wasm = read_wasm(&workspace_root, "intent_external");
    let client_wasm = read_wasm(&workspace_root, "intent_client");
    println!(
        "intent_race: wasm sizes authority={} external={} client={}",
        authority_wasm.len(),
        external_wasm.len(),
        client_wasm.len()
    );

    let pic = PocketIcBuilder::new().with_application_subnet().build();
    println!("intent_race: PocketIC ready");

    let external_id = pic.create_canister();
    pic.add_cycles(external_id, INSTALL_CYCLES);
    pic.install_canister(external_id, external_wasm, encode_one(()).unwrap(), None);
    println!("intent_race: installed external={external_id}");

    let authority_id = pic.create_canister();
    pic.add_cycles(authority_id, INSTALL_CYCLES);
    pic.install_canister(
        authority_id,
        authority_wasm,
        encode_one(external_id).unwrap(),
        None,
    );
    println!("intent_race: installed authority={authority_id}");

    let client_a = pic.create_canister();
    pic.add_cycles(client_a, INSTALL_CYCLES);
    pic.install_canister(client_a, client_wasm.clone(), encode_one(()).unwrap(), None);
    println!("intent_race: installed client_a={client_a}");

    let client_b = pic.create_canister();
    pic.add_cycles(client_b, INSTALL_CYCLES);
    pic.install_canister(client_b, client_wasm, encode_one(()).unwrap(), None);
    println!("intent_race: installed client_b={client_b}");

    let msg_a = pic
        .submit_call(
            client_a,
            Principal::anonymous(),
            "call_buy",
            encode_one(authority_id).unwrap(),
        )
        .expect("submit call A");
    let msg_b = pic
        .submit_call(
            client_b,
            Principal::anonymous(),
            "call_buy",
            encode_one(authority_id).unwrap(),
        )
        .expect("submit call B");
    println!("intent_race: submitted msg_a={msg_a:?} msg_b={msg_b:?}");

    pic.tick();
    pic.tick();
    println!("intent_race: ticked");

    let res_a = pic.await_call(msg_a).expect("await call A");
    let res_b = pic.await_call(msg_b).expect("await call B");
    println!("intent_race: awaited msg_a msg_b");

    let out_a: Result<(), String> = decode_one(&res_a).expect("decode call A");
    let out_b: Result<(), String> = decode_one(&res_b).expect("decode call B");
    println!("intent_race: results out_a={out_a:?} out_b={out_b:?}");

    let success_count = [out_a.is_ok(), out_b.is_ok()]
        .into_iter()
        .filter(|ok| *ok)
        .count();
    assert_eq!(success_count, 1, "expected exactly one success");
    println!("intent_race: success_count={success_count}");
}

fn build_canisters(workspace_root: &PathBuf) {
    if prebuilt_wasm_dir().is_some() {
        return;
    }

    // Build intent_* canisters for wasm32-unknown-unknown before installing.
    let target_dir = test_target_dir(workspace_root);
    let mut cmd = Command::new("cargo");
    cmd.current_dir(workspace_root);
    cmd.env("CARGO_TARGET_DIR", &target_dir);
    cmd.env("DFX_NETWORK", "local");
    cmd.args(["build", "--release", "--target", "wasm32-unknown-unknown"]);
    for name in CANISTERS {
        cmd.args(["-p", name]);
    }

    let output = cmd.output().expect("failed to run cargo build");
    println!(
        "intent_race: cargo build status={} stdout={} stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "cargo build failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn read_wasm(workspace_root: &Path, crate_name: &str) -> Vec<u8> {
    let wasm_path = wasm_path(workspace_root, crate_name);
    println!(
        "intent_race: read wasm {crate_name} path={}",
        wasm_path.display()
    );
    fs::read(&wasm_path).unwrap_or_else(|err| panic!("failed to read {crate_name} wasm: {err}"))
}

fn wasm_path(workspace_root: &Path, crate_name: &str) -> PathBuf {
    if let Some(dir) = prebuilt_wasm_dir() {
        return dir.join(format!("{crate_name}.wasm"));
    }

    let target_dir = test_target_dir(workspace_root);

    target_dir
        .join("wasm32-unknown-unknown")
        .join("release")
        .join(format!("{crate_name}.wasm"))
}

fn prebuilt_wasm_dir() -> Option<PathBuf> {
    env::var(PREBUILT_WASM_DIR_ENV).ok().map(PathBuf::from)
}

fn test_target_dir(workspace_root: &Path) -> PathBuf {
    workspace_root.join("target").join("pic-wasm")
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .map(PathBuf::from)
        .expect("workspace root")
}
