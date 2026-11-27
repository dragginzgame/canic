//! Stable placement records. Each write names the grant, allocation or receipt it changes.
use super::{Pools, payment, singleton, workers};
use canic_management_protocol::children::ChildId;
use ic_storage::{DefaultMemoryImpl, MemoryRuntime, adapter::StableBTreeMap};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

const LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 65_536,
    allocation: 1_048_576,
    nodes: 65_536,
    depth: 24,
};

#[derive(Serialize, Deserialize)]
pub(super) enum Key {
    Capacity,
    Permanent(ChildId),
    Funding(ChildId),
    FundingReceipts,
    Quote(payment::Owner),
    Payment([u8; 32]),
    Drain(ChildId),
    Retirement(ChildId),
    StandbyPolicy,
    Standby([u8; 4]),
    Return([u8; 4]),
    Shard(String),
    Instances,
    InstanceIncreases,
    Project([u8; 4]),
    Singleton,
    SingletonChild([u8; 32]),
    Worker(String),
    WorkerClaim(String, [u8; 4]),
}

fn encode(value: &impl Serialize) -> Vec<u8> {
    ic_wire::encode(value, ic_wire::Encoding::Payload, LIMITS).expect("bounded placement row")
}
fn decode<T: DeserializeOwned>(bytes: &[u8]) -> T {
    ic_wire::decode(bytes, ic_wire::Encoding::Payload, LIMITS).expect("retained placement row")
}
pub(super) fn ordinal(index: usize) -> [u8; 4] {
    u32::try_from(index)
        .expect("placement ordinal")
        .to_be_bytes()
}
fn push<T>(rows: &mut Vec<T>, key: [u8; 4], bytes: &[u8])
where
    T: DeserializeOwned,
{
    assert_eq!(ordinal(rows.len()), key, "missing placement ordinal");
    rows.push(decode(bytes));
}

pub(in crate::engine) struct PlacementRows(StableBTreeMap<Vec<u8>, Vec<u8>>);
impl PlacementRows {
    pub(super) fn put(&mut self, key: &Key, value: &impl Serialize) {
        self.0.insert(encode(key), encode(value));
    }
    pub(super) fn remove(&mut self, key: &Key) {
        self.0.remove(&encode(key)).expect("retained placement row");
    }
    pub(in crate::engine) fn open(
        runtime: &MemoryRuntime<DefaultMemoryImpl>,
        fresh: bool,
    ) -> (Self, Pools) {
        let rows = Self(canic::storage::map(
            runtime,
            crate::storage::SHARDS_KEY,
            fresh,
        ));
        let mut pools = Pools::default();
        for entry in rows.0.iter() {
            let bytes = entry.value();
            let b = bytes.as_slice();
            match decode(entry.key()) {
                Key::Capacity => pools.capacity = Some(decode(b)),
                Key::Permanent(id) => {
                    let () = decode(b);
                    pools.permanent.insert(id);
                }
                Key::Funding(id) => {
                    pools.funding.insert(id, decode(b));
                }
                Key::FundingReceipts => pools.receipts = decode(b),
                Key::Quote(owner) => {
                    pools.quotes.insert(owner, decode(b));
                }
                Key::Payment(id) => {
                    pools.payments.insert(id, decode(b));
                }
                Key::Drain(id) => {
                    pools.evacuated.insert(id);
                    pools.draining.insert(id, decode(b));
                }
                Key::Retirement(id) => {
                    pools.retirements.insert(id, decode(b));
                }
                Key::StandbyPolicy => pools.reserve_policy = Some(decode(b)),
                Key::Standby(i) => push(&mut pools.standby, i, b),
                Key::Return(i) => push(&mut pools.returns, i, b),
                Key::Shard(name) => {
                    pools.shards.insert(name, decode(b));
                }
                Key::Instances => pools.instances = Some(decode(b)),
                Key::InstanceIncreases => pools.instance_increases = decode(b),
                Key::Project(i) => push(&mut pools.projects, i, b),
                Key::Singleton => {
                    pools.singleton = Some(singleton::Registry {
                        grant: decode(b),
                        children: Default::default(),
                    })
                }
                Key::SingletonChild(generation) => {
                    pools
                        .singleton
                        .as_mut()
                        .expect("singleton grant")
                        .children
                        .insert(generation, decode(b));
                }
                Key::Worker(name) => {
                    pools.workers.insert(
                        name,
                        workers::Workers {
                            grant: decode(b),
                            claims: Vec::new(),
                        },
                    );
                }
                Key::WorkerClaim(name, i) => push(
                    &mut pools.workers.get_mut(&name).expect("worker grant").claims,
                    i,
                    b,
                ),
            }
        }
        super::persistence::validate(&pools);
        (rows, pools)
    }
}
