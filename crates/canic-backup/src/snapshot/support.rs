use crate::discovery::SnapshotTarget;

pub(super) fn target_role(
    selected_canister: &str,
    index: usize,
    target: &SnapshotTarget,
) -> String {
    target.role.clone().unwrap_or_else(|| {
        if target.canister_id == selected_canister {
            "root".to_string()
        } else {
            format!("member-{index}")
        }
    })
}

pub(super) fn safe_path_segment(value: &str) -> String {
    value
        .chars()
        .map(|ch| match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => ch,
            _ => '_',
        })
        .collect()
}
