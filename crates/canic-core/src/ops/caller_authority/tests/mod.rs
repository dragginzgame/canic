//! Module: ops::caller_authority::tests
//!
//! Receiver publication, strict revocation and retained local admission evidence.

use super::*;
use crate::workflow::caller_authority::admission::CallerAdmissionWorkflow;
use crate::{
    config::caller_authority::{
        CallerAuthorityConfig, CallerPermission, CallerScope, CallerSourceSelector,
    },
    ids::{
        CallerComponentInstallation, CallerInstallation, CallerRootAuthority, CanisterRole,
        ComponentChildBinding, ManagedCanisterBinding,
    },
};
use std::collections::BTreeMap;

fn setup() -> (
    CallerReceiverAuthority,
    CompiledCallerPolicy,
    CallerInstallation,
) {
    CallerReceiverStore::reset();
    let receiver = CallerInstallation {
        binding: crate::test::support::managed_component_binding(),
        install_id: [1; 32],
        component_install_id: [1; 32],
    };
    let source = CallerInstallation {
        binding: ManagedCanisterBinding::ComponentChild(ComponentChildBinding {
            component: receiver.component().clone(),
            parent_canister_id: receiver.canister(),
            role: CanisterRole::from("shard"),
            canister_id: Principal::from_slice(&[22; 29]),
        }),
        install_id: [2; 32],
        component_install_id: [1; 32],
    };
    let policy = CompiledCallerPolicy::compile(
        receiver.role().clone(),
        Some(CallerAuthorityConfig {
            maximum_entries: 100,
            maximum_bytes: 1_000_000,
            permissions: BTreeMap::from([(
                "notify".into(),
                CallerPermission {
                    direction: crate::config::caller_authority::CallerPermissionDirection::Caller,
                    scope: CallerScope::SameComponent,
                    sources: vec![CallerSourceSelector {
                        component_spec: source.component().component_spec.clone(),
                        role: source.role().clone(),
                    }],
                },
            )]),
        }),
    )
    .unwrap();
    let authority = CallerReceiverAuthority {
        issuer: CallerRootAuthority {
            registry: receiver.component().authority.clone(),
            root: receiver.component().fleet_subnet_root,
            install_id: [3; 32],
        },
        receiver,
        policy_digest: policy.digest,
    };
    CallerAuthorityOps::initialize(authority.clone()).unwrap();
    (authority, policy, source)
}

fn publication(
    authority: &CallerReceiverAuthority,
    id: u8,
    change: CallerChangeRecord,
) -> CallerPublicationRecord {
    CallerAuthorityOps::publication(
        authority.clone(),
        [id; 32],
        CallerAuthorityOps::receiver().unwrap().generation,
        change,
    )
    .unwrap()
}

fn apply(publication: &CallerPublicationRecord, policy: &CompiledCallerPolicy) {
    crate::workflow::caller_authority::prepare(publication.clone(), policy).unwrap();
    CallerAuthorityOps::commit(publication).unwrap();
    CallerAuthorityOps::complete(publication).unwrap();
}

fn restore_serialized(authority: &CallerReceiverAuthority, policy: &CompiledCallerPolicy) {
    let bytes = crate::cdk::serialize::serialize(&CallerReceiverStore::export()).unwrap();
    CallerReceiverStore::reset();
    CallerReceiverStore::import(crate::cdk::serialize::deserialize(&bytes).unwrap());
    CallerAuthorityOps::restore(authority, policy).unwrap();
}

#[test]
fn publication_replays_and_denial_survive_lost_replies_and_restore() {
    let (authority, policy, source) = setup();
    let open = publication(&authority, 10, CallerChangeRecord::OpenReceiver);
    apply(&open, &policy);
    let grant = publication(&authority, 11, CallerChangeRecord::Grant(source.clone()));
    crate::workflow::caller_authority::prepare(grant.clone(), &policy).unwrap();
    assert_eq!(
        CallerAdmissionWorkflow::admit(source.canister(), "notify", &policy),
        Err(CallerAdmissionError::Fenced)
    );
    restore_serialized(&authority, &policy);
    CallerAuthorityOps::commit(&grant).unwrap();
    CallerAuthorityOps::commit(&grant).unwrap();
    CallerAuthorityOps::complete(&grant).unwrap();
    let ticket = CallerAdmissionWorkflow::admit(source.canister(), "notify", &policy).unwrap();
    CallerAdmissionWorkflow::revalidate(&ticket, &policy).unwrap();
    assert_eq!(
        CallerAdmissionWorkflow::admit(source.canister(), "different", &policy),
        Err(CallerAdmissionError::PermissionDenied)
    );
    assert_eq!(
        CallerAdmissionWorkflow::admit(authority.receiver.canister(), "notify", &policy),
        Err(CallerAdmissionError::PermissionDenied)
    );
    let deny = publication(
        &authority,
        12,
        CallerChangeRecord::DenySource(source.clone()),
    );
    crate::workflow::caller_authority::prepare(deny.clone(), &policy).unwrap();
    assert_eq!(
        CallerAdmissionWorkflow::revalidate(&ticket, &policy),
        Err(CallerAdmissionError::Fenced)
    );
    restore_serialized(&authority, &policy);
    let retained = CallerAuthorityOps::receiver().unwrap();
    assert_eq!(
        crate::workflow::caller_authority::prepare(grant, &policy)
            .unwrap()
            .phase,
        CallerReceiptPhase::Complete
    );
    assert_eq!(CallerAuthorityOps::receiver().unwrap(), retained);
    assert_eq!(
        CallerAdmissionWorkflow::admit(source.canister(), "notify", &policy),
        Err(CallerAdmissionError::Fenced)
    );
    CallerAuthorityOps::commit(&deny).unwrap();
    CallerAuthorityOps::complete(&deny).unwrap();
    let reopen = publication(&authority, 13, CallerChangeRecord::Grant(source));
    assert_eq!(
        crate::workflow::caller_authority::prepare(reopen, &policy),
        Err(CallerPublicationError::Retired)
    );
    CallerAuthorityOps::restore(&authority, &policy).unwrap();
}

