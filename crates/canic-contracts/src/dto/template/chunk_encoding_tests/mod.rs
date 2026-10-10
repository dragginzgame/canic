//! Owned and borrowed Store requests retain one complete wire contract.

use super::{TemplateChunkInput, TemplateChunkInputRef, TemplateChunkSetPrepareInput};
use crate::ids::{TemplateId, TemplateVersion};
use candid::types::{internal::TypeContainer, subtype};
use std::collections::HashSet;

#[test]
fn borrowed_chunk_input_matches_owned_schema_and_bytes() {
    let mut types = TypeContainer::new();
    let owned = types.add::<TemplateChunkInput>();
    let borrowed = types.add::<TemplateChunkInputRef<'static>>();
    subtype::equal(&mut HashSet::new(), &types.env, &borrowed, &owned).unwrap();

    let template_id = TemplateId::new("test-chunk");
    let version = TemplateVersion::new("0.110.54");
    let preparation = TemplateChunkSetPrepareInput {
        manifest: None,
        template_id: template_id.clone(),
        version: version.clone(),
        payload_hash: vec![1; 32],
        payload_size_bytes: 3,
        chunk_hashes: vec![vec![2; 32]],
    };
    for preparation in [None, Some(preparation)] {
        let owned = TemplateChunkInput {
            preparation,
            template_id: template_id.clone(),
            version: version.clone(),
            chunk_index: 0,
            bytes: vec![0, 127, 255],
        };
        let borrowed = TemplateChunkInputRef {
            preparation: owned.preparation.as_ref(),
            template_id: &owned.template_id,
            version: &owned.version,
            chunk_index: owned.chunk_index,
            bytes: &owned.bytes,
        };
        let bytes = candid::encode_one(borrowed).unwrap();
        assert_eq!(bytes, candid::encode_one(&owned).unwrap());
        assert_eq!(
            candid::decode_one::<TemplateChunkInput>(&bytes).unwrap(),
            owned
        );
    }
}
