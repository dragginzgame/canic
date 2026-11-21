//! This application owns storage, authority, schema and fluent operations.
use crate::protocol::{
    InspectReply, InspectRequest, Page, QueryReply, QueryRequest, Receipt, Refusal, Row,
    WriteReply, WriteRequest,
};
use canic::db::Operation;
use ic_db::{Batch, Comparison, Database, Entity, Error, QueryCursor, Schema};
use ic_storage::{DefaultMemoryImpl, MemoryRuntime, RuntimeMemory, StableKey, StorageConfig};
use std::{cell::OnceCell, rc::Rc};

#[ic_db::canister]
struct Witness {}

#[ic_db::store(canister = Witness, key = "app.db.v1")]
struct Items {}

type Host = MemoryRuntime<DefaultMemoryImpl>;

thread_local! { static HOST: OnceCell<Rc<Host>> = const { OnceCell::new() }; }

///
/// Item
/// Stores the indexed scores used by the installed database witness.
///

#[ic_db::entity(
    store = crate::canister::Items,
    pk(field = "id"),
    index(fields = ["score", "id"]),
    fields(
        field(name = "id", value(item(prim = "Nat64"))),
        field(name = "score", value(item(prim = "Nat64")))
    )
)]
struct Item {}

///
/// UnreadableItem
/// Widens the stored score to exercise incompatible schema admission.
///

#[cfg(feature = "incompatible-schema")]
#[ic_db::entity(
    store = crate::canister::Items,
    name = "Item",
    pk(field = "id"),
    index(fields = ["score", "id"]),
    fields(
        field(name = "id", value(item(prim = "Nat64"))),
        field(name = "score", value(item(prim = "Nat128")))
    )
)]
struct UnreadableItem {}

fn schema() -> Schema {
    #[cfg(not(feature = "incompatible-schema"))]
    let entity = Item::SCHEMA;
    #[cfg(feature = "incompatible-schema")]
    let entity = UnreadableItem::SCHEMA;
    Schema::new(&[entity]).unwrap()
}

fn bootstrap(fresh: bool) -> Rc<Host> {
    #[cfg(feature = "fault-injection")]
    {
        assert!(matches!(
            ic_timers::metrics(),
            Err(ic_timers::Error::Uninitialized)
        ));
        canic::lifecycle::initialize_timers().unwrap();
        canic::lifecycle::initialize_timers().unwrap();
        let timers = canic::metrics::store::get::<ic_timers::MetricsStorage>().unwrap();
        assert_eq!(ic_timers::metrics().unwrap(), timers.snapshot());
        assert!(ic_timers::inventory().unwrap().timers.is_empty());

        let jobs = canic::metrics::store::get::<canic::jobs::MetricsStorage>()
            .expect("Jobs metrics attached before opening application state");
        assert_eq!(canic::jobs::metrics(), Some(jobs.snapshot()));
        // Jobs refuses a different recorder, even one with identical counters.
        canic::jobs::initialize_with_metrics(jobs).unwrap();
    }
    let host = Rc::new(canic::storage::bootstrap(
        DefaultMemoryImpl::default(),
        fresh,
        "app",
        &["app.db.v1", "app.owner.v1"],
        StorageConfig::default().pages,
    ));
    HOST.with(|slot| assert!(slot.set(Rc::clone(&host)).is_ok(), "duplicate bootstrap"));
    host
}

fn runtime() -> Rc<Host> {
    HOST.with(|slot| Rc::clone(slot.get().expect("DB lifecycle bootstrap")))
}

fn memory(host: &MemoryRuntime<DefaultMemoryImpl>, key: &str) -> RuntimeMemory<DefaultMemoryImpl> {
    host.open_memory(&StableKey::parse(key).unwrap()).unwrap()
}

fn open(host: &MemoryRuntime<DefaultMemoryImpl>) -> Result<Database, Error> {
    Database::open(memory(host, "app.db.v1"), schema())
}

#[cfg(feature = "managed-fixture")]
canic::mount_readiness_endpoint!(crate::canister::readiness);

#[cfg(feature = "managed-fixture")]
fn readiness() -> canic::readiness::State {
    use canic::readiness::State;
    match open(&runtime()) {
        Ok(_) => State::Ready,
        Err(Error::Maintenance) => State::Fenced,
        Err(_) => State::Unavailable,
    }
}

