// Keep Candid metadata embedding restricted to local/development Wasm.

use super::should_embed_candid_metadata;
use canic_contracts::ids::BuildNetwork;

#[test]
fn candid_metadata_embedding_is_dev_only() {
    assert!(should_embed_candid_metadata(BuildNetwork::Local));
    assert!(!should_embed_candid_metadata(BuildNetwork::Ic));
}