#[test]
fn component_fence_covers_descendant_only_permissions_and_existing_tickets() {
    let (authority, policy, source) = setup();
    apply(
        &publication(&authority, 20, CallerChangeRecord::OpenReceiver),
        &policy,
    );
    apply(
        &publication(&authority, 21, CallerChangeRecord::Grant(source.clone())),
        &policy,
    );
    let ticket = CallerAdmissionWorkflow::admit(source.canister(), "notify", &policy).unwrap();
    let deny = publication(
        &authority,
        22,
        CallerChangeRecord::DenyComponent(CallerComponentInstallation {
            binding: source.component().clone(),
            install_id: source.component_install_id,
        }),
    );
    crate::workflow::caller_authority::prepare(deny.clone(), &policy).unwrap();
    assert_eq!(
        CallerAdmissionWorkflow::revalidate(&ticket, &policy),
        Err(CallerAdmissionError::Fenced)
    );
    CallerAuthorityOps::restore(&authority, &policy).unwrap();
    CallerAuthorityOps::commit(&deny).unwrap();
    CallerAuthorityOps::complete(&deny).unwrap();
    assert_eq!(
        crate::workflow::caller_authority::prepare(
            publication(&authority, 23, CallerChangeRecord::Grant(source)),
            &policy
        ),
        Err(CallerPublicationError::Retired)
    );
}

#[test]
fn a_recycled_principal_requires_fencing_the_previous_installation() {
    let (authority, policy, source) = setup();
    apply(
        &publication(&authority, 70, CallerChangeRecord::OpenReceiver),
        &policy,
    );
    apply(
        &publication(&authority, 71, CallerChangeRecord::Grant(source.clone())),
        &policy,
    );
    let old_ticket = CallerAdmissionWorkflow::admit(source.canister(), "notify", &policy).unwrap();
    let mut replacement = source.clone();
    replacement.install_id = [72; 32];
    assert_eq!(
        crate::workflow::caller_authority::prepare(
            publication(
                &authority,
                72,
                CallerChangeRecord::StageSource(replacement.clone())
            ),
            &policy
        ),
        Err(CallerPublicationError::AuthorityConflict)
    );
    apply(
        &publication(&authority, 73, CallerChangeRecord::DenySource(source)),
        &policy,
    );
    apply(
        &publication(
            &authority,
            74,
            CallerChangeRecord::StageSource(replacement.clone()),
        ),
        &policy,
    );
    assert_eq!(
        CallerAdmissionWorkflow::admit(replacement.canister(), "notify", &policy),
        Err(CallerAdmissionError::Fenced)
    );
    apply(
        &publication(
            &authority,
            75,
            CallerChangeRecord::Grant(replacement.clone()),
        ),
        &policy,
    );
    assert_eq!(
        CallerAdmissionWorkflow::revalidate(&old_ticket, &policy),
        Err(CallerAdmissionError::TicketExpired)
    );
    restore_serialized(&authority, &policy);
    CallerAdmissionWorkflow::admit(replacement.canister(), "notify", &policy).unwrap();
}