fn decode_owner(bytes: &[u8]) -> Result<ic_db::Principal, std::convert::Infallible> {
    // The managed test artifact pins a synthetic user independently of its controller.
    #[cfg(feature = "managed-fixture")]
    if bytes.is_empty() {
        return Ok(
            "xjfxn-sdn7v-35kn3-shpt7-357bn-nqgyl-x7ijr-2a2c5-gre6i-qcox2-4ae"
                .parse()
                .expect("fixture owner"),
        );
    }
    #[cfg(feature = "managed-fixture")]
    if bytes.len() > 29 {
        let topology = canic::topology::Install::decode(bytes).expect("project topology");
        topology
            .check_root(ic_platform_runtime::system::caller())
            .expect("project installer");
        assert_eq!(topology.application, [8; 16]);
        return Ok(ic_db::Principal::from_slice(topology.parent.as_slice()));
    }
    Ok({ ic_db::Principal::try_from_slice(bytes).expect("installation owner") })
}

// Open diagnostics and the application's named storage at install and upgrade.
canic::mount_lifecycle! {
    input = ic_db::Principal,
    max_bytes = 2097152,
    decode = decode_owner,
    install = initialize,
    reopen = reopen,
}

fn initialize(owner: ic_db::Principal) {
    assert_ne!(owner, ic_db::Principal::anonymous());
    let host = bootstrap(true);
    let owner_memory = memory(&host, "app.owner.v1");
    assert_eq!(owner_memory.size(), 0, "initialization only");
    owner_memory.grow(1).unwrap();
    let mut record = [0_u8; 34];
    record[..4].copy_from_slice(b"OWN1");
    record[4] = owner.as_slice().len() as u8;
    record[5..5 + owner.as_slice().len()].copy_from_slice(owner.as_slice());
    owner_memory.write(0, &record);
    // Fresh fixture installation identity; no init hook on upgrade.
    Database::create(memory(&host, "app.db.v1"), schema(), [91; 16]).unwrap();
}

fn retained_owner(host: &Host) -> ic_db::Principal {
    let owner_memory = memory(host, "app.owner.v1");
    let mut bytes = [0; 34];
    assert_eq!(owner_memory.size(), 1, "missing or oversized owner record");
    owner_memory.read(0, &mut bytes);
    assert_eq!(&bytes[..4], b"OWN1", "owner format");
    assert!(bytes[4] <= 29, "owner length");
    let owner = ic_db::Principal::try_from_slice(&bytes[5..5 + usize::from(bytes[4])]).unwrap();
    assert_ne!(
        owner,
        ic_db::Principal::anonymous(),
        "invalid retained owner"
    );
    owner
}

fn reopen() {
    let host = bootstrap(false);
    retained_owner(&host);
    match open(&host) {
        Ok(_) | Err(Error::Maintenance | Error::IncompatibleSchema) => (),
        Err(error) => panic!("retained database unavailable: {error:?}"),
    }
}

fn authorize(caller: ic_db::Principal, _operation: Operation) -> Result<(), String> {
    if caller == retained_owner(&runtime()) {
        Ok(())
    } else {
        Err("denied".into())
    }
}

fn refusal(error: Error) -> Refusal {
    match error {
        Error::Limit => Refusal::Limit,
        Error::Exists => Refusal::Exists,
        Error::DuplicateTarget => Refusal::DuplicateTarget,
        Error::Maintenance => Refusal::Maintenance,
        Error::InvalidCursor => Refusal::Cursor,
        other => panic!("unexpected database failure: {other:?}"),
    }
}

#[allow(clippy::needless_pass_by_value)]
fn inspect(_caller: ic_db::Principal, request: InspectRequest) -> InspectReply {
    use canic::db::inspection::{Admission, Cardinality};
    let host = runtime();
    // The pre-decode owner gate also permits this application's cardinality.
    // Closed/unsupported state is never opened through an alternate reader.
    let (admission, count) = match open(&host) {
        Ok(db) => {
            let count = if matches!(&request, InspectRequest::Entity { name, count: true, .. } if name.as_str() == Item::SCHEMA.name)
            {
                db.cardinality::<Item>()
                    .map(Cardinality::Available)
                    .unwrap_or(Cardinality::Unavailable)
            } else {
                Cardinality::NotExposed
            };
            (
                Admission::Readable {
                    incarnation: db.incarnation(),
                },
                count,
            )
        }
        Err(Error::Maintenance) => (Admission::Maintenance, Cardinality::Unavailable),
        Err(Error::IncompatibleSchema) => (Admission::Incompatible, Cardinality::Unavailable),
        Err(_) => (Admission::Unavailable, Cardinality::Unavailable),
    };
    let reply = canic::db::schema::inspect(&schema(), &request, admission, count);
    #[cfg(feature = "fault-injection")]
    let reply = controls::inspection_reply(reply);
    reply
}

