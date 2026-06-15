use super::super::super::*;

pub(super) fn append_root_verification_check_items(
    lines: &mut Vec<String>,
    label: &str,
    items: &[DeploymentRootVerificationCheckV1],
) {
    if items.is_empty() {
        return;
    }
    lines.push(String::new());
    lines.push(format!("{label}:"));
    for item in items {
        lines.push(format!(
            "  - {} expected={} observed={} satisfied={}",
            item.name,
            item.expected.as_deref().unwrap_or("missing"),
            item.observed.as_deref().unwrap_or("missing"),
            item.satisfied
        ));
    }
}
