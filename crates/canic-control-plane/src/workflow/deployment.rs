use canic_core::{
    cdk::types::Principal,
    control_plane_support::{
        error::InternalError,
        model::replay::CommandKind,
        ops::{
            cost_guard::{CostGuardPermit, CostGuardRequest, CostGuardReserveError},
            ic::IcOps,
        },
        policy::deployment::MINIMUM_DEPLOYMENT_RESERVE_CYCLES,
        workflow::cost_guard::{CostGuardWorkflow, map_cost_guard_reserve_error},
    },
    replay_policy::CostClass,
};

const CONTROL_PLANE_DEPLOYMENT_QUOTA_WINDOW_SECONDS: u64 = 60;
const MAX_CONTROL_PLANE_DEPLOYMENT_OPERATIONS_PER_WINDOW: u64 = 64;

pub const COMPONENT_CREATE_COMMAND_KIND: &str = "management.control_plane.component_create.v1";
pub const COMPONENT_CHILD_CREATE_COMMAND_KIND: &str =
    "management.control_plane.component_child_create.v1";
pub const COMPONENT_CHILD_INSTALL_COMMAND_KIND: &str =
    "management.control_plane.component_child_install.v1";
pub const COMPONENT_INSTALL_COMMAND_KIND: &str = "management.control_plane.component_install.v1";
pub const CANISTER_POOL_CREATE_COMMAND_KIND: &str =
    "cycles_ledger.control_plane.canister_pool_create.v1";

pub fn reserve_canister_pool_creation_cost_guard() -> Result<CostGuardPermit, InternalError> {
    let root = IcOps::canister_self();
    reserve_control_plane_deployment_cost_guard(CANISTER_POOL_CREATE_COMMAND_KIND, root, root, 0)
}

pub fn reserve_component_pool_claim_guard() -> Result<CostGuardPermit, InternalError> {
    let root = IcOps::canister_self();
    reserve_control_plane_deployment_cost_guard(COMPONENT_CREATE_COMMAND_KIND, root, root, 0)
}

pub fn reserve_component_child_pool_claim_guard() -> Result<CostGuardPermit, InternalError> {
    let root = IcOps::canister_self();
    reserve_control_plane_deployment_cost_guard(COMPONENT_CHILD_CREATE_COMMAND_KIND, root, root, 0)
}

pub fn reserve_component_child_install_cost_guard() -> Result<CostGuardPermit, InternalError> {
    let root = IcOps::canister_self();
    reserve_control_plane_deployment_cost_guard(COMPONENT_CHILD_INSTALL_COMMAND_KIND, root, root, 0)
}

pub fn reserve_component_install_cost_guard() -> Result<CostGuardPermit, InternalError> {
    let root = IcOps::canister_self();
    reserve_control_plane_deployment_cost_guard(COMPONENT_INSTALL_COMMAND_KIND, root, root, 0)
}

fn reserve_control_plane_deployment_cost_guard(
    command_kind: &'static str,
    quota_subject: Principal,
    payer: Principal,
    cycle_reservation_cycles: u128,
) -> Result<CostGuardPermit, InternalError> {
    CostGuardWorkflow::reserve(CostGuardRequest {
        cost_class: CostClass::ManagementDeployment,
        command_kind: CommandKind::new(command_kind)
            .expect("control-plane deployment command kind is a valid static label"),
        quota_subject,
        payer,
        now_secs: IcOps::now_secs(),
        quota_window_secs: CONTROL_PLANE_DEPLOYMENT_QUOTA_WINDOW_SECONDS,
        max_operations_per_window: MAX_CONTROL_PLANE_DEPLOYMENT_OPERATIONS_PER_WINDOW,
        current_cycle_balance: IcOps::canister_cycle_balance().to_u128(),
        cycle_reservation_cycles,
        min_cycles_after_reservation: MINIMUM_DEPLOYMENT_RESERVE_CYCLES,
    })
    .map_err(|error| match error {
        CostGuardReserveError::CycleReserveRejected { .. } => {
            InternalError::public(canic_core::diagnostics::codes::DEPLOYMENT_CYCLE_RESERVE_REQUIRED)
        }
        other => map_cost_guard_reserve_error(other),
    })
}
