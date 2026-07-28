use crate::ids::CanisterRole;
use canic_core::{
    cdk::types::{Principal, TC},
    control_plane_support::{
        error::InternalError,
        model::replay::CommandKind,
        ops::{
            config::ConfigOps,
            cost_guard::{CostGuardPermit, CostGuardRequest},
            ic::{IcOps, mgmt::MgmtOps},
        },
        workflow::canister_lifecycle::{
            CanisterLifecycleEvent, CanisterLifecycleResult, CanisterLifecycleWorkflow,
        },
        workflow::cost_guard::{CostGuardWorkflow, map_cost_guard_reserve_error},
    },
    log,
    log::Topic,
    replay_policy::CostClass,
};

const CONTROL_PLANE_DEPLOYMENT_QUOTA_WINDOW_SECONDS: u64 = 60;
const MAX_CONTROL_PLANE_DEPLOYMENT_OPERATIONS_PER_WINDOW: u64 = 64;
const MIN_CONTROL_PLANE_CYCLES_AFTER_RESERVATION: u128 = TC;

pub const BOOTSTRAP_WASM_STORE_CREATE_COMMAND_KIND: &str =
    "management.control_plane.bootstrap_wasm_store_create.v1";
pub const PUBLICATION_WASM_STORE_CREATE_COMMAND_KIND: &str =
    "management.control_plane.publication_wasm_store_create.v1";
pub const COMPONENT_CREATE_COMMAND_KIND: &str = "management.control_plane.component_create.v1";
pub const COMPONENT_CHILD_CREATE_COMMAND_KIND: &str =
    "management.control_plane.component_child_create.v1";
pub const COMPONENT_CHILD_INSTALL_COMMAND_KIND: &str =
    "management.control_plane.component_child_install.v1";
pub const COMPONENT_INSTALL_COMMAND_KIND: &str = "management.control_plane.component_install.v1";

pub async fn create_canister_with_deployment_guard(
    command_kind: &'static str,
    role: CanisterRole,
    parent: Principal,
    extra_arg: Option<Vec<u8>>,
) -> Result<CanisterLifecycleResult, InternalError> {
    let quota_subject = IcOps::canister_self();
    let payer = IcOps::canister_self();
    let cycle_reservation_cycles = ConfigOps::try_get_canister_by_role(&role)?
        .initial_cycles
        .to_u128();
    let cost_permit = reserve_control_plane_deployment_cost_guard(
        command_kind,
        quota_subject,
        payer,
        cycle_reservation_cycles,
    )?;
    log!(
        Topic::CanisterLifecycle,
        Info,
        "control_plane_create: deployment cost guard reserved command_kind={} role={} parent={} quota_subject={} payer={}",
        command_kind,
        role,
        parent,
        quota_subject,
        payer
    );

    let result = CanisterLifecycleWorkflow::apply(CanisterLifecycleEvent::Create {
        deployment_permit: &cost_permit,
        role,
        parent,
        extra_arg,
    })
    .await;

    match result {
        Ok(result) => {
            CostGuardWorkflow::complete(&cost_permit, IcOps::now_secs())?;
            Ok(result)
        }
        Err(err) => Err(CostGuardWorkflow::recover_after_failure(
            &cost_permit,
            IcOps::now_secs(),
            err,
        )),
    }
}

pub fn reserve_component_creation_cost_guard(
    initial_cycles: &canic_core::cdk::types::Cycles,
) -> Result<CostGuardPermit, InternalError> {
    let root = IcOps::canister_self();
    reserve_control_plane_deployment_cost_guard(
        COMPONENT_CREATE_COMMAND_KIND,
        root,
        root,
        initial_cycles.to_u128(),
    )
}

pub fn reserve_component_child_creation_cost_guard(
    initial_cycles: &canic_core::cdk::types::Cycles,
) -> Result<CostGuardPermit, InternalError> {
    let root = IcOps::canister_self();
    reserve_control_plane_deployment_cost_guard(
        COMPONENT_CHILD_CREATE_COMMAND_KIND,
        root,
        root,
        initial_cycles.to_u128(),
    )
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
        current_cycle_balance: MgmtOps::canister_cycle_balance().to_u128(),
        cycle_reservation_cycles,
        min_cycles_after_reservation: MIN_CONTROL_PLANE_CYCLES_AFTER_RESERVATION,
    })
    .map_err(map_cost_guard_reserve_error)
}
