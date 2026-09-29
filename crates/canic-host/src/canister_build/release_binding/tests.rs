//! Qualification of fixed-width binding, rejected templates and installed runtime reads.

use super::*;
use canic_core::ids::ReleaseBuildNonce;

fn identity(byte: u8) -> ReleaseBuildId {
    ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes([byte; 32]))
}

fn template_slot() -> Vec<u8> {
    [
        RELEASE_BINDING_PREFIX,
        &[b'?'; RELEASE_BINDING_ID_BYTES],
        RELEASE_BINDING_SUFFIX,
    ]
    .concat()
}

fn leb(mut value: usize) -> Vec<u8> {
    let mut encoded = Vec::new();
    loop {
        let next = u8::try_from(value & 0x7f).unwrap();
        value >>= 7;
        encoded.push(next | if value == 0 { 0 } else { 0x80 });
        if value == 0 {
            return encoded;
        }
    }
}

fn wasm(data: &[u8], passive: bool) -> Vec<u8> {
    let mut payload = if passive {
        vec![1, 1]
    } else {
        vec![1, 0, 0x41, 0, 0x0b]
    };
    payload.extend(leb(data.len()));
    payload.extend(data);
    [
        b"\0asm\x01\0\0\0".as_slice(),
        &[11],
        &leb(payload.len()),
        &payload,
    ]
    .concat()
}

#[test]
fn one_template_binds_distinct_releases_without_changing_code_or_layout() {
    let template = wasm(&template_slot(), false);
    let slot = find_slot(&template).unwrap();
    let mut first = template.clone();
    let mut second = template.clone();
    bind_release_build_id(&mut first, identity(1)).unwrap();
    bind_release_build_id(&mut second, identity(2)).unwrap();
    assert_ne!(first, second);
    for (bound, id) in [(&first, identity(1)), (&second, identity(2))] {
        assert_eq!(bound.len(), template.len());
        assert_eq!(&bound[..slot.start], &template[..slot.start]);
        assert_eq!(&bound[slot.end..], &template[slot.end..]);
        assert_eq!(&bound[slot.clone()], id.to_string().as_bytes());
    }
    let original = first.clone();
    for id in [identity(1), identity(2)] {
        assert_eq!(
            bind_release_build_id(&mut first, id),
            Err(ReleaseBindingError::AlreadyBound)
        );
        assert_eq!(first, original);
    }
}

#[test]
fn invalid_templates_are_rejected_before_mutation() {
    let slot = template_slot();
    for (mut candidate, expected) in [
        (vec![], ReleaseBindingError::InvalidWasm),
        (wasm(b"no binding", false), ReleaseBindingError::MissingSlot),
        (wasm(&slot, true), ReleaseBindingError::MissingSlot),
        (
            wasm(&[slot.as_slice(), &slot].concat(), false),
            ReleaseBindingError::MultipleSlots,
        ),
        (
            wasm(&slot[..slot.len() - 1], false),
            ReleaseBindingError::MissingSlot,
        ),
        (
            {
                let mut value = wasm(&slot, false);
                value.pop();
                value
            },
            ReleaseBindingError::InvalidWasm,
        ),
    ] {
        let before = candidate.clone();
        assert_eq!(
            bind_release_build_id(&mut candidate, identity(1)),
            Err(expected)
        );
        assert_eq!(candidate, before);
    }
}

#[test]
#[ignore = "governed PocketIC proof compiles and installs one reusable template"]
fn governed_pocketic_release_binding_retains_runtime_identity() {
    use crate::{
        artifact_io::{WasmArtifactFinalization, finalize_wasm_artifact},
        build_toolchain::BuildToolchain,
        canister_build::CanisterBuildProfile,
    };
    use canic_core::ids::BuildNetwork;
    use ic_testkit::pic::PocketIcBuilder;
    use std::fs;

    let root = crate::test_support::temp_dir("release-binding");
    let template_path = compile_template(&root);
    let template = fs::read(&template_path).unwrap();
    let pic =
        crate::test_support::start_pocket_ic(PocketIcBuilder::new().with_application_subnet());
    for profile in [CanisterBuildProfile::Fast, CanisterBuildProfile::Release] {
        let tools = BuildToolchain::resolve(profile).unwrap();
        for id in [identity(1), identity(2)] {
            let wasm_path = root.join(format!("{}-{id}.wasm", profile.target_dir_name()));
            let did_path = wasm_path.with_extension("did");
            let gzip_path = wasm_path.with_extension("wasm.gz");
            finalize_wasm_artifact(
                &WasmArtifactFinalization {
                    release_build_id: Some(id),
                    profile,
                    build_network: BuildNetwork::Local,
                    embed_candid: true,
                    validate_sidecar_only: false,
                    source_wasm_path: &template_path,
                    candid: b"service : { binding : () -> (text) query }",
                    wasm_path: &wasm_path,
                    did_path: &did_path,
                    wasm_gz_path: &gzip_path,
                },
                &tools,
            )
            .unwrap();
            assert_eq!(fs::read(&template_path).unwrap(), template);
            let bound = fs::read(&wasm_path).unwrap();
            assert_runtime_identity(&pic, &bound, &template, id);
        }
    }
    fs::remove_dir_all(root).unwrap();
}

