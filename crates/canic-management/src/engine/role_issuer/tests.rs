use super::*;

#[test]
fn complete_issuer_record_fits_storage_budget() {
    let entries = ChildId::SHARDS.map(|id| {
        (
            Principal::from_slice(&[1; 29]),
            [u8::try_from(id.index()).unwrap(); 32],
            u64::MAX,
        )
    });
    let issuer = Issuer {
        seed: Some([255; 32]),
        closed: List::new(entries.to_vec()).unwrap(),
    };
    let bytes = issuer.bytes();
    let reopened: Issuer = ic_wire::decode(&bytes, Encoding::Payload, STORAGE_LIMITS).unwrap();
    assert_eq!(reopened.seed, issuer.seed);
    assert_eq!(reopened.closed.as_slice(), entries);
}

#[test]
fn renewal_requires_completed_creation_and_settled_upgrade() {
    use canic_management_protocol::{Failure, Phase, upgrade};
    let mut journal = crate::storage::Journal::default();
    assert!(!journal.ready_for_role());
    journal.0.phase = Phase::Ready;
    assert!(journal.ready_for_role());
    for phase in [
        upgrade::Phase::Scheduled,
        upgrade::Phase::InstallIssued,
        upgrade::Phase::Verified,
        upgrade::Phase::Expired,
    ] {
        journal.1.phase = phase;
        assert!(!journal.ready_for_role());
    }
    journal.1.phase = upgrade::Phase::Ready;
    assert!(journal.ready_for_role());
    journal.1.failure = Some(Failure::Uncertain);
    assert!(!journal.ready_for_role());
}
