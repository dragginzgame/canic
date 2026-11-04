//! Typed operator Auth calls retain receiver refusals inside Root transport results.
use super::*;

pub(super) fn policy(
    icp: &IcpCli,
    root: Principal,
    target: Principal,
) -> Result<CanisterObservabilityResponse, CanisterProtocolError> {
    let reply = call_wire(
        icp,
        root,
        protocol::portable_policy::METHOD,
        protocol::portable_policy::METHOD_ID,
        &dto::root_observation::Request {
            canister_id: target.into(),
            input: (),
        },
        protocol::portable_policy::REQUEST_LIMITS,
        protocol::portable_policy::RESPONSE_LIMITS,
    )?;
    Ok(CanisterObservabilityResponse::PortablePolicy(reply))
}

pub(super) fn sessions(
    icp: &IcpCli,
    root: Principal,
    target: Principal,
    input: dto::portable_inspection::Request,
) -> Result<CanisterObservabilityResponse, CanisterProtocolError> {
    let reply = call_wire(
        icp,
        root,
        protocol::portable_sessions::METHOD,
        protocol::portable_sessions::METHOD_ID,
        &dto::root_observation::Request {
            canister_id: target.into(),
            input,
        },
        protocol::portable_sessions::REQUEST_LIMITS,
        protocol::portable_sessions::RESPONSE_LIMITS,
    )?;
    Ok(CanisterObservabilityResponse::PortableSessions(reply))
}
