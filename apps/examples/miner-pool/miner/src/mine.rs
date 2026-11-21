// mine
// Attempt one toy block. Return the retained attempt number and whether it found a block.
// guard  : configured parent pool
// limits : default Wire limits, two batches per round
#[update(guard = require_parent)]
pub(super) fn mine(batch: (ic_wire::Bytes<32>, u64, u8)) -> (u64, bool) {
    use sha2::{Digest, Sha256};
    let (generation, round, ordinal) = batch;
    WORK_CELL.with_borrow_mut(|slot| {
        let cell = slot.as_mut().expect("miner open");
        let mut work = Work::decode(cell);
        assert_eq!(work.generation, generation, "stale worker allocation");
        assert!(round != 0 && round >= work.round, "stale round");
        assert!((1..=2).contains(&ordinal), "invalid batch");
        if round > work.round {
            work.round = round;
            work.batches = 0;
            work.finds = 0;
        }
        let bit = 1 << (ordinal - 1);
        if ordinal <= work.batches {
            return (work.attempts - u64::from(work.batches - ordinal), work.finds & bit != 0);
        }
        assert_eq!(ordinal, work.batches + 1, "missing earlier batch");
        work.batches = ordinal;
        let hash = Sha256::new()
            .chain_update(ic_platform_runtime::system::canister_self().as_slice())
            .chain_update(round.to_be_bytes())
            .chain_update([ordinal])
            .finalize();
        let found = hash[0] < 64;
        if found { work.finds |= bit; }
        work.attempts = work.attempts.checked_add(1).expect("attempt count exhausted");
        cell.set(work.encode());
        (work.attempts, found)
    })
}
