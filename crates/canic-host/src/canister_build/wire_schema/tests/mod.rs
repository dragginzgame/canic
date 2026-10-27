//! Retained native producer output, with real final-Wasm admission and retry.

use super::*;
use crate::test_support::temp_dir;
use sha2_host::{Digest, Sha256};
use std::fs;

#[test]
#[ignore = "requires CANIC_WIRE_PRODUCER and CANIC_WIRE_FINAL_WASM from a managed artifact"]
fn native_producer_retains_exact_schema_and_refuses_changed_wasm() {
    let producer = std::env::var_os("CANIC_WIRE_PRODUCER").expect("native producer");
    let original = std::env::var_os("CANIC_WIRE_FINAL_WASM").expect("managed final Wasm");
    let root = temp_dir("native-wire-producer");
    fs::create_dir_all(&root).unwrap();
    let wasm = root.join("final.wasm");
    fs::copy(original, &wasm).unwrap();
    let output = root.join("wire-schema.json");
    let cache = root.join("cache");
    let producer = Path::new(&producer);
    let first = produce_wire_schema(&cache, producer, &wasm, &output).unwrap();
    assert!(matches!(first.acquisition, ArtifactCacheOutcome::Built(_)));
    let schema = fs::read(&output).unwrap();
    assert!(!schema.is_empty());
    let bindings_path = output.with_extension("ts");
    let bindings = fs::read(&bindings_path).unwrap();
    assert_eq!(bindings, first.bindings.typescript.as_bytes());
    fs::write(&output, b"corrupted materialized output").unwrap();
    fs::write(&bindings_path, b"corrupted materialized bindings").unwrap();
    let reused = produce_wire_schema(&cache, producer, &wasm, &output).unwrap();
    assert!(matches!(
        reused.acquisition,
        ArtifactCacheOutcome::Reused(_)
    ));
    assert_eq!(first.bindings, reused.bindings);
    assert_eq!(
        first.bindings.schema_sha256,
        <[u8; 32]>::from(Sha256::digest(&schema))
    );
    assert_eq!(fs::read(&output).unwrap(), schema);
    assert_eq!(fs::read(&bindings_path).unwrap(), bindings);
    let mut bytes = fs::read(&wasm).unwrap();
    let facts = inspect(
        &bytes,
        InspectionLimits {
            module_bytes: bytes.len(),
            sections: 1024,
            exports: 4096,
            custom_sections: 1024,
        },
    )
    .unwrap();
    let section = facts
        .custom_sections
        .iter()
        .find(|section| section.name == "icp:private canic:wire")
        .unwrap();
    let offset = section.data.as_ptr() as usize - bytes.as_ptr() as usize;
    // The first record's u64 ID follows its version/kind/name-length header.
    bytes[offset + 4] ^= 1;
    fs::write(&wasm, bytes).unwrap();
    let error = produce_wire_schema(&cache, producer, &wasm, &output).unwrap_err();
    let error = error
        .downcast_ref::<ic_host_process::tool::ToolError>()
        .expect("producer refusal");
    assert!(matches!(
        error.execution_error().unwrap().failure,
        ic_host_process::tool::ExecutionFailure::ExitStatus
    ));
    assert_eq!(fs::read(&output).unwrap(), schema);
    assert_eq!(fs::read(&bindings_path).unwrap(), bindings);
    drop((first, reused));
    fs::remove_dir_all(root).unwrap();
}
