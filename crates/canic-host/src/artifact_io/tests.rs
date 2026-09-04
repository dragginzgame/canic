use super::*;
use canic_core::ids::BuildNetwork;
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

#[test]
fn ic_code_limit_failure_preserves_the_published_artifact_set() {
    let root = unique_temp_dir("canic-artifact-set-ic-limit");
    fs::create_dir_all(&root).expect("create temp dir");
    let source_wasm_path = root.join("source.wasm");
    let wasm_path = root.join("app.wasm");
    let did_path = root.join("app.did");
    let wasm_gz_path = root.join("app.wasm.gz");
    let oversized =
        wasm_with_code_section_size(super::wasm::CURRENT_IC_MAINNET_CODE_SECTION_LIMIT_BYTES + 1);
    fs::write(&source_wasm_path, oversized).expect("write oversized source Wasm");
    fs::write(&wasm_path, b"previous wasm").expect("write previous Wasm");
    fs::write(&did_path, b"previous candid").expect("write previous Candid");
    fs::write(&wasm_gz_path, b"previous gzip").expect("write previous gzip");

    let finalization = WasmArtifactFinalization {
        profile: CanisterBuildProfile::Fast,
        build_network: BuildNetwork::Ic,
        embed_candid: false,
        validate_sidecar_only: false,
        source_wasm_path: &source_wasm_path,
        candid: b"service : {}",
        wasm_path: &wasm_path,
        did_path: &did_path,
        wasm_gz_path: &wasm_gz_path,
    };
    let error = stage_and_publish_artifact_set(&finalization, |staged| {
        enforce_wasm_code_section_limit(BuildNetwork::Ic, &staged.wasm_path)?;
        write_gzip_artifact(&staged.wasm_path, &staged.wasm_gz_path)?;
        Ok(())
    })
    .expect_err("IC-bound oversized Wasm must reject");

    assert!(error.to_string().contains("IC mainnet Wasm artifact"));
    assert_eq!(
        fs::read(&wasm_path).expect("read previous Wasm"),
        b"previous wasm"
    );
    assert_eq!(
        fs::read(&did_path).expect("read previous Candid"),
        b"previous candid"
    );
    assert_eq!(
        fs::read(&wasm_gz_path).expect("read previous gzip"),
        b"previous gzip"
    );
    assert_no_artifact_stage(&root);
    fs::remove_dir_all(root).expect("remove temp root");
}

#[test]
fn local_build_publishes_wasm_above_the_current_ic_mainnet_limit() {
    let root = unique_temp_dir("canic-artifact-set-local-limit");
    fs::create_dir_all(&root).expect("create temp dir");
    let source_wasm_path = root.join("source.wasm");
    let wasm_path = root.join("app.wasm");
    let did_path = root.join("app.did");
    let wasm_gz_path = root.join("app.wasm.gz");
    let oversized =
        wasm_with_code_section_size(super::wasm::CURRENT_IC_MAINNET_CODE_SECTION_LIMIT_BYTES + 1);
    fs::write(&source_wasm_path, &oversized).expect("write oversized source Wasm");

    let finalization = WasmArtifactFinalization {
        profile: CanisterBuildProfile::Fast,
        build_network: BuildNetwork::Local,
        embed_candid: false,
        validate_sidecar_only: false,
        source_wasm_path: &source_wasm_path,
        candid: b"service : {}",
        wasm_path: &wasm_path,
        did_path: &did_path,
        wasm_gz_path: &wasm_gz_path,
    };
    stage_and_publish_artifact_set(&finalization, |staged| {
        enforce_wasm_code_section_limit(BuildNetwork::Local, &staged.wasm_path)?;
        write_gzip_artifact(&staged.wasm_path, &staged.wasm_gz_path)?;
        Ok(())
    })
    .expect("local oversized Wasm must publish");

    assert_eq!(
        fs::read(&wasm_path).expect("read published Wasm"),
        oversized
    );
    assert_eq!(
        fs::read(&did_path).expect("read published Candid"),
        b"service : {}"
    );
    assert!(fs::metadata(&wasm_gz_path).expect("gzip metadata").len() > 0);
    assert_no_artifact_stage(&root);
    fs::remove_dir_all(root).expect("remove temp root");
}

