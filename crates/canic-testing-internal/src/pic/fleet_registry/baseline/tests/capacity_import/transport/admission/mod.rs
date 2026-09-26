//! Retain observed fixture authority for the production admission reader's HTTP journey.

use super::*;
use canic_host::fleet_ensure::{
    model::capacity_import::admission::{
        CapacityImportAdmissionRecord, CapacityImportInfrastructureKind,
        CapacityImportInfrastructureRecord,
    },
    ops::capacity_import::{
        admission::{
            CapacityImportDeclaration, CapacityImportDeclarations, CapacityImportDispositionKind,
        },
        with_admission,
    },
};

pub(super) fn review(
    pic: &PocketIc,
    initial: CapacityImportPlanRecord,
) -> CapacityImportPlanRecord {
    let root = initial.authority.root;
    let coordinator = initial.authority.coordinator;
    let operator = initial.authority.operator;
    let pool = root_pool_status_as(pic, root, operator);
    let store = pool
        .entries
        .iter()
        .find(|entry| entry.status == canic::dto::pool::CanisterPoolAssetStatus::Store)
        .unwrap()
        .canister_id;
    // Production admission requires real Principals for every infrastructure controller.
    pic.set_controllers(root, Some(operator), vec![operator])
        .unwrap();
    pic.set_controllers(coordinator, Some(operator), vec![operator])
        .unwrap();
    let registry: Result<CoordinatorRegistryResponse, Error> = pic
        .query_candid_as(
            coordinator,
            operator,
            canic::protocol::CANIC_COORDINATOR_REGISTRY,
            (CoordinatorRegistryRequest::Registry,),
        )
        .unwrap();
    let CoordinatorRegistryResponse::Registry(registry) = registry.unwrap();
    let infrastructure = [
        (
            coordinator,
            operator,
            CapacityImportInfrastructureKind::Coordinator,
        ),
        (root, operator, CapacityImportInfrastructureKind::Root),
        (
            store,
            root,
            CapacityImportInfrastructureKind::Store { root },
        ),
    ]
    .into_iter()
    .map(|(id, owner, kind)| {
        let status = pic.canister_status(id, Some(owner)).unwrap();
        let mut controllers = status.settings.controllers;
        controllers.sort_unstable();
        CapacityImportInfrastructureRecord {
            kind,
            principal: id,
            subnet: canic::ids::SubnetId::from_principal(pic.get_subnet(id).unwrap()),
            controllers,
            module_sha256: status.module_hash.unwrap().try_into().unwrap(),
        }
    })
    .collect();
    let source = &initial.sources[0];
    let declaration = CapacityImportDeclaration {
        canister: source.binding.canister_id.to_text(),
        subnet: source.binding.subnet.into_principal().to_text(),
        controllers: source
            .binding
            .controllers
            .iter()
            .map(Principal::to_text)
            .collect(),
        module_sha256: hex_bytes(source.binding.module_sha256.unwrap()),
        canister_version: source.binding.canister_version,
        disposition: CapacityImportDispositionKind::Absence,
        no_external_obligations: true,
        no_other_fleet_ownership: true,
        evidence:
            "Fixture created an isolated empty Wasm with no outgoing calls or outside accounts."
                .into(),
    };
    let declarations_toml = toml::to_string(&CapacityImportDeclarations {
        schema_version: 1,
        operator: operator.to_text(),
        network_root_key_sha256: hex_bytes(initial.authority.network_root_key_sha256),
        canisters: vec![declaration.clone()],
    })
    .unwrap();
    let declarations_sha256 = wasm_hash(declarations_toml.as_bytes()).try_into().unwrap();
    let mut sources = initial.sources;
    sources[0].disposition = declaration.disposition(declarations_sha256);
    let plan = prepare_review(initial.authority, sources, initial.root_budget).unwrap();
    with_admission(
        plan,
        CapacityImportAdmissionRecord {
            infrastructure,
            registry_candid_hex: hex_bytes(encode_one(&registry).unwrap()),
            declarations_toml,
            declarations_sha256,
        },
    )
    .unwrap()
}