#[test]
fn changed_authority_or_census_identity_cannot_replace_a_retained_operation() {
    let (authority, policy, source) = setup();
    let grant = publication(&authority, 30, CallerChangeRecord::Grant(source));
    crate::workflow::caller_authority::prepare(grant.clone(), &policy).unwrap();
    let mut changed_authority = authority.clone();
    changed_authority.issuer.install_id = [99; 32];
    let altered = CallerAuthorityOps::publication(
        changed_authority.clone(),
        grant.operation_id,
        grant.previous_generation,
        grant.change.clone(),
    )
    .unwrap();
    assert_eq!(
        crate::workflow::caller_authority::prepare(altered, &policy),
        Err(CallerPublicationError::ReplayConflict)
    );
    assert_eq!(
        CallerAuthorityOps::restore(&changed_authority, &policy),
        Err(CallerPublicationError::AuthorityConflict)
    );
    let next = publication(&authority, 31, CallerChangeRecord::OpenReceiver);
    assert_eq!(
        crate::workflow::caller_authority::prepare(next, &policy),
        Err(CallerPublicationError::InProgress)
    );
    assert_eq!(
        CallerAuthorityOps::complete(&grant),
        Err(CallerPublicationError::PhaseConflict)
    );
}

#[test]
fn insufficient_capacity_does_not_write_a_denial_or_consume_generation() {
    let (mut authority, _, source) = setup();
    CallerReceiverStore::reset();
    let policy = CompiledCallerPolicy::compile(
        authority.receiver.role().clone(),
        Some(CallerAuthorityConfig {
            maximum_entries: 1,
            maximum_bytes: crate::config::caller_authority::CALLER_HEADER_BYTES * 2,
            permissions: BTreeMap::new(),
        }),
    )
    .unwrap();
    authority.policy_digest = policy.digest;
    CallerAuthorityOps::initialize(authority.clone()).unwrap();
    let before = CallerAuthorityOps::receiver().unwrap();
    let deny = publication(&authority, 40, CallerChangeRecord::DenySource(source));
    assert_eq!(
        crate::workflow::caller_authority::prepare(deny.clone(), &policy),
        Err(CallerPublicationError::Capacity)
    );
    assert_eq!(CallerAuthorityOps::receiver().unwrap(), before);
    assert!(CallerAuthorityOps::receipt(deny.operation_id).is_none());
    assert!(CallerRowStore::rows().is_empty());
}

#[test]
fn target_selection_does_not_authenticate_a_proxy_caller_or_expand_across_roots() {
    let (mut authority, policy, source) = setup();
    let mut configuration = policy.configuration.unwrap();
    let mut target = configuration.permissions.get("notify").unwrap().clone();
    target.direction = crate::config::caller_authority::CallerPermissionDirection::Target;
    configuration
        .permissions
        .insert("metrics_target".into(), target);
    let policy =
        CompiledCallerPolicy::compile(authority.receiver.role().clone(), Some(configuration))
            .unwrap();
    authority.policy_digest = policy.digest;
    CallerReceiverStore::reset();
    CallerAuthorityOps::initialize(authority.clone()).unwrap();
    apply(
        &publication(&authority, 80, CallerChangeRecord::Grant(source.clone())),
        &policy,
    );
    apply(
        &publication(&authority, 81, CallerChangeRecord::OpenReceiver),
        &policy,
    );
    let target =
        CallerAdmissionWorkflow::select_target(source.canister(), "metrics_target", &policy)
            .unwrap();
    assert_eq!(target.target(), source.canister());
    assert_eq!(
        CallerAdmissionWorkflow::admit(source.canister(), "metrics_target", &policy),
        Err(CallerAdmissionError::PermissionDenied)
    );
    assert_eq!(
        CallerAdmissionWorkflow::select_target(source.canister(), "notify", &policy),
        Err(CallerAdmissionError::PermissionDenied)
    );
    assert_eq!(
        CallerAdmissionWorkflow::select_target(Principal::anonymous(), "metrics_target", &policy),
        Err(CallerAdmissionError::PermissionDenied)
    );
    let mut foreign = source.clone();
    if let ManagedCanisterBinding::ComponentChild(child) = &mut foreign.binding {
        child.component.fleet_subnet_root = Principal::from_slice(&[89; 29]);
    }
    assert_eq!(
        crate::workflow::caller_authority::prepare(
            publication(&authority, 82, CallerChangeRecord::Grant(foreign)),
            &policy
        ),
        Err(CallerPublicationError::AuthorityConflict)
    );
    apply(
        &publication(&authority, 83, CallerChangeRecord::DenySource(source)),
        &policy,
    );
    assert_eq!(
        CallerAdmissionWorkflow::revalidate_target(&target, &policy),
        Err(CallerAdmissionError::TicketExpired)
    );
    restore_serialized(&authority, &policy);
}

