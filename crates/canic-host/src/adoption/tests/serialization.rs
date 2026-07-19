use super::*;

#[test]
fn adoption_report_round_trips_through_json() {
    let manifest = RoleArtifactManifestV1 {
        schema_version: 1,
        manifest_id: "manifest-1".to_string(),
        environment: "local".to_string(),
        artifact_root: None,
        role_artifacts: vec![RoleArtifactV1 {
            role: "api".to_string(),
            source: ArtifactSourceV1::LocalBuild,
            build_profile: "fast".to_string(),
            wasm_path: None,
            wasm_gz_path: None,
            wasm_gz_size_bytes: None,
            wasm_sha256: None,
            wasm_gz_sha256: None,
            wasm_gz_sha256_source: None,
            observed_wasm_gz_file_sha256: None,
            observed_wasm_gz_file_sha256_source: None,
            installed_module_hash: None,
            candid_path: None,
            candid_sha256: None,
            raw_config_sha256: None,
            canonical_embedded_config_sha256: None,
            embedded_topology_sha256: None,
            builder_version: None,
            rust_toolchain: None,
            package_version: None,
        }],
        unresolved_artifacts: Vec::new(),
    };
    let report = adoption_report_from_config_source(AdoptionReportRequest {
        report_id: "adoption-1",
        generated_at: "2026-05-30T00:00:00Z",
        profile: AdoptionProfileV1::Brownfield,
        config_source: CONFIG,
        inventory: None,
        artifact_manifest: Some(&manifest),
        package_metadata: Vec::new(),
    })
    .expect("adoption report");

    let encoded = serde_json::to_string(&report).expect("encode report");
    let decoded = serde_json::from_str::<AdoptionReportV1>(&encoded).expect("decode report");

    assert_eq!(decoded, report);
    assert_eq!(
        role(&decoded, "api").artifact_state,
        AdoptionArtifactStateV1::CanicBuilt
    );
}

#[test]
fn adoption_recommendation_rejects_unknown_json_fields() {
    let value = serde_json::json!({
        "kind": "review_authority_before_declaration",
        "severity": "Warning",
        "description": "review authority",
        "suggested_action": null,
        "suggested_action_effect": "ReadOnly",
        "suggested_action_support": "SupportedByAdoption",
        "operator_action_requirement": "Required",
        "unexpected": true,
    });

    assert!(serde_json::from_value::<AdoptionRecommendationV1>(value).is_err());
}
