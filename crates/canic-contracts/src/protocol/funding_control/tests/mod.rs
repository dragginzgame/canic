//! The actual funding graph, principal boundaries and original deadlines remain representable.
use crate::{dto::funding_control as dto, protocol::funding_control as wire};

#[cfg(feature = "wire-schema")]
#[test]
fn command_families_have_native_metadata() {
    use crate::dto::error::Error;
    use crate::dto::*;
    macro_rules! schema { ($($ty:ty),* $(,)?) => { $(ic_wire::typescript::native_schema::<$ty>().unwrap();)* }; }
    schema!(dto::Request, dto::AcceptanceRequest, dto::BeginRequest, dto::StageRequest, dto::PrepareRequest, dto::ActivateRequest,
        Result<dto::Response, Error>, Result<dto::AcceptanceReceipt, Error>, Result<dto::RotationReceipt, Error>,
        fleet_funding::FleetFundingPolicyRotationApplyRequest, state::SetCyclesFundingRequest, Result<state::SetStateResponse<bool>, Error>,
        coordinator_acknowledgements::AcknowledgeRequest, Result<coordinator_acknowledgements::RootAcknowledgement, Error>,
        icp_refill::CycleRefillInput, Result<icp_refill::IcpRefillDryRun, Error>, Result<role::OperationReceipt, Error>,
        component_registry::RootComponentAllocationRequest, component_control::PreparationRequest, component_control::RegistryStatus, component_control::SubtreeRequest, component_control::PeerRequest, component_registry::RootComponentChildAllocationRequest, component_registry::RootComponentDrainingRequest,
        root_deletion::PrepareRequest, root_deletion::CompleteRequest, Result<root_deletion::Execution, Error>, Result<root_deletion::Receipt, Error>,
        pool_control::HandoffRequest, Result<pool_control::HandoffResponse, Error>,
        root_store_control::AdoptRequest, root_store::RootStoreBootstrapRequest, root_store::RootStoreFixturePrepareRequest, Result<Result<fixture_provisioning::FixtureSourceStatus, fixture_provisioning::FixtureStoreError>, Error>,
        registry_control::SynchronizeRequest, pool_control::MaintenanceResponse, registry_control::JoinRequest, registry_control::ActivateRequest, Result<registry_control::JoinResponse, Error>, Result<registry_control::ActivateResponse, Error>);
}

#[test]
fn rotation_receipt_keeps_principal_generations_and_original_time() {
    let stored = crate::dto::fleet_funding::FleetFundingPolicyRotationRootReceipt {
        operation_id: [1; 32],
        plan_digest: [2; 32],
        fleet_subnet_root: ic_platform::principal::ExternalPrincipal::from_slice(&[3; 29]),
        predecessor_generation: u64::MAX - 1,
        successor_generation: u64::MAX,
        prepared: true,
        activated: false,
        recorded_at_ns: u64::MAX,
    };
    let native = dto::RotationReceipt::from(stored.clone());
    let bytes = ic_wire::native::encode_response(
        wire::root_prepare::METHOD_ID,
        &native,
        wire::RESPONSE_LIMITS,
    )
    .unwrap();
    let decoded = ic_wire::native::decode_response::<dto::RotationReceipt>(
        wire::root_prepare::METHOD_ID,
        &bytes,
        wire::RESPONSE_LIMITS,
    )
    .unwrap();
    assert_eq!(
        crate::dto::fleet_funding::FleetFundingPolicyRotationRootReceipt::from(decoded),
        stored
    );
    assert!(matches!(
        ic_wire::native::decode_response::<dto::RotationReceipt>(
            wire::root_activate::METHOD_ID,
            &bytes,
            wire::RESPONSE_LIMITS
        ),
        Err(ic_wire::Error::Identity)
    ));
}