#[test]
fn component_source_cleanup_is_bounded_and_retains_the_installation_fence() {
    let (mut authority, policy, source) = setup();
    let mut config = policy.configuration.unwrap();
    config.maximum_entries = 1000;
    config.maximum_bytes = 10_000_000;
    let policy =
        CompiledCallerPolicy::compile(authority.receiver.role().clone(), Some(config)).unwrap();
    authority.policy_digest = policy.digest;
    CallerReceiverStore::reset();
    CallerAuthorityOps::initialize(authority.clone()).unwrap();
    apply(
        &publication(&authority, 90, CallerChangeRecord::OpenReceiver),
        &policy,
    );
    let mut original = None;
    for index in 0_u32..150 {
        let mut child = source.clone();
        if let ManagedCanisterBinding::ComponentChild(binding) = &mut child.binding {
            binding.canister_id = Principal::from_slice(&index.to_be_bytes());
        }
        let mut operation = [91; 32];
        operation[..4].copy_from_slice(&index.to_be_bytes());
        let grant = CallerAuthorityOps::publication(
            authority.clone(),
            operation,
            CallerAuthorityOps::receiver().unwrap().generation,
            CallerChangeRecord::Grant(child),
        )
        .unwrap();
        apply(&grant, &policy);
        original = Some(grant);
    }
    let deny = publication(
        &authority,
        92,
        CallerChangeRecord::DenyComponent(CallerComponentInstallation {
            binding: source.component().clone(),
            install_id: source.component_install_id,
        }),
    );
    crate::workflow::caller_authority::prepare(deny.clone(), &policy).unwrap();
    CallerAuthorityOps::commit(&deny).unwrap();
    let before = CallerAuthorityOps::receiver().unwrap().entries;
    assert_eq!(
        CallerAuthorityOps::complete(&deny).unwrap().phase,
        CallerReceiptPhase::Committed
    );
    assert_eq!(
        CallerAuthorityOps::receiver().unwrap().entries,
        before - 128
    );
    restore_serialized(&authority, &policy);
    assert_eq!(
        CallerAuthorityOps::complete(&deny).unwrap().phase,
        CallerReceiptPhase::Complete
    );
    assert_eq!(CallerRowStore::source_page(None, 1).len(), 0);
    let original = original.unwrap();
    crate::workflow::caller_authority::prepare(original.clone(), &policy).unwrap();
    CallerAuthorityOps::commit(&original).unwrap();
    CallerAuthorityOps::complete(&original).unwrap();
    assert_eq!(CallerRowStore::source_page(None, 1).len(), 0);
    assert_eq!(
        crate::workflow::caller_authority::prepare(
            publication(&authority, 93, original.change),
            &policy
        ),
        Err(CallerPublicationError::Retired)
    );
    restore_serialized(&authority, &policy);
}

#[test]
fn component_fence_key_covers_maximum_encoded_identifiers() {
    use crate::cdk::structures::Storable;
    let key = CallerRowKey::component(
        ComponentInstanceId::from_generated_bytes([255; 32]),
        [255; 32],
    );
    let bytes = key.to_bytes_checked();
    assert_eq!(CallerRowKey::from_bytes(bytes), key);
}

#[test]
fn cold_restore_rejects_source_installation_and_generation_corruption() {
    let (authority, policy, source) = setup();
    apply(
        &publication(&authority, 17, CallerChangeRecord::Grant(source.clone())),
        &policy,
    );
    let key = CallerRowKey::source(source.canister());
    let Some(CallerRowRecord::Source(original)) = CallerRowStore::get(&key) else {
        unreachable!()
    };
    let mut corrupt = original.clone();
    corrupt.generation = u64::MAX;
    CallerRowStore::insert(key.clone(), CallerRowRecord::Source(corrupt));
    assert_eq!(
        CallerAuthorityOps::restore(&authority, &policy),
        Err(CallerPublicationError::GenerationConflict)
    );
    let mut corrupt = original;
    corrupt.installation.component_install_id = [0; 32];
    CallerRowStore::insert(key, CallerRowRecord::Source(corrupt));
    assert_eq!(
        CallerAuthorityOps::restore(&authority, &policy),
        Err(CallerPublicationError::AuthorityConflict)
    );
}

#[test]
fn undeclared_source_cannot_reserve_publication_authority() {
    let (authority, policy, mut source) = setup();
    let ManagedCanisterBinding::ComponentChild(child) = &mut source.binding else {
        panic!("child fixture");
    };
    child.role = CanisterRole::from("unselected");
    let before = CallerAuthorityOps::receiver().unwrap();
    for change in [
        CallerChangeRecord::StageSource(source.clone()),
        CallerChangeRecord::Grant(source),
    ] {
        let publication = publication(&authority, 101, change);
        let operation = publication.operation_id;
        assert_eq!(
            crate::workflow::caller_authority::prepare(publication, &policy),
            Err(CallerPublicationError::AuthorityConflict)
        );
        assert_eq!(CallerAuthorityOps::receiver().unwrap(), before);
        assert!(CallerAuthorityOps::receipt(operation).is_none());
        assert!(CallerRowStore::rows().is_empty());
    }
}
