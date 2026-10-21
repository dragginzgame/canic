//! Module: model::application_checkpoint::tests
//!
//! Responsibility: qualify participant ordering, exact replay and durable source binding.
//! Boundary: these native cases establish no application freeze or IC snapshot effect.

use super::{
    ApplicationCheckpointError as Error, ApplicationCheckpointRecord as Record,
    ApplicationCheckpointTransition as Transition,
};
use canic_contracts::ids::{
    ApplicationCheckpointBinding, ApplicationCheckpointPhase as Phase, CallerInstallation,
    CallerReceiverAuthority, CallerRootAuthority,
};

pub fn binding(sequence: u64) -> ApplicationCheckpointBinding {
    let receiver = CallerInstallation {
        binding: crate::test::support::managed_component_binding(),
        install_id: [1; 32],
        component_install_id: [2; 32],
    };
    let component = receiver.component();
    let issuer = CallerRootAuthority {
        registry: component.authority.clone(),
        root: component.fleet_subnet_root,
        install_id: [3; 32],
    };
    ApplicationCheckpointBinding {
        authority: CallerReceiverAuthority {
            receiver,
            issuer,
            policy_digest: [4; 32],
        },
        operation_id: [u8::try_from(sequence).expect("fixture sequence fits u8"); 32],
        sequence,
        unit_digest: [5; 32],
    }
}

fn changed(result: Result<Transition, Error>) -> Record {
    match result {
        Ok(Transition::Changed(record)) => record,
        _ => panic!("expected a state transition"),
    }
}

fn retained(result: Result<Transition, Error>, expected: &Record) {
    match result {
        Ok(Transition::Retained(record)) => assert_eq!(&record, expected),
        _ => panic!("expected exact retained state"),
    }
}

fn prepared(binding: &ApplicationCheckpointBinding) -> Record {
    changed(Record::prepare(None, binding.clone()))
        .acknowledge_prepared(binding)
        .unwrap()
}

fn roundtrip(record: &Record) -> Result<Record, ciborium::de::Error<std::io::Error>> {
    let mut bytes = Vec::new();
    ciborium::ser::into_writer(record, &mut bytes).unwrap();
    ciborium::de::from_reader(bytes.as_slice())
}

#[test]
fn preparation_and_acknowledgement_replay_without_opening_mutation() {
    let binding = binding(1);
    let preparing = changed(Record::prepare(None, binding.clone()));
    assert_eq!(preparing.phase(), Phase::Preparing);
    assert!(!preparing.mutation_allowed());
    retained(
        Record::prepare(Some(&preparing), binding.clone()),
        &preparing,
    );
    let prepared = preparing.acknowledge_prepared(&binding).unwrap();
    assert!(!prepared.mutation_allowed());
    assert_eq!(prepared.acknowledge_prepared(&binding).unwrap(), prepared);
    retained(Record::prepare(Some(&prepared), binding), &prepared);
}

#[test]
fn release_before_prepare_fences_delayed_preparation() {
    let binding = binding(2);
    let releasing = changed(Record::release(None, binding.clone()));
    assert!(!releasing.mutation_allowed());
    assert!(matches!(
        Record::prepare(Some(&releasing), binding.clone()),
        Err(Error::InvalidPhase)
    ));
    let released = releasing.acknowledge_released(&binding).unwrap();
    assert!(released.mutation_allowed());
    assert!(matches!(
        Record::prepare(Some(&released), binding.clone()),
        Err(Error::Released)
    ));
    retained(Record::release(Some(&released), binding), &released);
}

#[test]
fn newer_checkpoint_requires_acknowledged_release_and_fences_old_messages() {
    let original = binding(1);
    let next = binding(2);
    let frozen = prepared(&original);
    assert!(matches!(
        Record::prepare(Some(&frozen), next.clone()),
        Err(Error::ActiveCheckpoint)
    ));
    assert!(matches!(
        Record::release(Some(&frozen), next.clone()),
        Err(Error::ActiveCheckpoint)
    ));
    let released = changed(Record::release(Some(&frozen), original.clone()))
        .acknowledge_released(&original)
        .unwrap();
    let newer = changed(Record::prepare(Some(&released), next));
    assert!(matches!(
        Record::release(Some(&newer), original.clone()),
        Err(Error::StaleSequence)
    ));
    assert!(matches!(
        Record::prepare(Some(&newer), original),
        Err(Error::StaleSequence)
    ));
}

#[test]
fn same_sequence_changed_identity_and_installation_refuse() {
    let original = binding(1);
    let frozen = prepared(&original);
    let mut changed_identity = original.clone();
    changed_identity.operation_id = [9; 32];
    assert!(matches!(
        Record::prepare(Some(&frozen), changed_identity),
        Err(Error::IdentityConflict)
    ));
    let mut changed_installation = original;
    changed_installation.authority.receiver.install_id = [9; 32];
    assert!(matches!(
        Record::release(Some(&frozen), changed_installation),
        Err(Error::IdentityConflict)
    ));
}

