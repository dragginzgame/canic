//! Module: model::application_checkpoint::root::tests
//!
//! Responsibility: qualify original-unit issuance, acknowledgement and durable recovery.
//! Boundary: native model tests do not establish stable-memory or IC-effect integration.

use super::{
    RootCheckpointError as Error, RootCheckpointPhase as Phase, RootCheckpointRecord as Record,
};
use canic_contracts::ids::{
    ApplicationCheckpointPhase, CallerReceiverAuthority, ManagedCanisterBinding,
};

fn authorities() -> Vec<CallerReceiverAuthority> {
    let first = crate::model::application_checkpoint::tests::binding(1).authority;
    let mut second = first.clone();
    let ManagedCanisterBinding::Component(component) = &mut second.receiver.binding else {
        panic!("component fixture required");
    };
    component.canister_id = ic_platform::Principal::from_slice(&[8]);
    vec![first, second]
}

fn begin(previous: Option<&Record>, expected: u64, operation: u8) -> Result<Record, Error> {
    let members = authorities();
    Record::begin(
        previous,
        &members[0].issuer,
        expected,
        [operation; 32],
        [7; 32],
        members.clone(),
    )
}

fn roundtrip(record: &Record) -> Record {
    let mut bytes = Vec::new();
    ciborium::ser::into_writer(record, &mut bytes).unwrap();
    ciborium::de::from_reader(bytes.as_slice()).unwrap()
}

fn acknowledge_all(mut record: Record, phase: ApplicationCheckpointPhase) -> Record {
    let bindings: Vec<_> = record
        .participants()
        .iter()
        .map(|participant| participant.binding().clone())
        .collect();
    for binding in bindings {
        record = record.acknowledge(&binding, phase).unwrap();
        assert_eq!(roundtrip(&record), record);
    }
    record
}

#[test]
fn original_unit_replays_after_lost_reply_and_reordered_discovery() {
    let original = begin(None, 0, 1).unwrap();
    let mut members = authorities();
    members.reverse();
    let replay = Record::begin(
        Some(&roundtrip(&original)),
        &members[0].issuer,
        0,
        [1; 32],
        [7; 32],
        members.clone(),
    )
    .unwrap();
    assert_eq!(original, replay);
    assert_eq!(replay.participants()[0].binding().sequence, 1);
    assert!(!replay.membership_mutation_allowed());
    assert_eq!(begin(Some(&original), 1, 2), Err(Error::ActiveCheckpoint));
}

#[test]
fn complete_unit_acknowledgement_and_release_survive_recovery() {
    let original = begin(None, 0, 1).unwrap();
    let binding = original.participants()[0].binding().clone();
    let partial = original
        .acknowledge(&binding, ApplicationCheckpointPhase::Prepared)
        .unwrap();
    assert_eq!(partial.phase(), Phase::Preparing);
    assert_eq!(
        partial
            .acknowledge(&binding, ApplicationCheckpointPhase::Prepared)
            .unwrap(),
        partial
    );
    let prepared = acknowledge_all(partial, ApplicationCheckpointPhase::Prepared);
    assert_eq!(prepared.phase(), Phase::Prepared);
    assert!(!prepared.membership_mutation_allowed());
    let releasing = roundtrip(&prepared.release());
    assert_eq!(releasing.phase(), Phase::Releasing);
    assert_eq!(
        releasing.acknowledge(&binding, ApplicationCheckpointPhase::Prepared),
        Err(Error::InvalidAcknowledgement)
    );
    let partial = releasing
        .acknowledge(&binding, ApplicationCheckpointPhase::Released)
        .unwrap();
    assert!(!partial.membership_mutation_allowed());
    let released = acknowledge_all(partial, ApplicationCheckpointPhase::Released);
    assert!(released.membership_mutation_allowed());
    assert_eq!(released.release(), released);
    assert_eq!(begin(Some(&released), 0, 1).unwrap(), released);
}

#[test]
fn cancelled_unanswered_prepares_still_require_every_release_reply() {
    let original = begin(None, 0, 1).unwrap();
    let releasing = original.release();
    assert!(!releasing.membership_mutation_allowed());
    let released = acknowledge_all(releasing, ApplicationCheckpointPhase::Released);
    let next = begin(Some(&released), 1, 2).unwrap();
    assert_eq!(next.participants()[0].binding().sequence, 2);
    assert_eq!(begin(Some(&next), 0, 1), Err(Error::InvalidSequence));
    assert_eq!(
        next.acknowledge(
            released.participants()[0].binding(),
            ApplicationCheckpointPhase::Prepared
        ),
        Err(Error::IdentityConflict)
    );
}

#[test]
fn changed_unit_authority_and_installation_refuse_without_replacing_original() {
    let original = begin(None, 0, 1).unwrap();
    let mut members = authorities();
    members[0].receiver.install_id = [9; 32];
    assert_eq!(
        Record::begin(
            Some(&original),
            &members[0].issuer,
            0,
            [1; 32],
            [7; 32],
            members.clone()
        ),
        Err(Error::IdentityConflict)
    );
    let root = authorities()[0].issuer.clone();
    members[0].issuer.install_id = [8; 32];
    assert_eq!(
        Record::begin(Some(&original), &root, 0, [1; 32], [7; 32], members),
        Err(Error::IdentityConflict)
    );
    assert_eq!(roundtrip(&original), original);
}

#[test]
fn invalid_catalogues_and_sequence_exhaustion_refuse() {
    let members = authorities();
    let root = members[0].issuer.clone();
    for invalid in [vec![], vec![members[0].clone(), members[0].clone()]] {
        assert_eq!(
            Record::begin(None, &root, 0, [1; 32], [7; 32], invalid),
            Err(Error::InvalidCatalogue)
        );
    }
    assert_eq!(begin(None, 1, 1), Err(Error::InvalidSequence));
    assert_eq!(begin(None, u64::MAX, 1), Err(Error::InvalidSequence));
    assert_eq!(begin(None, 0, 0), Err(Error::InvalidParticipant));
}

#[test]
fn decoding_cannot_claim_release_without_acknowledgements() {
    let record = begin(None, 0, 1).unwrap();
    let mut invalid = record.clone();
    invalid.phase = Phase::Released;
    let mut bytes = Vec::new();
    ciborium::ser::into_writer(&invalid, &mut bytes).unwrap();
    let decoded: Result<Record, _> = ciborium::de::from_reader(bytes.as_slice());
    assert!(matches!(decoded, Err(ciborium::de::Error::Semantic(..))));
    assert_eq!(roundtrip(&record), record);
}
