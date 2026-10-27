//! Module: release_set::test_artifact
//!
//! Responsibility: supply a valid declared Wasm/schema pair for release tests.
//! Does not own: runtime execution or production schema generation.
//! Boundary: test-only bytes exercise release admission.
use canic_contracts::protocol::wire_declaration::{
    CompiledMethod, HEADER_BYTES, MethodKind, MethodLimits, SECTION,
};
/// Native schema matching the fixture module declaration.
pub const WIRE_SCHEMA: &[u8] = br#"{"schema_version":1,"browser_modules":{"canic-adapter.ts":"export {};","identities.ts":"export {};","wire-adapter.ts":"export {};","wire-runtime.ts":"export {};"},"types":{"request":{"k":"unit"},"response":{"k":"u","bits":64}},"methods":[{"name":"echo","id":"9001","kind":"query","mode":"query","request":{"k":"ref","name":"request"},"response":{"k":"ref","name":"response"},"requestBytes":128,"requestAllocation":256,"requestNodes":16,"requestDepth":8,"responseBytes":128,"responseAllocation":256,"responseNodes":16,"responseDepth":8}]}"#;

/// Build a valid module with caller-selected identity bytes and one declared query.
pub fn wire_wasm(identity: &[u8]) -> Vec<u8> {
    let mut wasm = b"\0asm\x01\0\0\0".to_vec();
    section(&mut wasm, 1, &[1, 0x60, 0, 0]);
    section(&mut wasm, 3, &[1, 0]);
    let name = b"canister_query echo";
    let mut export = vec![1, u8::try_from(name.len()).unwrap()];
    export.extend_from_slice(name);
    export.extend_from_slice(&[0, 0]);
    section(&mut wasm, 7, &export);
    section(&mut wasm, 10, &[1, 2, 0, 0x0b]);
    let limits = MethodLimits {
        bytes: 128,
        allocation: 256,
        nodes: 16,
        depth: 8,
    };
    let method = CompiledMethod {
        name: "echo",
        kind: MethodKind::Query,
        id: 9001,
        request: limits,
        response: limits,
    };
    let bytes = method.encode::<{ HEADER_BYTES + 4 }>();
    custom(&mut wasm, SECTION.as_bytes(), &bytes);
    custom(&mut wasm, b"test-identity", identity);
    wasm
}
fn custom(wasm: &mut Vec<u8>, name: &[u8], bytes: &[u8]) {
    let mut data = Vec::new();
    size(&mut data, name.len());
    data.extend_from_slice(name);
    data.extend_from_slice(bytes);
    section(wasm, 0, &data);
}
fn section(wasm: &mut Vec<u8>, id: u8, data: &[u8]) {
    wasm.push(id);
    size(wasm, data.len());
    wasm.extend_from_slice(data);
}
fn size(bytes: &mut Vec<u8>, mut value: usize) {
    loop {
        let mut byte = u8::try_from(value & 127).unwrap();
        value >>= 7;
        if value != 0 {
            byte |= 128;
        }
        bytes.push(byte);
        if value == 0 {
            break;
        }
    }
}

/// Write retained application evidence for offline declaration and binding tests.
pub fn write_declaration_application(
    root: &std::path::Path,
    release: canic_contracts::ids::ReleaseBuildId,
    directory: &std::path::Path,
    wire_schema: &[u8],
) -> [u8; 32] {
    use crate::release_set::{ApplicationArtifactEntry, ApplicationArtifactUnion};
    use sha2_host::{Digest, Sha256};
    use std::fs;
    let topology = canic_core::bootstrap::parse_config_model(
        r#"
[app]
name = "demo"
[roles.root]
kind = "root"
[roles.app]
kind = "canister"
package = "app"
[component_specs.app]
component_role = "app"
maximum_instances = 1
"#,
    )
    .unwrap()
    .compile_component_topology()
    .unwrap();
    let candid = b"service : { submit : (blob) -> (); }\n";
    let wasm = format!(".canic/release-builds/{release}/artifacts/app/app.wasm");
    let did = root.join(&wasm).with_extension("did");
    fs::create_dir_all(did.parent().unwrap()).unwrap();
    fs::write(&did, candid).unwrap();
    fs::write(did.with_extension("wire.json"), wire_schema).unwrap();
    let application = ApplicationArtifactUnion {
        release_build_id: release,
        fleet_component_topology_digest: topology.digest().unwrap(),
        entries: vec![ApplicationArtifactEntry {
            role: "app".into(),
            package: "app".into(),
            release_build_id: release,
            wasm_gz_relative_path: format!("{wasm}.gz"),
            wasm_relative_path: wasm,
            wasm_size_bytes: 1,
            wasm_gz_size_bytes: 1,
            wasm_sha256_hex: "01".repeat(32),
            wasm_gz_sha256_hex: "02".repeat(32),
            wire_schema_sha256: Sha256::digest(wire_schema).into(),
            candid_sha256: Sha256::digest(candid).into(),
            protocol_profile_digest: canic_core::role_contract::derive_protocol_profile_hashes(
                "test-release",
                &"app".into(),
                &std::collections::BTreeSet::default(),
                wire_schema,
            )
            .protocol_profile_digest,
        }],
    };
    fs::write(
        directory.join("application-artifact-union.json"),
        application.canonical_bytes(&topology).unwrap(),
    )
    .unwrap();
    application.digest(&topology).unwrap()
}