#[test]
fn acknowledgement_requires_exact_identity_and_phase() {
    let original = binding(1);
    let preparing = changed(Record::prepare(None, original.clone()));
    assert_eq!(
        preparing.acknowledge_released(&original),
        Err(Error::InvalidPhase)
    );
    let mut other = original;
    other.unit_digest = [9; 32];
    assert_eq!(
        preparing.acknowledge_prepared(&other),
        Err(Error::IdentityConflict)
    );
}

#[test]
fn restore_replays_before_and_after_prepared_acknowledgement_and_serialization() {
    let captured = binding(1);
    let destination = binding(2);
    let source = prepared(&captured);
    let restoring = changed(Record::restore(&source, &captured, destination.clone()));
    let restoring = roundtrip(&restoring).unwrap();
    retained(
        Record::restore(&restoring, &captured, destination.clone()),
        &restoring,
    );
    let restored = restoring.acknowledge_prepared(&destination).unwrap();
    let restored = roundtrip(&restored).unwrap();
    retained(
        Record::restore(&restored, &captured, destination.clone()),
        &restored,
    );
    assert!(matches!(
        Record::prepare(Some(&restored), destination),
        Err(Error::IdentityConflict)
    ));
}

#[test]
fn restored_release_preserves_source_and_never_refreezes_on_replay() {
    let captured = binding(1);
    let destination = binding(2);
    let restoring = changed(Record::restore(
        &prepared(&captured),
        &captured,
        destination.clone(),
    ));
    let releasing = changed(Record::release(Some(&restoring), destination.clone()));
    assert!(matches!(
        Record::restore(&releasing, &captured, destination.clone()),
        Err(Error::InvalidPhase)
    ));
    let released = releasing.acknowledge_released(&destination).unwrap();
    let released = roundtrip(&released).unwrap();
    retained(
        Record::release(Some(&released), destination.clone()),
        &released,
    );
    assert!(matches!(
        Record::restore(&released, &captured, destination),
        Err(Error::Released)
    ));
}

#[test]
fn restored_replay_refuses_changed_source_and_capture_purpose() {
    let captured = binding(1);
    let destination = binding(2);
    let restoring = changed(Record::restore(
        &prepared(&captured),
        &captured,
        destination.clone(),
    ));
    let mut other_source = captured;
    other_source.operation_id = [9; 32];
    assert!(matches!(
        Record::restore(&restoring, &other_source, destination.clone()),
        Err(Error::IdentityConflict)
    ));
    assert!(matches!(
        Record::restore(&prepared(&destination), &other_source, destination),
        Err(Error::IdentityConflict)
    ));
}

#[test]
fn restore_requires_prepared_original_source_new_sequence_and_same_unit() {
    let captured = binding(1);
    let source = changed(Record::prepare(None, captured.clone()));
    assert!(matches!(
        Record::restore(&source, &captured, binding(2)),
        Err(Error::InvalidSource)
    ));
    let source = prepared(&captured);
    assert!(matches!(
        Record::restore(&source, &captured, captured.clone()),
        Err(Error::InvalidSource)
    ));
    let mut destination = binding(2);
    destination.unit_digest = [9; 32];
    assert!(matches!(
        Record::restore(&source, &captured, destination),
        Err(Error::InvalidSource)
    ));
}

#[test]
fn invalid_identity_and_inconsistent_root_refuse_without_a_record() {
    let mut invalid = binding(1);
    invalid.sequence = 0;
    assert!(matches!(
        Record::prepare(None, invalid),
        Err(Error::InvalidIdentity)
    ));
    let mut invalid = binding(1);
    invalid.authority.receiver.component_install_id = [0; 32];
    assert!(matches!(
        Record::release(None, invalid),
        Err(Error::InvalidIdentity)
    ));
    let mut invalid = binding(1);
    invalid.authority.issuer.root = ic_platform::Principal::anonymous();
    assert!(matches!(
        Record::prepare(None, invalid),
        Err(Error::InvalidIdentity)
    ));
}

#[test]
fn durable_decode_validates_original_identity_and_restore_source_binding() {
    let captured = binding(1);
    let original = prepared(&captured);
    assert_eq!(roundtrip(&original).unwrap(), original);
    let mut invalid = original.clone();
    invalid.binding.sequence = 0;
    assert!(roundtrip(&invalid).is_err());
    let mut invalid = changed(Record::restore(&original, &captured, binding(2)));
    invalid.captured.as_mut().unwrap().unit_digest = [9; 32];
    assert!(roundtrip(&invalid).is_err());
}
