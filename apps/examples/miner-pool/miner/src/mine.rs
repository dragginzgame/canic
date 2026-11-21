// mine
// Produce one toy block and return its cumulative number. The pool serializes rounds per miner.
// guard  : configured parent pool
// limits : default Wire limits, two batches per round
#[update(guard = require_parent)]
pub(super) fn mine(batch: (u64, u8)) -> u64 {
    let (round, ordinal) = batch;
    WORK_CELL.with_borrow_mut(|slot| {
        let cell = slot.as_mut().expect("miner open");
        let mut work = Work::decode(cell);
        assert!(round != 0 && round >= work.round, "stale round");
        assert!((1..=2).contains(&ordinal), "invalid batch");
        if round > work.round {
            work.round = round;
            work.batches = 0;
        }
        if ordinal <= work.batches {
            return work.blocks - u64::from(work.batches - ordinal);
        }
        assert_eq!(ordinal, work.batches + 1, "missing earlier batch");
        work.batches = ordinal;
        work.blocks = work.blocks.checked_add(1).expect("block count exhausted");
        cell.set(work.encode());
        work.blocks
    })
}
