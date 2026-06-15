use super::*;

pub(in crate::deployment_truth::tests) fn assert_plan_excludes_declared_only_store(
    plan: &DeploymentPlanV1,
) {
    assert!(
        plan.role_artifacts
            .iter()
            .all(|artifact| artifact.role != "store")
    );
    assert!(
        plan.unresolved_assumptions
            .iter()
            .all(|assumption| assumption.key != "local_artifacts.store")
    );
}

pub(in crate::deployment_truth::tests) fn assert_plan_has_implicit_wasm_store_artifact(
    plan: &DeploymentPlanV1,
) {
    assert!(
        plan.role_artifacts
            .iter()
            .any(|artifact| artifact.role == "wasm_store"
                && artifact.source == ArtifactSourceV1::WasmStore
                && artifact.observed_wasm_gz_file_sha256_source
                    == Some(ArtifactDigestSourceV1::ObservedFileDigest))
    );
}

pub(in crate::deployment_truth::tests) fn assert_plan_has_user_hub_release_artifact(
    plan: &DeploymentPlanV1,
) {
    assert!(
        plan.role_artifacts
            .iter()
            .any(|artifact| artifact.role == "user_hub"
                && artifact.wasm_gz_sha256.as_deref() == Some("user-hub-hash")
                && artifact.wasm_gz_sha256_source
                    == Some(ArtifactDigestSourceV1::ReleaseSetManifest)
                && artifact.observed_wasm_gz_file_sha256_source
                    == Some(ArtifactDigestSourceV1::ObservedFileDigest))
    );
}

pub(in crate::deployment_truth::tests) fn assert_json_round_trip<T>(value: &T)
where
    T: Clone + std::fmt::Debug + Eq + serde::de::DeserializeOwned + Serialize,
{
    let encoded = serde_json::to_string(value).expect("value should encode");
    let decoded = serde_json::from_str::<T>(&encoded).expect("value should decode");
    assert_eq!(decoded, *value);
}

pub(in crate::deployment_truth::tests) fn assert_object_keys(
    value: &serde_json::Value,
    expected: &[&str],
) {
    let object = value.as_object().expect("value should be a JSON object");
    let mut actual = object.keys().map(String::as_str).collect::<Vec<_>>();
    actual.sort_unstable();
    let mut expected = expected.to_vec();
    expected.sort_unstable();
    assert_eq!(actual, expected);
}