// Replace the source artifact only after a successful shrink command.
#[cfg(unix)]
#[test]
fn successful_ic_wasm_shrink_replaces_artifact() {
    let root = unique_temp_dir("canic-successful-ic-wasm-shrink");
    fs::create_dir_all(&root).expect("create temp dir");
    let wasm_path = root.join("test.wasm");
    let command_path = root.join("ic-wasm");
    fs::write(&wasm_path, b"original wasm").expect("write wasm placeholder");
    write_executable(
        &command_path,
        "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then printf 'ic-wasm 0.11.1\\n'; exit 0; fi\nprintf 'shrunk wasm' > \"$3\"\n",
    );
    let tool = crate::ic_wasm::resolve_test_ic_wasm(&command_path.display().to_string())
        .expect("admit fake pinned ic-wasm");

    let transform =
        shrink_wasm_artifact(&tool, &wasm_path).expect("successful shrink should replace artifact");

    assert_eq!(
        fs::read(&wasm_path).expect("read shrunk wasm"),
        b"shrunk wasm"
    );
    assert_eq!(transform.tool_version.as_deref(), Some("ic-wasm 0.11.1"));
    assert_eq!(transform.tool_sha256, None);
    assert_eq!(transform.outcome, ArtifactTransformOutcome::Applied);
    fs::remove_dir_all(root).expect("remove temp root");
}

// Reject a present failing tool without exposing its partial output.
#[cfg(unix)]
#[test]
fn failed_ic_wasm_shrink_preserves_original_and_removes_partial_output() {
    let root = unique_temp_dir("canic-failed-ic-wasm-shrink");
    fs::create_dir_all(&root).expect("create temp dir");
    let wasm_path = root.join("test.wasm");
    let shrunk_path = wasm_path.with_extension("wasm.shrunk");
    let command_path = root.join("ic-wasm");
    fs::write(&wasm_path, b"original wasm").expect("write wasm placeholder");
    write_executable(
        &command_path,
        "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then printf 'ic-wasm 0.11.1\\n'; exit 0; fi\nprintf 'partial wasm' > \"$3\"\nprintf 'shrink failed' >&2\nexit 23\n",
    );
    let tool = crate::ic_wasm::resolve_test_ic_wasm(&command_path.display().to_string())
        .expect("admit fake pinned ic-wasm");

    shrink_wasm_artifact(&tool, &wasm_path).expect_err("non-zero shrink command must fail");

    assert_eq!(
        fs::read(&wasm_path).expect("read original wasm"),
        b"original wasm"
    );
    assert!(!shrunk_path.exists());
    fs::remove_dir_all(root).expect("remove temp root");
}

#[test]
fn sidecar_only_runtime_matches_declared_methods_without_candid_payload() {
    let root = unique_temp_dir("canic-sidecar-only-candid");
    fs::create_dir_all(&root).expect("create temp dir");
    let wasm_path = root.join("test.wasm");
    let did_path = root.join("test.did");
    fs::write(
        &wasm_path,
        minimal_wasm_with_exports(&[
            "canister_query read",
            "canister_update write",
            "canister_update <ic-cdk internal> timer_executor",
        ]),
    )
    .expect("write runtime Wasm");
    fs::write(
        &did_path,
        b"service : { read : () -> (text) query; write : () -> (); }",
    )
    .expect("write Candid sidecar");

    validate_sidecar_only_candid_artifact(&wasm_path, &did_path)
        .expect("sidecar and runtime methods must agree");

    fs::remove_dir_all(root).expect("remove temp dir");
}

#[test]
fn sidecar_only_runtime_rejects_embedded_or_mismatched_candid() {
    let root = unique_temp_dir("canic-sidecar-only-candid-reject");
    fs::create_dir_all(&root).expect("create temp dir");
    let wasm_path = root.join("test.wasm");
    let did_path = root.join("test.did");
    fs::write(&did_path, b"service : { read : () -> () query; }").expect("write Candid sidecar");

    fs::write(&wasm_path, minimal_wasm_with_contract()).expect("write metadata Wasm");
    validate_sidecar_only_candid_artifact(&wasm_path, &did_path)
        .expect_err("embedded metadata must fail sidecar-only validation");

    fs::write(
        &wasm_path,
        minimal_wasm_with_exports(&["canister_query other"]),
    )
    .expect("write mismatched runtime Wasm");
    validate_sidecar_only_candid_artifact(&wasm_path, &did_path)
        .expect_err("mismatched runtime method must fail");

    fs::write(
        &wasm_path,
        minimal_wasm_with_exports(&["canister_query read", "get_candid_pointer"]),
    )
    .expect("write declaration export Wasm");
    validate_sidecar_only_candid_artifact(&wasm_path, &did_path)
        .expect_err("declaration pointer must fail");

    fs::remove_dir_all(root).expect("remove temp dir");
}

