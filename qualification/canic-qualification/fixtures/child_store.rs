//! Publish the exact child module before exercising a managed Root endpoint.
use super::local_store::Client;
use canic_management_protocol::{
    application::Deployment,
    children::{ChildId, Request},
    stored,
};
use ic_testkit::pocket_ic::{CanisterId, nonblocking::PocketIc};
use serde::Serialize;

pub(crate) struct ChildStore<'a>(Client<'a>);

impl<'a> ChildStore<'a> {
    pub(crate) async fn install(pic: &'a PocketIc, root: CanisterId, wasm: Vec<u8>) -> Self {
        let id = pic.create_canister().await;
        pic.install_canister(id, wasm, vec![], None).await;
        pic.set_controllers(id, None, vec![root]).await.unwrap();
        Self::bound(pic, root, id)
    }

    pub(crate) fn bound(pic: &'a PocketIc, root: CanisterId, id: CanisterId) -> Self {
        Self(Client {
            pic,
            id,
            operator: root,
        })
    }

    pub(crate) async fn frame<A: Serialize>(
        &self,
        method: u64,
        slot: ChildId,
        approval: &A,
        wasm: &[u8],
    ) -> Vec<u8> {
        let version: String = ic_wasm_store_protocol::digest(wasm)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        let manifest = self.0.publish_named("child", &version, wasm).await;
        let input = Request {
            slot,
            value: Deployment::<_, ()>::Stored(stored::Stage {
                approval,
                reference: stored::Reference {
                    store: self.0.id.to_text(),
                    manifest,
                },
            }),
        };
        ic_wire::encode(
            &input,
            ic_wire::Encoding::Frame {
                method,
                direction: ic_wire::Direction::Request,
            },
            stored::LIMITS,
        )
        .unwrap()
    }
}