#[allow(clippy::needless_pass_by_value)]
fn query(_caller: ic_db::Principal, request: QueryRequest) -> QueryReply {
    if !(1..=3).contains(&request.limit) || !(1..=4).contains(&request.work) {
        return Err(Refusal::Limit);
    }
    let host = runtime();
    let db = open(&host).map_err(refusal)?;
    let cursor = request
        .after
        .as_deref()
        .map(QueryCursor::from_bytes)
        .transpose()
        .map_err(refusal)?;
    let mut query = db
        .query::<Item>()
        .filter("score", Comparison::Ge, request.minimum)
        .limit(usize::from(request.limit))
        .work_limit(usize::from(request.work));
    if let Some(cursor) = &cursor {
        query = query.after(cursor);
    }
    let page = query.fetch().map_err(refusal)?;
    if page.output_bytes > 1024 {
        return Err(Refusal::Limit);
    }
    Ok(Page {
        rows: page
            .rows
            .into_iter()
            .map(|row| Row {
                id: row.id,
                score: row.score,
            })
            .collect(),
        next: page.next.map(|cursor| cursor.to_bytes()),
        examined: page.examined as u32,
    })
}

fn write(_caller: ic_db::Principal, rows: WriteRequest, _time: u64) -> WriteReply {
    if rows.is_empty() || rows.len() > 4 {
        return Err(Refusal::Limit);
    }
    let host = runtime();
    let mut db = open(&host).map_err(refusal)?;
    let inserted = rows.len() as u32;
    let mut batch = Batch::new();
    for row in rows {
        batch
            .insert(&Item {
                id: row.id,
                score: row.score,
            })
            .map_err(refusal)?;
    }
    let recorder = canic::metrics::store::get::<ic_db::metrics::MetricsStorage>()
        .expect("attached DB metrics");
    let before = recorder.snapshot();
    let result = db.commit(batch);
    let after = recorder.snapshot();
    assert_eq!(after.commits.total(), before.commits.total() + 1);
    if result.is_err() {
        assert_eq!(
            after.commit_failures.total(),
            before.commit_failures.total() + 1
        );
        let logs = canic::logging::page(None);
        assert_eq!(logs.entries[0].source, "ic-db.commit");
    }
    result.map_err(refusal)?;
    let receipt = Receipt {
        inserted,
        note: String::new(),
    };
    #[cfg(feature = "fault-injection")]
    let receipt = if controls::reply_failure() {
        Receipt {
            note: "x".repeat(65_536),
            ..receipt
        }
    } else {
        receipt
    };
    Ok(receipt)
}

canic::mount_db_endpoints! {
    protocol = crate::protocol,
    authorize = crate::canister::authorize,
    inspect = crate::canister::inspect,
    query = crate::canister::query,
    write = crate::canister::write,
}

#[cfg(feature = "fault-injection")]
mod controls {
    type EmptyReply = ();
    use super::{Entity, Operation, Schema, authorize, memory, open, runtime, schema};
    use canic::db::endpoint;
    use endpoint::api;
    use ic_db::{Maintenance, SchemaTransition};
    use ic_platform_runtime as runtime;
    use std::cell::Cell;

    thread_local! { static REPLY_FAILURE: Cell<bool> = const { Cell::new(false) }; }

    thread_local! { static INSPECTION_FAILURE: Cell<bool> = const { Cell::new(false) }; }

    pub(super) fn reply_failure() -> bool {
        REPLY_FAILURE.get()
    }

    // Switch this fixture from per-call opening to the same strictly reopened role store.
    mod database {
        canic::declare_db_store!(pub(super) mod store, key = "app.db.v1", schema = crate::canister::schema);
    }

    fn item(score: u64, id: u64) -> Result<Option<super::Item>, ic_db::Error> {
        use super::Item;
        use canic::db;
        db!()
            .read()?
            .query::<Item>()
            .eq(Item::SCORE, score)
            .eq(Item::ID, id)
            .work_limit(1)
            .try_one()
    }

    fn nested_update() -> Result<(), ic_db::Error> {
        use canic::db;
        database::store::attach(&runtime(), None);
        let outer = db!();
        let mut row = item(50, 7)?.expect("installed predecessor row");
        row.score = 51;
        let mut batch = ic_db::Batch::new();
        batch.replace(&row)?;
        outer.commit(batch)?;
        assert_eq!(item(51, 7)?.unwrap().score, 51);
        drop(outer);
        assert_eq!(database::store::with(|db| db.incarnation()), [91; 16]);
        Ok(())
    }