#[cfg(unix)]
#[test]
fn successful_ic_wasm_metadata_records_tool_identity() {
    let root = unique_temp_dir("canic-successful-ic-wasm-metadata");
    fs::create_dir_all(&root).expect("create temp dir");
    let wasm_path = root.join("test.wasm");
    let did_path = root.join("test.did");
    let command_path = root.join("ic-wasm");
    fs::write(&wasm_path, b"\0asm").expect("write wasm placeholder");
    fs::write(&did_path, b"service : {}").expect("write did placeholder");
    write_executable(
        &command_path,
        "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then printf 'ic-wasm 0.11.1\\n'; fi\n",
    );
    let tool = crate::ic_wasm::resolve_test_ic_wasm(&command_path.display().to_string())
        .expect("admit fake pinned ic-wasm");

    let transform =
        embed_candid_metadata(&tool, &wasm_path, &did_path).expect("successful metadata transform");

    assert_eq!(transform.transform, ArtifactTransformKind::CandidMetadata);
    assert_eq!(transform.tool_version.as_deref(), Some("ic-wasm 0.11.1"));
    assert_eq!(transform.outcome, ArtifactTransformOutcome::Applied);

    fs::remove_dir_all(root).expect("remove temp dir");
}

#[cfg(unix)]
#[test]
fn failed_ic_wasm_metadata_is_rejected() {
    let root = unique_temp_dir("canic-failed-ic-wasm-metadata");
    fs::create_dir_all(&root).expect("create temp dir");
    let wasm_path = root.join("test.wasm");
    let did_path = root.join("test.did");
    let command_path = root.join("ic-wasm");
    fs::write(&wasm_path, b"\0asm").expect("write wasm placeholder");
    fs::write(&did_path, b"service : {}").expect("write did placeholder");
    write_executable(
        &command_path,
        "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then printf 'ic-wasm 0.11.1\\n'; exit 0; fi\nexit 23\n",
    );
    let tool = crate::ic_wasm::resolve_test_ic_wasm(&command_path.display().to_string())
        .expect("admit fake pinned ic-wasm");

    embed_candid_metadata(&tool, &wasm_path, &did_path)
        .expect_err("non-zero metadata command must fail");

    fs::remove_dir_all(root).expect("remove temp dir");
}

#[test]
fn fast_wasm_does_not_request_binaryen_optimization() {
    let root = unique_temp_dir("canic-fast-wasm-opt");
    fs::create_dir_all(&root).expect("create temp dir");
    let wasm_path = root.join("test.wasm");
    fs::write(&wasm_path, minimal_wasm_with_contract()).expect("write Wasm fixture");

    let transform = optimize_release_wasm_artifact_with_command(
        &root.join("missing-wasm-opt").display().to_string(),
        CanisterBuildProfile::Fast,
        &wasm_path,
    )
    .expect("fast builds must not invoke Binaryen");

    assert_eq!(transform.transform, ArtifactTransformKind::Optimize);
    assert_eq!(transform.outcome, ArtifactTransformOutcome::NotRequested);
    assert_eq!(transform.tool_version, None);
    assert_eq!(transform.tool_sha256, None);
    assert_eq!(transform.metrics, None);
    fs::remove_dir_all(root).expect("remove temp root");
}

#[test]
fn release_wasm_fails_closed_when_binaryen_is_missing() {
    let root = unique_temp_dir("canic-missing-wasm-opt");
    fs::create_dir_all(&root).expect("create temp dir");
    let wasm_path = root.join("test.wasm");
    let original = minimal_wasm_with_contract();
    fs::write(&wasm_path, &original).expect("write Wasm fixture");

    let error = optimize_release_wasm_artifact_with_command(
        &root.join("missing-wasm-opt").display().to_string(),
        CanisterBuildProfile::Release,
        &wasm_path,
    )
    .expect_err("release build must reject without Binaryen");

    assert!(matches!(
        error.downcast_ref::<crate::binaryen::BinaryenToolError>(),
        Some(crate::binaryen::BinaryenToolError::RequestedExecutableMissing { path })
            if path == &root.join("missing-wasm-opt")
    ));
    assert_eq!(fs::read(&wasm_path).expect("read Wasm fixture"), original);
    fs::remove_dir_all(root).expect("remove temp root");
}

