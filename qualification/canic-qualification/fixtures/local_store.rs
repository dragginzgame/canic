//! Local Root and Wasm Store used by installed qualification fleets.
use canic_management_protocol::stored::{STORE_CHUNK_LIMITS, STORE_LIMITS};
use ic_testkit::pocket_ic::{CanisterId, nonblocking::PocketIc};
use ic_wasm_store_protocol as store;
use serde::{Serialize, de::DeserializeOwned};

pub(crate) struct Client<'a> {
    pub(crate) pic: &'a PocketIc,
    pub(crate) id: CanisterId,
    pub(crate) operator: CanisterId,
}
impl Client<'_> {
    pub(crate) async fn update<T: Serialize, R: DeserializeOwned>(
        &self,
        method: &str,
        id: u64,
        input: &T,
        limits: ic_wire::Limits,
    ) -> R {
        let reply = self
            .pic
            .update_call(
                self.id,
                self.operator,
                method,
                ic_wire::encode(
                    input,
                    ic_wire::Encoding::Frame {
                        method: id,
                        direction: ic_wire::Direction::Request,
                    },
                    limits,
                )
                .unwrap(),
            )
            .await
            .unwrap();
        ic_wire::decode(
            &reply,
            ic_wire::Encoding::Frame {
                method: id,
                direction: ic_wire::Direction::Response,
            },
            limits,
        )
        .unwrap()
    }

    pub(crate) async fn publish_named(
        &self,
        name: &str,
        version: &str,
        bytes: &[u8],
    ) -> store::Manifest {
        let manifest = store::Manifest {
            release: store::Release {
                name: name.into(),
                version: version.into(),
            },
            hash: store::digest(bytes),
            bytes: bytes.len() as u64,
            chunks: bytes
                .chunks(store::CHUNK_BYTES)
                .map(store::digest)
                .collect(),
        };
        self.update::<_, Result<store::Status, store::Error>>(
            "prepare",
            store::PREPARE,
            &manifest,
            STORE_LIMITS,
        )
        .await
        .unwrap();
        for (index, bytes) in bytes.chunks(store::CHUNK_BYTES).enumerate() {
            self.update::<_, Result<store::Status, store::Error>>(
                "put",
                store::PUT,
                &store::Put {
                    release: manifest.release.clone(),
                    index: index as u32,
                    bytes: bytes.to_vec(),
                },
                STORE_CHUNK_LIMITS,
            )
            .await
            .unwrap();
        }
        self.update::<_, Result<store::Status, store::Error>>(
            "publish",
            store::PUBLISH,
            &manifest.release,
            STORE_LIMITS,
        )
        .await
        .unwrap();
        manifest
    }
}

pub(crate) async fn fixture_with_code(
    pic: &PocketIc,
    root_wasm: Vec<u8>,
    store_wasm: Vec<u8>,
) -> (Client<'_>, Client<'_>) {
    let operator = CanisterId::self_authenticating(b"Store bootstrap operator");
    let root = pic
        .create_canister_with_settings(Some(operator), None)
        .await;
    let supplier = pic
        .create_canister_with_settings(Some(operator), None)
        .await;
    for (id, wasm) in [(root, root_wasm), (supplier, store_wasm)] {
        pic.add_cycles(id, 10_000_000_000_000).await;
        pic.install_canister(id, wasm, vec![], Some(operator)).await;
    }
    (
        Client {
            pic,
            id: root,
            operator,
        },
        Client {
            pic,
            id: supplier,
            operator,
        },
    )
}
