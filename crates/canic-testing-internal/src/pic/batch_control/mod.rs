//! Installed batch provisioning uses the actual native controller endpoint.
use canic::{
    Error,
    dto::{
        batch_control, component_provisioning::FleetComponentProvisioningPrepareRequest,
        role::OperationReceipt,
    },
    protocol::batch_control as wire,
};
use ic_platform::{pocket_ic::PocketIc, principal::ExternalPrincipal};

/// Retain the supplied plan and operation ID under the actual caller's authority.
pub(super) fn prepare(
    pic: &PocketIc,
    coordinator: ExternalPrincipal,
    caller: ExternalPrincipal,
    request: FleetComponentProvisioningPrepareRequest,
) -> Result<OperationReceipt, Error> {
    ic_testkit::pic::update_wire(
        pic,
        coordinator,
        caller,
        ic_testkit::pic::WireCallSpec {
            method: wire::prepare::METHOD,
            method_id: wire::prepare::METHOD_ID,
            request_limits: wire::PLAN_LIMITS,
            response_limits: wire::RESPONSE_LIMITS,
        },
        &batch_control::PrepareRequest::from(request),
    )
    .expect("native Component batch preparation transport")
}