#[cfg(unix)]
#[test]
fn release_wasm_rejects_the_wrong_binaryen_version() {
    let root = unique_temp_dir("canic-wrong-wasm-opt");
    fs::create_dir_all(&root).expect("create temp dir");
    let wasm_path = root.join("test.wasm");
    let command_path = root.join("wasm-opt");
    fs::write(&wasm_path, minimal_wasm_with_contract()).expect("write Wasm fixture");
    write_executable(
        &command_path,
        "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then printf 'wasm-opt version 109\\n'; exit 0; fi\nexit 99\n",
    );

    optimize_release_wasm_artifact_with_command(
        &command_path.display().to_string(),
        CanisterBuildProfile::Release,
        &wasm_path,
    )
    .expect_err("wrong Binaryen must reject");

    assert_eq!(
        fs::read(&wasm_path).expect("read unchanged Wasm"),
        minimal_wasm_with_contract()
    );
    fs::remove_dir_all(root).expect("remove temp root");
}

#[cfg(unix)]
#[test]
fn release_wasm_optimization_records_metrics_and_preserves_contract() {
    let root = unique_temp_dir("canic-release-wasm-opt");
    fs::create_dir_all(&root).expect("create temp dir");
    let wasm_path = root.join("test.wasm");
    let command_path = root.join("wasm-opt");
    let original = minimal_wasm_with_contract();
    fs::write(&wasm_path, &original).expect("write Wasm fixture");
    write_executable(
        &command_path,
        "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then printf 'wasm-opt version 132 (version_132)\\n'; exit 0; fi\ncase \" $* \" in *\" --print-features \"*) printf '%s\\n' --enable-mutable-globals --enable-nontrapping-float-to-int --enable-bulk-memory --enable-sign-ext --enable-bulk-memory-opt; exit 0;; esac\n[ \"$2\" = \"-o\" ] || exit 91\n[ \"$4\" = \"-Oz\" ] || exit 92\ncase \" $* \" in *\" --enable-mutable-globals \"*) ;; *) exit 93;; esac\ncase \" $* \" in *\" --enable-bulk-memory-opt \"*) ;; *) exit 94;; esac\ncp \"$1\" \"$3\"\n",
    );

    let transform = optimize_release_wasm_artifact_with_command(
        &command_path.display().to_string(),
        CanisterBuildProfile::Release,
        &wasm_path,
    )
    .expect("release optimization");

    assert_eq!(fs::read(&wasm_path).expect("read optimized Wasm"), original);
    assert_eq!(transform.transform, ArtifactTransformKind::Optimize);
    assert_eq!(transform.outcome, ArtifactTransformOutcome::Applied);
    assert_eq!(
        transform.tool_version.as_deref(),
        Some("wasm-opt version 132 (version_132)")
    );
    assert!(
        transform
            .tool_sha256
            .as_deref()
            .is_some_and(|sha256| sha256.len() == 64)
    );
    let metrics = transform.metrics.expect("optimization metrics");
    assert_eq!(metrics.before, metrics.after);
    assert_eq!(metrics.before.defined_functions, 1);
    assert!(metrics.before.gzip_bytes > 0);
    fs::remove_dir_all(root).expect("remove temp root");
}

#[cfg(unix)]
#[test]
fn release_wasm_optimization_rejects_export_or_candid_drift() {
    let cases = [
        (
            "exports",
            minimal_wasm_with_contract_parts("other", b"service : {}"),
            "export inventory",
        ),
        (
            "candid",
            minimal_wasm_with_contract_parts("go", b"service : { changed : () -> (); }"),
            "public Candid",
        ),
    ];

    for (label, replacement, expected_error) in cases {
        let root = unique_temp_dir(&format!("canic-release-wasm-opt-{label}"));
        fs::create_dir_all(&root).expect("create temp dir");
        let wasm_path = root.join("test.wasm");
        let replacement_path = root.join("replacement.wasm");
        let command_path = root.join("wasm-opt");
        let original = minimal_wasm_with_contract();
        fs::write(&wasm_path, &original).expect("write Wasm fixture");
        fs::write(&replacement_path, replacement).expect("write replacement Wasm fixture");
        write_executable(
            &command_path,
            &format!(
                "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then printf 'wasm-opt version 132 (version_132)\\n'; exit 0; fi\ncase \" $* \" in *\" --print-features \"*) printf '%s\\n' --enable-sign-ext --enable-bulk-memory --enable-nontrapping-float-to-int; exit 0;; esac\ncp '{}' \"$3\"\n",
                replacement_path.display()
            ),
        );

        let error = optimize_release_wasm_artifact_with_command(
            &command_path.display().to_string(),
            CanisterBuildProfile::Release,
            &wasm_path,
        )
        .expect_err("contract drift must reject");

        assert!(error.to_string().contains(expected_error));
        assert_eq!(fs::read(&wasm_path).expect("read original Wasm"), original);
        assert!(!wasm_path.with_extension("wasm.optimized").exists());
        fs::remove_dir_all(root).expect("remove temp root");
    }
}