fn compile_template(root: &std::path::Path) -> std::path::PathBuf {
    use std::fs;
    fs::create_dir_all(root).unwrap();
    let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../canic-core/src/bootstrap/release_binding/mod.rs");
    fs::copy(source, root.join("release_binding.rs")).unwrap();
    fs::write(root.join("lib.rs"), include_str!("probe.rs.txt")).unwrap();
    fs::write(
        root.join("Cargo.toml"),
        r#"
[package]
name = "release-binding-probe"
version = "0.0.0"
edition = "2024"
[workspace]
[lib]
path = "lib.rs"
crate-type = ["cdylib"]
[profile.release]
opt-level = "z"
lto = true
"#,
    )
    .unwrap();
    fs::write(
        root.join("build.rs"),
        r#"fn main() {
        println!("cargo:rerun-if-env-changed=CANIC_RELEASE_BUILD_ID");
    }"#,
    )
    .unwrap();
    assert!(!compile_probe(root, identity(1)));
    let template_path =
        root.join("target/wasm32-unknown-unknown/release/release_binding_probe.wasm");
    let original = fs::read(&template_path).unwrap();
    assert!(
        compile_probe(root, identity(2)),
        "new release must reuse Cargo output"
    );
    assert_eq!(fs::read(&template_path).unwrap(), original);
    let changed = format!(
        "{}\n// A source edit must still rebuild.\n",
        include_str!("probe.rs.txt")
    );
    fs::write(root.join("lib.rs"), changed).unwrap();
    assert!(!compile_probe(root, identity(2)));
    let template = fs::read(&template_path).unwrap();
    assert!(find_slot(&template).is_ok());
    template_path
}

fn assert_runtime_identity(
    pic: &ic_testkit::pocket_ic::PocketIc,
    bound: &[u8],
    template: &[u8],
    id: ReleaseBuildId,
) {
    let canister = pic.create_canister();
    pic.add_cycles(canister, 5_000_000_000_000);
    pic.install_canister(canister, bound.to_vec(), id.to_string().into_bytes(), None);
    let reply = pic
        .query_call(
            canister,
            candid::Principal::anonymous(),
            "binding",
            candid::encode_args(()).unwrap(),
        )
        .unwrap();
    assert_eq!(
        candid::decode_one::<String>(&reply).unwrap(),
        id.to_string()
    );
    // A mismatched initializer must not replace a working installation.
    let rejected = pic
        .reinstall_canister(
            canister,
            bound.to_vec(),
            identity(3).to_string().into_bytes(),
            None,
        )
        .unwrap_err();
    assert_eq!(
        rejected.reject_code,
        ic_testkit::pocket_ic::RejectCode::CanisterError
    );
    let unbound = pic
        .reinstall_canister(
            canister,
            template.to_vec(),
            id.to_string().into_bytes(),
            None,
        )
        .unwrap_err();
    assert_eq!(
        unbound.reject_code,
        ic_testkit::pocket_ic::RejectCode::CanisterError
    );
    pic.reinstall_canister(canister, bound.to_vec(), id.to_string().into_bytes(), None)
        .unwrap();
    let reply = pic
        .query_call(
            canister,
            candid::Principal::anonymous(),
            "binding",
            candid::encode_args(()).unwrap(),
        )
        .unwrap();
    assert_eq!(
        candid::decode_one::<String>(&reply).unwrap(),
        id.to_string()
    );
}

fn compile_probe(root: &std::path::Path, release_build_id: ReleaseBuildId) -> bool {
    use crate::canister_build::{CanisterBuildProfile, WorkspaceBuildContext};
    use canic_core::ids::BuildNetwork;
    use std::process::Command;

    let context = WorkspaceBuildContext {
        role: "probe".into(),
        profile: CanisterBuildProfile::Release,
        environment: "local".into(),
        build_network: BuildNetwork::Local,
        workspace_root: root.into(),
        icp_root: root.into(),
        config_path: root.join("canic.toml"),
        local_replica: None,
        refresh_canonical_infrastructure_did: false,
        release_build_id: Some(release_build_id),
    };
    let mut command = Command::new("cargo");
    command
        .current_dir(root)
        .args([
            "build",
            "--offline",
            "--release",
            "--target",
            "wasm32-unknown-unknown",
            "--message-format=json",
        ])
        .env("CARGO_TARGET_DIR", root.join("target"));
    context.apply_to_command(&mut command);
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let artifact = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .find(|value| {
            value["reason"] == "compiler-artifact"
                && value["target"]["name"] == "release_binding_probe"
        })
        .expect("probe artifact");
    artifact["fresh"].as_bool().unwrap()
}

#[test]
fn overlapping_data_initializers_are_rejected_before_binding() {
    let slot = template_slot();
    for offset in [
        0,
        1,
        RELEASE_BINDING_PREFIX.len(),
        slot.len() - 1,
        slot.len(),
        slot.len() + 1,
    ] {
        let mut payload = vec![2, 0, 0x41, 0, 0x0b];
        payload.extend(leb(slot.len()));
        payload.extend(&slot);
        // The later segment may touch, follow or overwrite the binding allocation.
        payload.extend([
            0,
            0x41,
            u8::try_from(offset & 0x7f).unwrap() | 0x80,
            u8::try_from(offset >> 7).unwrap(),
            0x0b,
            1,
            b'!',
        ]);
        let mut candidate = [
            b"\0asm\x01\0\0\0".as_slice(),
            &[11],
            &leb(payload.len()),
            &payload,
        ]
        .concat();
        let original = candidate.clone();
        let result = bind_release_build_id(&mut candidate, identity(1));
        if offset < slot.len() {
            assert_eq!(result, Err(ReleaseBindingError::InvalidWasm));
            assert_eq!(candidate, original);
        } else {
            result.unwrap();
        }
    }
}
