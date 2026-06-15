use super::super::super::*;

pub(super) fn authority_controller_observation_from_action(
    action: &CanisterAuthorityActionV1,
) -> AuthorityControllerObservationV1 {
    AuthorityControllerObservationV1 {
        subject: authority_action_subject(action),
        canister_id: action.canister_id.clone(),
        role: action.role.clone(),
        state: action.state,
        action: action.action,
        observed_controllers: action.observed_controllers.clone(),
        desired_controllers: action.desired_controllers.clone(),
        controller_delta: action.controller_delta.clone(),
    }
}

fn authority_action_subject(action: &CanisterAuthorityActionV1) -> String {
    action
        .canister_id
        .clone()
        .or_else(|| action.role.as_ref().map(|role| format!("role:{role}")))
        .unwrap_or_else(|| "unknown".to_string())
}
