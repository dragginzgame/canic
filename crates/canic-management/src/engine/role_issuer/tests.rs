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
