//! Installed application + Canic mount + actual IC DB. Expected rows are literal
//! application outcomes; no engine codec/query implementation in this host.
mod managed;
use canic::db::RPC_LIMITS;
use canic::db::inspection::{Admission, Cardinality, Entity, Kind, Request, Selection};
use canic_db_witness::protocol::{
    INSPECT, InspectReply, QUERY, QueryReply, QueryRequest, Refusal, Row, WRITE, WriteReply,
};
use ic_testkit::pocket_ic::common::rest::BlobCompression;
use ic_testkit::pocket_ic::{CanisterId as Principal, PocketIcBuilder, nonblocking::PocketIc};
use ic_testkit::server::{Limits, PreparedServer, Server};
use serde::{Serialize, de::DeserializeOwned};
use std::{path::PathBuf, time::Duration};

fn frame<T: Serialize>(id: u64, input: &T, direction: ic_wire::Direction) -> Vec<u8> {
    ic_wire::encode(
        input,
        ic_wire::Encoding::Frame {
            method: id,
            direction,
        },
        RPC_LIMITS,
    )
    .unwrap()
}
struct Client<'a> {
    pic: &'a PocketIc,
    canister: Principal,
    owner: Principal,
}
impl Client<'_> {
    async fn query<T: Serialize, R: DeserializeOwned>(
        &self,
        method: &str,
        id: u64,
        input: &T,
    ) -> R {
        let bytes = self
            .pic
            .query_call(
                self.canister,
                self.owner,
                method,
                frame(id, input, ic_wire::Direction::Request),
            )
            .await
            .unwrap();
        ic_wire::decode(
            &bytes,
            ic_wire::Encoding::Frame {
                method: id,
                direction: ic_wire::Direction::Response,
            },
            RPC_LIMITS,
        )
        .unwrap()
    }
    async fn update<T: Serialize, R: DeserializeOwned>(
        &self,
        method: &str,
        id: u64,
        input: &T,
    ) -> R {
        let bytes = self
            .pic
            .update_call(
                self.canister,
                self.owner,
                method,
                frame(id, input, ic_wire::Direction::Request),
            )
            .await
            .unwrap();
        ic_wire::decode(
            &bytes,
            ic_wire::Encoding::Frame {
                method: id,
                direction: ic_wire::Direction::Response,
            },
            RPC_LIMITS,
        )
        .unwrap()
    }
    async fn catalogue(&self) -> canic::db::inspection::Response {
        self.query::<_, InspectReply>("db_inspect", INSPECT, &Request::Catalogue)
            .await
            .unwrap()
    }
    async fn inspect(&self) -> Entity {
        let catalog = self.catalogue().await;
        let reply = self
            .query::<_, InspectReply>(
                "db_inspect",
                INSPECT,
                &Request::Entity {
                    schema: catalog.schema,
                    name: ic_wire::Text::new("Item".into()).unwrap(),
                    count: true,
                },
            )
            .await
            .unwrap();
        let Selection::Entity(entity) = reply.selection else {
            panic!("entity description");
        };
        entity
    }
    async fn page(&self, request: &QueryRequest) -> QueryReply {
        self.query("db_query", QUERY, request).await
    }
    async fn write(&self, rows: &[Row]) -> WriteReply {
        self.update("db_write", WRITE, &rows).await
    }
    async fn control(&self, command: u8) {
        self.update::<_, ()>("db_test_control", 991, &command).await;
    }
}
fn row(id: u64, score: u64) -> Row {
    Row { id, score }
}
fn wasm(name: &str) -> Vec<u8> {
    std::fs::read(std::env::var_os(name).expect("run integration script")).unwrap()
}

