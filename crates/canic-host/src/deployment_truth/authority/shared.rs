use super::super::*;
use std::collections::BTreeSet;

pub(in crate::deployment_truth) const AUTHORITY_UNSAFE_BLOCKED_CODE: &str =
    "authority_unsafe_blocked";
pub(super) fn difference(left: &[String], right: &[String]) -> Vec<String> {
    left.iter()
        .filter(|value| !right.iter().any(|candidate| candidate == *value))
        .cloned()
        .collect()
}

pub(super) fn controller_delta(missing: &[String], extra: &[String]) -> AuthorityControllerDeltaV1 {
    AuthorityControllerDeltaV1 {
        add_controllers: missing.to_vec(),
        remove_controllers: extra.to_vec(),
    }
}

pub(super) fn sorted_unique(values: Vec<String>) -> Vec<String> {
    values
        .into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

pub(super) fn controller_delta_reason(missing: &[String], extra: &[String]) -> String {
    match (missing.is_empty(), extra.is_empty()) {
        (false, true) => format!("missing desired controllers: {}", missing.join(",")),
        (true, false) => format!("extra observed controllers: {}", extra.join(",")),
        (false, false) => format!(
            "controller set differs: missing {}; extra {}",
            missing.join(","),
            extra.join(",")
        ),
        (true, true) => "observed controller set already matches desired authority".to_string(),
    }
}

pub(super) fn action_subject(action: &CanisterAuthorityActionV1) -> Option<String> {
    action
        .canister_id
        .clone()
        .or_else(|| action.role.as_ref().map(|role| format!("role:{role}")))
}
