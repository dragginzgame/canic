use super::should_embed_candid_metadata;
use canic_core::ids::BuildNetwork;

// Keep Candid metadata embedding restricted to local/development Wasm.
#[test]
fn candid_metadata_embedding_is_dev_only() {
    assert!(should_embed_candid_metadata(BuildNetwork::Local));
    assert!(!should_embed_candid_metadata(BuildNetwork::Ic));
}
