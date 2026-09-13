//! Module: storage::stable::allocation_tests
//!
//! Responsibility: qualify stable allocation envelopes and storage-layout tradeoffs.
//! Does not own: deployment policy or protocol admission limits.
//! Boundary: uses isolated memories with the pinned stable-structures implementation.

use super::{
    fleet_admission::FleetAdmissionAuthorityRecord,
    template::chunked::{TemplateChunkPayloadRecord, TemplateChunkRefRecord},
};
use crate::ids::{TemplateChunkKey, TemplateId, TemplateReleaseKey, TemplateVersion};
use canic_core::{
    cdk::bounded_cell::BoundedCell,
    cdk::structures::{
        BTreeMap, Memory, Storable, Vec as StableVec, VectorMemory, storable::Bound,
    },
};
use std::{borrow::Cow, cell::Cell, rc::Rc};

#[derive(Clone, Default)]
struct ObservedMemory {
    memory: VectorMemory,
    reads: Rc<Cell<u64>>,
    writes: Rc<Cell<u64>>,
}

impl Memory for ObservedMemory {
    fn size(&self) -> u64 {
        self.memory.size()
    }
    fn grow(&self, pages: u64) -> i64 {
        self.memory.grow(pages)
    }
    fn read(&self, offset: u64, dst: &mut [u8]) {
        self.reads.set(self.reads.get() + 1);
        self.memory.read(offset, dst);
    }
    fn write(&self, offset: u64, bytes: &[u8]) {
        self.writes.set(self.writes.get() + 1);
        self.memory.write(offset, bytes);
    }
}

struct OverflowChunk(Vec<u8>);
impl Storable for OverflowChunk {
    const BOUND: Bound = Bound::Unbounded;
    fn to_bytes(&self) -> Cow<'_, [u8]> {
        Cow::Borrowed(&self.0)
    }
    fn into_bytes(self) -> Vec<u8> {
        self.0
    }
    fn from_bytes(bytes: Cow<'_, [u8]>) -> Self {
        Self(bytes.into_owned())
    }
}

#[test]
fn coordinator_admission_cell_has_one_page_when_empty() {
    let memory = VectorMemory::default();
    let cell = BoundedCell::init(memory.clone(), None::<FleetAdmissionAuthorityRecord>);
    assert!(cell.get().is_none());
    assert_eq!(memory.size(), 1);
}

#[test]
fn template_payload_layout_tradeoff_is_measured() {
    for payload_len in [4_096, 1_048_576] {
        let refs_memory = ObservedMemory::default();
        let payload_memory = ObservedMemory::default();
        let direct_memory = ObservedMemory::default();
        let mut refs = BTreeMap::init(refs_memory.clone());
        let payloads = StableVec::init(payload_memory.clone());
        let mut direct = BTreeMap::init(direct_memory.clone());
        let release = TemplateReleaseKey::new(
            TemplateId::new("measurement"),
            TemplateVersion::new("current"),
        );
        for index in 0..8 {
            let key = TemplateChunkKey::new(release.clone(), index);
            let bytes = vec![u8::try_from(index).unwrap(); payload_len];
            let slot = payloads.len();
            payloads.push(&TemplateChunkPayloadRecord {
                bytes: bytes.clone(),
            });
            refs.insert(
                key.clone(),
                TemplateChunkRefRecord {
                    slot,
                    payload_len: u32::try_from(payload_len).unwrap(),
                },
            );
            direct.insert(key, OverflowChunk(bytes));
        }
        let split_writes = refs_memory.writes.get() + payload_memory.writes.get();
        let direct_writes = direct_memory.writes.get();
        refs_memory.reads.set(0);
        payload_memory.reads.set(0);
        direct_memory.reads.set(0);
        for index in 0..8 {
            let key = TemplateChunkKey::new(release.clone(), index);
            let reference = refs.get(&key).unwrap();
            let split = payloads.get(reference.slot).unwrap();
            assert_eq!(split.bytes, direct.get(&key).unwrap().0);
        }
        let split_reads = refs_memory.reads.get() + payload_memory.reads.get();
        let direct_reads = direct_memory.reads.get();
        eprintln!(
            "template bytes={payload_len} split_pages={} direct_pages={} split_writes={split_writes} direct_writes={direct_writes} split_reads={split_reads} direct_reads={direct_reads}",
            refs_memory.size() + payload_memory.size(),
            direct_memory.size()
        );
        if payload_len == 1_048_576 {
            assert!(
                direct_reads > split_reads * 10,
                "re-evaluate payload consolidation if overflow reads become cheap"
            );
        }
    }
}
