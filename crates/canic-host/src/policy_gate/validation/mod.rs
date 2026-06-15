use super::{CiPolicyV1, PolicyGateError, ProjectEvidenceManifestV1};
use std::{
    collections::BTreeSet,
    path::{Component, Path},
};

pub(super) fn validate_ci_policy_v1(policy: &CiPolicyV1) -> Result<(), PolicyGateError> {
    if policy.schema_version != 1 {
        return Err(PolicyGateError::InvalidPolicy(format!(
            "unsupported schema_version {}; expected 1",
            policy.schema_version
        )));
    }
    ensure_nonempty("envelope.required_schema", &policy.envelope.required_schema)?;
    ensure_optional_allow_list(
        "envelope.allowed_payload_schemas",
        policy.envelope.allowed_payload_schemas.as_deref(),
    )?;
    ensure_optional_allow_list(
        "envelope.allowed_payload_stability",
        policy.envelope.allowed_payload_stability.as_deref(),
    )?;
    if policy.exit_class.allowed.is_empty() {
        return Err(PolicyGateError::InvalidPolicy(
            "exit_class.allowed must not be empty".to_string(),
        ));
    }
    if policy
        .build_provenance
        .as_ref()
        .is_some_and(|rules| rules.rules.is_empty())
    {
        return Err(PolicyGateError::InvalidPolicy(
            "build_provenance must enable at least one rule".to_string(),
        ));
    }
    for (index, rule) in policy.required_input.iter().enumerate() {
        ensure_nonempty(&format!("required_input[{index}].kind"), &rule.kind)?;
        if let Some(schema) = &rule.schema {
            ensure_nonempty(&format!("required_input[{index}].schema"), schema)?;
        }
    }
    Ok(())
}

pub(super) fn validate_project_evidence_manifest_v1(
    manifest: &ProjectEvidenceManifestV1,
) -> Result<(), PolicyGateError> {
    if manifest.schema_version != 1 {
        return Err(PolicyGateError::InvalidPolicy(format!(
            "unsupported project evidence manifest schema_version {}; expected 1",
            manifest.schema_version
        )));
    }
    ensure_nonempty("project.name", &manifest.project.name)?;
    ensure_nonempty("project.root", &manifest.project.root)?;
    if manifest.evidence.is_empty() {
        return Err(PolicyGateError::InvalidPolicy(
            "evidence must not be empty".to_string(),
        ));
    }
    let mut seen_paths = BTreeSet::new();
    for (index, entry) in manifest.evidence.iter().enumerate() {
        ensure_nonempty(&format!("evidence[{index}].kind"), &entry.kind)?;
        ensure_nonempty(&format!("evidence[{index}].path"), &entry.path)?;
        let path_key = manifest_evidence_path_key(&entry.path);
        if !seen_paths.insert(path_key.clone()) {
            return Err(PolicyGateError::InvalidPolicy(format!(
                "evidence[{index}].path duplicates an earlier evidence path after normalization: {path_key}"
            )));
        }
        ensure_nonempty(
            &format!("evidence[{index}].payload_schema"),
            &entry.payload_schema,
        )?;
        if !entry.target.has_selector() {
            return Err(PolicyGateError::InvalidPolicy(format!(
                "evidence[{index}].target must include at least one target field"
            )));
        }
    }
    Ok(())
}

fn manifest_evidence_path_key(path: &str) -> String {
    let mut components = Vec::new();

    for component in Path::new(path.trim()).components() {
        match component {
            Component::Prefix(prefix) => {
                components.push(prefix.as_os_str().to_string_lossy().to_string());
            }
            Component::RootDir => components.push(String::new()),
            Component::CurDir => {}
            Component::ParentDir => {
                if components
                    .last()
                    .is_some_and(|component| !component.is_empty() && component != "..")
                {
                    components.pop();
                } else {
                    components.push("..".to_string());
                }
            }
            Component::Normal(segment) => {
                components.push(segment.to_string_lossy().to_string());
            }
        }
    }

    if components.is_empty() {
        ".".to_string()
    } else {
        components.join("/")
    }
}

fn ensure_optional_allow_list<T>(field: &str, value: Option<&[T]>) -> Result<(), PolicyGateError> {
    if value.is_some_and(<[T]>::is_empty) {
        return Err(PolicyGateError::InvalidPolicy(format!(
            "{field} must not be empty when present"
        )));
    }
    Ok(())
}

fn ensure_nonempty(field: &str, value: &str) -> Result<(), PolicyGateError> {
    if value.trim().is_empty() {
        return Err(PolicyGateError::InvalidPolicy(format!(
            "{field} must not be empty"
        )));
    }
    Ok(())
}