#[cfg(unix)]
#[test]
fn release_wasm_optimization_rejects_required_feature_drift() {
    let root = unique_temp_dir("canic-release-wasm-opt-features");
    fs::create_dir_all(&root).expect("create temp dir");
    let wasm_path = root.join("test.wasm");
    let command_path = root.join("wasm-opt");
    let original = minimal_wasm_with_contract();
    fs::write(&wasm_path, &original).expect("write Wasm fixture");
    write_executable(
        &command_path,
        "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then printf 'wasm-opt version 132 (version_132)\\n'; exit 0; fi\ncase \" $* \" in *\" --print-features \"*) case \"$1\" in *.optimized) printf '%s\\n' --enable-bulk-memory;; *) printf '%s\\n' --enable-sign-ext --enable-bulk-memory --enable-nontrapping-float-to-int;; esac; exit 0;; esac\ncp \"$1\" \"$3\"\n",
    );

    let error = optimize_release_wasm_artifact_with_command(
        &command_path.display().to_string(),
        CanisterBuildProfile::Release,
        &wasm_path,
    )
    .expect_err("required feature drift must reject");

    assert!(error.to_string().contains("changed required Wasm features"));
    assert_eq!(fs::read(&wasm_path).expect("read original Wasm"), original);
    assert!(!wasm_path.with_extension("wasm.optimized").exists());
    fs::remove_dir_all(root).expect("remove temp root");
}

fn minimal_wasm_with_contract() -> Vec<u8> {
    minimal_wasm_with_contract_parts("go", b"service : {}")
}

fn minimal_wasm_with_contract_parts(export: &str, candid: &[u8]) -> Vec<u8> {
    let mut wasm = minimal_wasm_with_exports(&[export]);
    let name = b"icp:public candid:service";
    let mut metadata = vec![u8::try_from(name.len()).expect("short custom-section name")];
    metadata.extend_from_slice(name);
    metadata.extend_from_slice(candid);
    push_section(&mut wasm, 0, &metadata);
    wasm
}

fn minimal_wasm_with_exports(exports: &[&str]) -> Vec<u8> {
    let mut wasm = b"\0asm\x01\0\0\0".to_vec();
    push_section(&mut wasm, 1, &[1, 0x60, 0, 0]);
    push_section(&mut wasm, 3, &[1, 0]);

    let mut export_section = vec![u8::try_from(exports.len()).expect("short export inventory")];
    for export in exports {
        export_section.push(u8::try_from(export.len()).expect("short export name"));
        export_section.extend_from_slice(export.as_bytes());
        export_section.extend_from_slice(&[0, 0]);
    }
    push_section(&mut wasm, 7, &export_section);
    push_section(&mut wasm, 10, &[1, 2, 0, 0x0b]);
    wasm
}

fn push_section(wasm: &mut Vec<u8>, id: u8, payload: &[u8]) {
    wasm.push(id);
    wasm.push(u8::try_from(payload.len()).expect("short test section"));
    wasm.extend_from_slice(payload);
}

fn wasm_with_code_section_size(size: usize) -> Vec<u8> {
    let mut wasm = b"\0asm\x01\0\0\0".to_vec();
    wasm.push(10);
    push_u32_leb128(
        &mut wasm,
        u32::try_from(size).expect("test section size fits u32"),
    );
    wasm.resize(wasm.len() + size, 0);
    wasm
}

fn push_u32_leb128(bytes: &mut Vec<u8>, mut value: u32) {
    loop {
        let mut byte = u8::try_from(value & 0x7f).expect("LEB128 byte fits u8");
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        bytes.push(byte);
        if value == 0 {
            return;
        }
    }
}

fn assert_no_artifact_stage(root: &Path) {
    let staged = fs::read_dir(root)
        .expect("read artifact root")
        .filter_map(Result::ok)
        .any(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with(".canic-artifact-stage-")
        });
    assert!(!staged, "artifact staging directory must be removed");
}

fn unique_temp_dir(label: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("{label}-{}-{nanos}", std::process::id()))
}

#[cfg(unix)]
fn write_executable(path: &Path, contents: &str) {
    let staged_path = path.with_extension("staged");
    fs::write(&staged_path, contents).expect("write staged fake executable");
    let mut permissions = fs::metadata(&staged_path)
        .expect("read fake executable metadata")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&staged_path, permissions).expect("make fake executable runnable");
    fs::rename(staged_path, path).expect("publish closed fake executable");
}