    // Deliberately corrupt an otherwise admitted application reply after projection.
    // This qualifies the endpoint's final byte ceiling, not a different DB schema.
    pub(super) fn inspection_reply(
        mut reply: crate::protocol::InspectReply,
    ) -> crate::protocol::InspectReply {
        use canic::db::inspection::{Body, Definition, Selection};
        if INSPECTION_FAILURE.get()
            && let Ok(response) = &mut reply
            && let Selection::Entity(entity) = &mut response.selection
        {
            let fields = (0..64)
                .map(|index| {
                    let mut field = entity.fields.as_slice()[0].clone();
                    field.name = ic_wire::Text::new(format!("field{index:059}")).unwrap();
                    field
                })
                .collect::<Vec<_>>();
            entity.types = ic_wire::List::new(
                (0..64)
                    .map(|index| Definition {
                        name: ic_wire::Text::new(format!("type{index:060}")).unwrap(),
                        body: Body::Record(ic_wire::List::new(fields.clone()).unwrap()),
                    })
                    .collect(),
            )
            .unwrap();
        }
        reply
    }

    ///
    /// FutureItem
    /// Describes the widened score used by the maintenance witness.
    ///

    #[ic_db::entity(
        store = crate::canister::Items,
        name = "Item",
        pk(field = "id"),
        index(fields = ["score", "id"]),
        fields(
            field(name = "id", value(item(prim = "Nat64"))),
            field(name = "score", value(item(prim = "Nat128")))
        )
    )]
    struct FutureItem {}

    /// Require write access before admitting the bounded fault-control frame.
    fn admit() -> Result<(), String> {
        authorize(api::caller(), Operation::Write)?;
        endpoint::admit::<991>()
    }

    // Exercise the selected supplier in the same installed actor as DB, Jobs and Timers.
    async fn verify_entropy() {
        use canic::entropy::supplier;
        let recorder = canic::metrics::store::get::<supplier::MetricsStorage>()
            .expect("attached entropy recorder");
        let before = recorder.snapshot();
        supplier::fresh_seed().await.expect("fresh IC entropy");
        assert_eq!(
            recorder.snapshot().fresh.succeeded.total(),
            before.fresh.succeeded.total() + 1
        );

        supplier::suspend().unwrap();
        assert!(matches!(
            supplier::fresh_seed().await,
            Err(supplier::Error::Disabled)
        ));
        supplier::resume_fresh_requests().unwrap();
        let logs = canic::logging::page(None);
        assert_eq!(logs.entries[0].source, "ic-entropy");
        assert_eq!(logs.entries[0].message, "Fresh requests resumed");
        assert_eq!(logs.entries[1].message, "Entropy suspended");
    }

    /// Select a database failure point for the installed rollback witness.
    #[runtime::update(
        runtime = runtime,
        guard = admit,
        decode = endpoint::decode::<u8, 991>,
        max_bytes = 2097152,
        on_error = endpoint::fail,
        name = "db_test_control",
        encode = endpoint::encode::<EmptyReply, 991>,
    )]
    async fn db_test_control(control: u8) -> EmptyReply {
        match control {
            7 => verify_entropy().await,
            8 => {
                use canic::jobs::{self, Outcome, Policy, RetryPolicy, Schedule};
                thread_local! {
                    static JOB: std::cell::OnceCell<jobs::Job> = const { std::cell::OnceCell::new() };
                }
                let job = jobs::bind(
                    ["witness", "metrics", "once"],
                    Schedule::once(ic_platform_runtime::system::time()),
                    Policy::SkipLate {
                        lateness_ns: u64::MAX,
                    },
                    RetryPolicy::NONE,
                    |_| async { Outcome::Success },
                )
                .unwrap();
                JOB.with(|slot| assert!(slot.set(job).is_ok()));
            }
            0 => REPLY_FAILURE.set(false),
            1 => REPLY_FAILURE.set(true),
            4 => INSPECTION_FAILURE.set(true),
            5 => INSPECTION_FAILURE.set(false),
            6 => nested_update().unwrap(),
            2 | 3 => {
                let host = runtime();
                let target = Schema::new(&[FutureItem::SCHEMA]).unwrap();
                let transition =
                    SchemaTransition::new(&schema(), &target, "widen-score", 1, &[]).unwrap();
                if control == 2 {
                    open(&host).unwrap().begin_maintenance(&transition).unwrap();
                } else {
                    Maintenance::resume(memory(&host, "app.db.v1"), &transition)
                        .unwrap()
                        .cancel();
                }
            }
            _ => panic!("unknown fixture control"),
        }
    }
}
