use super::*;

#[test]
fn a_floor_requires_the_exact_approved_frame_and_a_positive_balance() {
    for floor in [NonZeroU64::MIN, NonZeroU64::MAX] {
        let bytes = reuse_policy(floor).unwrap();
        assert_eq!(minimum_reuse_cycles(&bytes), Some(floor));
        for n in 0..bytes.len() {
            assert_eq!(minimum_reuse_cycles(&bytes[..n]), None);
        }
    }
    let zero = ic_wire::encode(&0_u64, FRAME, LIMITS).unwrap();
    assert_eq!(minimum_reuse_cycles(&zero), None);
    assert_eq!(
        minimum_reuse_cycles(crate::topology::WORKER_GENERATION),
        None
    );
}
