use crate::ids::CanisterRole;
use canic_core::{
    control_plane_support::{
        cdk::types::{Principal, TC},
        error::{InternalError, InternalErrorOrigin},
        ops::{
            config::ConfigOps,
            cost_guard::{
                CostGuardOps, CostGuardPermit, CostGuardRequest, CostGuardReserveError,
                CostGuardReservePublicKind,
            },
            ic::{IcOps, mgmt::MgmtOps},
            replay::model::CommandKind,
        },
        replay_policy::CostClass,
        workflow::canister_lifecycle::{
            CanisterLifecycleEvent, CanisterLifecycleResult, CanisterLifecycleWorkflow,
        },
    },
    dto::error::Error,
    log,
    log::Topic,
};

const CONTROL_PLANE_DEPLOYMENT_QUOTA_WINDOW_SECONDS: u64 = 60;
const MAX_CONTROL_PLANE_DEPLOYMENT_OPERATIONS_PER_WINDOW: u64 = 64;
const MIN_CONTROL_PLANE_CYCLES_AFTER_RESERVATION: u128 = TC;

pub const BOOTSTRAP_AUTO_CREATE_COMMAND_KIND: &str =
    "management.control_plane.bootstrap_auto_create.v1";
pub const BOOTSTRAP_WASM_STORE_CREATE_COMMAND_KIND: &str =
    "management.control_plane.bootstrap_wasm_store_create.v1";
pub const PUBLICATION_WASM_STORE_CREATE_COMMAND_KIND: &str =
    "management.control_plane.publication_wasm_store_create.v1";

pub async fn create_canister_with_deployment_guard(
    command_kind: &'static str,
    role: CanisterRole,
    parent: Principal,
    extra_arg: Option<Vec<u8>>,
) -> Result<CanisterLifecycleResult, InternalError> {
    let quota_subject = IcOps::canister_self();
    let payer = IcOps::canister_self();
    let cost_permit =
        reserve_control_plane_deployment_cost_guard(command_kind, &role, quota_subject, payer)?;
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
            CostGuardOps::complete(&cost_permit, IcOps::now_secs())?;
            Ok(result)
        }
        Err(err) => {
            let _ = CostGuardOps::recover(&cost_permit, IcOps::now_secs());
            Err(err)
        }
    }
}

fn reserve_control_plane_deployment_cost_guard(
    command_kind: &'static str,
    role: &CanisterRole,
    quota_subject: Principal,
    payer: Principal,
) -> Result<CostGuardPermit, InternalError> {
    let cycle_reservation_cycles = ConfigOps::current_subnet()?
        .get_canister(role)
        .ok_or_else(|| {
            InternalError::workflow(
                InternalErrorOrigin::Config,
                format!("canister {role} not defined in current subnet"),
            )
        })?
        .initial_cycles
        .to_u128();

    CostGuardOps::reserve(CostGuardRequest {
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
    .map_err(map_control_plane_cost_guard_reserve_error)
}

fn map_control_plane_cost_guard_reserve_error(err: CostGuardReserveError) -> InternalError {
    match err.public_kind() {
        Some(CostGuardReservePublicKind::InvalidInput) => {
            InternalError::public(Error::invalid(err.to_string()))
        }
        Some(CostGuardReservePublicKind::ResourceExhausted) => {
            InternalError::public(Error::exhausted(err.to_string()))
        }
        None => err.into(),
    }
}
