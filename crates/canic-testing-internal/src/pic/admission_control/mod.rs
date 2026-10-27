//! Installed admission queries exercise the exact native production contract.
use canic::{
    Error,
    dto::{
        admission_control::RootStatus, fleet_admission::FleetAdmissionRootStatusResponse,
        page::PageRequest,
    },
    protocol::admission_control,
};
use ic_platform::{pocket_ic::PocketIc, principal::ExternalPrincipal};

pub fn root_status(
    pic: &PocketIc,
    root: ExternalPrincipal,
    caller: ExternalPrincipal,
    page: PageRequest,
) -> Result<FleetAdmissionRootStatusResponse, Error> {
    let reply: Result<RootStatus, Error> = ic_testkit::pic::query_wire(
        pic,
        root,
        caller,
        ic_testkit::pic::WireCallSpec {
            method: admission_control::root_status::METHOD,
            method_id: admission_control::root_status::METHOD_ID,
            request_limits: admission_control::REQUEST_LIMITS,
            response_limits: admission_control::RESPONSE_LIMITS,
        },
        &page,
    )
    .expect("native Root admission status transport");
    reply.map(Into::into)
}

/// Observe the exact admission operation as its authorized caller.
pub fn operation_status(
    pic: &PocketIc,
    coordinator: ExternalPrincipal,
    caller: ExternalPrincipal,
    operation_id: [u8; 32],
) -> Result<canic::dto::fleet_admission::FleetAdmissionOperationStatusResponse, Error> {
    let reply: Result<canic::dto::admission_control::OperationStatus, Error> =
        ic_testkit::pic::query_wire(
            pic,
            coordinator,
            caller,
            ic_testkit::pic::WireCallSpec {
                method: admission_control::operation::METHOD,
                method_id: admission_control::operation::METHOD_ID,
                request_limits: admission_control::REQUEST_LIMITS,
                response_limits: admission_control::RESPONSE_LIMITS,
            },
            &operation_id,
        )
        .expect("native admission operation transport");
    reply.map(Into::into)
}