#[tokio::test]
async fn typed_application_mount_preserves_authority_pages_and_commit_boundary() {
    let directory = PathBuf::from(
        std::env::var_os("IC_TESTKIT_SERVER_DIRECTORY").expect("prepared testkit directory"),
    );
    let mut server = Server::start(
        PreparedServer::check(&directory).unwrap(),
        Limits {
            startup: Duration::from_secs(30),
            lifetime: Duration::from_secs(180),
            stdout_bytes: 1024 * 1024,
            stderr_bytes: 1024 * 1024,
        },
    )
    .unwrap();
    let instance = server
        .instance(
            PocketIcBuilder::new().with_application_subnet(),
            Duration::from_secs(30),
        )
        .await
        .unwrap();
    let pic = instance.client();
    let owner = Principal::self_authenticating([11]);
    let denied = Principal::self_authenticating([12]);
    let canister = pic.create_canister().await;
    pic.add_cycles(canister, 10_000_000_000_000).await;
    let normal = wasm("CANIC_DB_WASM");
    let fault = wasm("CANIC_DB_FAULT_WASM");
    let incompatible = wasm("CANIC_DB_INCOMPATIBLE_WASM");
    pic.install_canister(canister, normal.clone(), owner.as_slice().to_vec(), None)
        .await;
    let client = Client {
        pic,
        canister,
        owner,
    };
    let info = client.inspect().await;
    assert_eq!(info.name.as_str(), "Item");
    assert_eq!(info.primary_key.as_str(), "id");
    assert_eq!(
        info.fields
            .as_slice()
            .iter()
            .map(|f| (f.name.as_str(), f.value.kind.clone()))
            .collect::<Vec<_>>(),
        [("id", Kind::Unsigned(64)), ("score", Kind::Unsigned(64))]
    );
    assert_eq!(info.indexes.as_slice()[0].name.as_str(), "score,id");
    assert_eq!(info.cardinality, Cardinality::Available(0));
    let catalogue = client.catalogue().await;
    assert_eq!(
        catalogue.admission,
        Admission::Readable {
            incarnation: [91; 16]
        }
    );
    assert_eq!(
        catalogue.selection,
        Selection::Catalogue(
            ic_wire::List::new(vec![ic_wire::Text::new("Item".into()).unwrap()]).unwrap()
        )
    );
    let no_count = client
        .query::<_, InspectReply>(
            "db_inspect",
            INSPECT,
            &Request::Entity {
                schema: catalogue.schema,
                name: ic_wire::Text::new("Item".into()).unwrap(),
                count: false,
            },
        )
        .await
        .unwrap();
    let Selection::Entity(no_count) = no_count.selection else {
        panic!("entity");
    };
    assert_eq!(no_count.cardinality, Cardinality::NotExposed);
    for (hash, name, expected) in [
        (
            [0; 32],
            "Item",
            canic::db::inspection::Refusal::SchemaChanged,
        ),
        (
            catalogue.schema,
            "missing",
            canic::db::inspection::Refusal::UnknownEntity,
        ),
    ] {
        assert_eq!(
            client
                .query::<_, InspectReply>(
                    "db_inspect",
                    INSPECT,
                    &Request::Entity {
                        schema: hash,
                        name: ic_wire::Text::new(name.into()).unwrap(),
                        count: false
                    }
                )
                .await,
            Err(expected)
        );
    }
    // Production artifact has no private maintenance/fault entrypoint.
    assert!(
        pic.update_call(
            canister,
            owner,
            "db_test_control",
            frame(991, &1_u8, ic_wire::Direction::Request)
        )
        .await
        .is_err()
    );
    for (method, id) in [("db_inspect", INSPECT), ("db_query", QUERY)] {
        let error = pic
            .query_call(canister, denied, method, vec![0])
            .await
            .unwrap_err();
        assert!(
            format!("{error:?}").contains("denied"),
            "authorization before malformed decode"
        );
        assert!(
            pic.query_call(
                canister,
                owner,
                method,
                frame(id + 1, &(), ic_wire::Direction::Request)
            )
            .await
            .is_err()
        );
        assert!(
            pic.query_call(
                canister,
                owner,
                method,
                frame(id, &(), ic_wire::Direction::Response)
            )
            .await
            .is_err()
        );
        assert!(
            pic.query_call(canister, owner, method, vec![0; RPC_LIMITS.bytes + 1])
                .await
                .is_err()
        );
    }
    let error = pic
        .update_call(canister, denied, "db_write", vec![0])
        .await
        .unwrap_err();
    assert!(format!("{error:?}").contains("denied"));
    for bytes in [
        vec![0],
        frame(WRITE + 1, &vec![row(1, 1)], ic_wire::Direction::Request),
        frame(WRITE, &vec![row(1, 1)], ic_wire::Direction::Response),
        vec![0; RPC_LIMITS.bytes + 1],
    ] {
        assert!(
            pic.update_call(canister, owner, "db_write", bytes)
                .await
                .is_err()
        );
    }
    assert_eq!(
        client.inspect().await.cardinality,
        Cardinality::Available(0)
    );
    assert_eq!(
        client
            .write(&[row(1, 0), row(2, 0), row(3, 10), row(4, 20)])
            .await
            .unwrap()
            .inserted,
        4
    );
    assert_eq!(
        client.write(&[row(5, 30), row(3, 999)]).await,
        Err(Refusal::Exists)
    );
    assert_eq!(
        client.write(&[row(5, 30), row(5, 40)]).await,
        Err(Refusal::DuplicateTarget)
    );
    assert_eq!(
        client.inspect().await.cardinality,
        Cardinality::Available(4)
    );
    let mut request = QueryRequest {
        minimum: 10,
        limit: 2,
        work: 2,
        after: None,
    };
    let first = client.page(&request).await.unwrap();
    assert!(first.rows.is_empty());
    assert_eq!(first.examined, 2);
    request.after = Some(first.next.unwrap());
    let mut mismatched = request.clone();
    mismatched.minimum = 11;
    assert_eq!(client.page(&mismatched).await, Err(Refusal::Cursor));
    let mut invalid = request.clone();
    invalid.after = Some(vec![0]);
    assert_eq!(client.page(&invalid).await, Err(Refusal::Cursor));
    invalid = request.clone();
    invalid.work = 0;
    assert_eq!(client.page(&invalid).await, Err(Refusal::Limit));
    let before = pic.get_stable_memory(canister).await;
    let original_module = pic
        .canister_status(canister, None)
        .await
        .unwrap()
        .module_hash
        .unwrap();
    assert!(
        canic_qualification::upgrade_stopped(pic, canister, fault.clone(), vec![1], None)
            .await
            .is_err()
    );
    assert_eq!(pic.get_stable_memory(canister).await, before);
    assert_eq!(
        client.page(&request).await.unwrap().rows,
        [row(3, 10), row(4, 20)]
    );
    assert_eq!(
        pic.canister_status(canister, None)
            .await
            .unwrap()
            .module_hash
            .as_ref(),
        Some(&original_module)
    );
    // Upgrade loses heap caches, not authority/data/continuation.
    canic_qualification::upgrade_stopped(pic, canister, normal.clone(), vec![], None)
        .await
        .unwrap();
    assert_eq!(
        client.inspect().await.cardinality,
        Cardinality::Available(4)
    );
    let page = client.page(&request).await.unwrap();
    assert_eq!(page.rows, [row(3, 10), row(4, 20)]);
    if let Some(next) = page.next {
        request.after = Some(next);
        let terminal = client.page(&request).await.unwrap();
        assert!(terminal.rows.is_empty());
        assert!(terminal.next.is_none());
    }
    assert!(
        pic.query_call(
            canister,
            denied,
            "db_inspect",
            frame(INSPECT, &(), ic_wire::Direction::Request)
        )
        .await
        .is_err()
    );
    canic_qualification::upgrade_stopped(pic, canister, fault.clone(), vec![], None)
        .await
        .unwrap();
    let before_limit = client.catalogue().await;
    let backing = pic.get_stable_memory(canister).await;
    client.control(4).await;
    let error = pic
        .query_call(
            canister,
            owner,
            "db_inspect",
            frame(
                INSPECT,
                &Request::Entity {
                    schema: before_limit.schema,
                    name: ic_wire::Text::new("Item".into()).unwrap(),
                    count: true,
                },
                ic_wire::Direction::Request,
            ),
        )
        .await
        .unwrap_err();
    assert!(
        error.reject_message.contains("endpoint refused: Encode("),
        "oversized inspection must reach the final encoder: {error:?}"
    );
    assert_eq!(pic.get_stable_memory(canister).await, backing);
    client.control(5).await;
    assert_eq!(client.catalogue().await, before_limit);
    assert_eq!(
        client.inspect().await.cardinality,
        Cardinality::Available(4)
    );
    client.control(1).await;
    let error = pic
        .update_call(
            canister,
            owner,
            "db_write",
            frame(
                WRITE,
                &vec![row(5, 30), row(6, 40)],
                ic_wire::Direction::Request,
            ),
        )
        .await
        .unwrap_err();
    assert!(
        error.reject_message.contains("endpoint refused: Encode("),
        "must reach post-mutation encoder: {error:?}"
    );
    assert_eq!(
        client.inspect().await.cardinality,
        Cardinality::Available(4)
    );
    client.control(0).await;
    assert_eq!(
        client
            .write(&[row(5, 30), row(6, 40)])
            .await
            .unwrap()
            .inserted,
        2
    );
    client.control(2).await;
    assert_eq!(client.catalogue().await.admission, Admission::Maintenance);
    assert_eq!(client.inspect().await.cardinality, Cardinality::Unavailable);
    request.after = None;
    assert_eq!(client.page(&request).await, Err(Refusal::Maintenance));
    assert_eq!(client.write(&[row(7, 50)]).await, Err(Refusal::Maintenance));
    // Even the ordinary source executable must respect the frozen gate.
    canic_qualification::upgrade_stopped(pic, canister, normal.clone(), vec![], None)
        .await
        .unwrap();
    assert_eq!(client.inspect().await.cardinality, Cardinality::Unavailable);
    assert_eq!(client.write(&[row(7, 50)]).await, Err(Refusal::Maintenance));
    canic_qualification::upgrade_stopped(pic, canister, fault, vec![], None)
        .await
        .unwrap();
    client.control(3).await;
    assert_eq!(
        client.inspect().await.cardinality,
        Cardinality::Available(6)
    );
    assert_eq!(client.write(&[row(7, 50)]).await.unwrap().inserted, 1);
    // A different executable can describe its schema, but cannot silently open
    // or repair the retained database. Inspection itself leaves backing intact.
    canic_qualification::upgrade_stopped(pic, canister, incompatible, vec![], None)
        .await
        .unwrap();
    let backing = pic.get_stable_memory(canister).await;
    let changed = client.catalogue().await;
    assert_ne!(changed.schema, catalogue.schema);
    assert_eq!(changed.admission, Admission::Incompatible);
    assert_eq!(client.inspect().await.cardinality, Cardinality::Unavailable);
    assert_eq!(pic.get_stable_memory(canister).await, backing);
    canic_qualification::upgrade_stopped(pic, canister, normal.clone(), vec![], None)
        .await
        .unwrap();
    assert_eq!(client.catalogue().await.schema, catalogue.schema);
    assert_eq!(
        client.inspect().await.cardinality,
        Cardinality::Available(7)
    );
    // Model an installation with no directory. Reopen must refuse without
    // allocating even an empty owner/database or accepting new authority.
    let mut corrupt = pic.get_stable_memory(canister).await;
    corrupt[0] ^= 0xff;
    for damaged in [corrupt, vec![]] {
        pic.set_stable_memory(canister, damaged.clone(), BlobCompression::NoCompression)
            .await;
        assert!(
            canic_qualification::upgrade_stopped(pic, canister, normal.clone(), vec![], None)
                .await
                .is_err()
        );
        assert_eq!(pic.get_stable_memory(canister).await, damaged);
    }
    assert_eq!(
        instance.close(std::time::Duration::from_secs(10)).await,
        ic_testkit::server::Deletion::Attempted
    );
    let stopped = server.stop().unwrap();
    assert!(stopped.process.cleanup.is_empty(), "{stopped:?}");
}
