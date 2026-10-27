//! Owned and borrowed publication frames preserve exact first-chunk metadata and bytes.
use super::*;
use crate::{
    dto::{
        template::{TemplateChunkInput, TemplateChunkSetPrepareInput},
        template_bytes::{PublishRequest, PublishRequestRef},
    },
    ids::{TemplateId, TemplateVersion},
};

#[test]
fn borrowed_and_owned_requests_encode_identically_with_and_without_preparation() {
    let template_id = TemplateId::new("test-chunk");
    let version = TemplateVersion::new("current");
    let preparation = TemplateChunkSetPrepareInput {
        manifest: None,
        template_id: template_id.clone(),
        version: version.clone(),
        payload_hash: vec![1; 32],
        payload_size_bytes: 1_048_576,
        chunk_hashes: vec![vec![2; 32]],
    };
    for preparation in [None, Some(preparation)] {
        let original = TemplateChunkInput {
            preparation,
            template_id: template_id.clone(),
            version: version.clone(),
            chunk_index: 0,
            bytes: vec![0xab; 1_048_576],
        };
        let borrowed = PublishRequestRef::from(&original);
        let native = PublishRequest::from(original.clone());
        let bytes = ic_wire::native::encode_request(METHOD_ID, &borrowed, REQUEST_LIMITS).unwrap();
        assert_eq!(
            bytes,
            ic_wire::native::encode_request(METHOD_ID, &native, REQUEST_LIMITS).unwrap()
        );
        let decoded: PublishRequest =
            ic_wire::native::decode_request(METHOD_ID, &bytes, REQUEST_LIMITS).unwrap();
        assert_eq!(TemplateChunkInput::from(decoded), original);
        assert!(bytes.len() < 1_049_600);
        assert!(matches!(
            ic_wire::native::decode_request::<PublishRequest>(
                METHOD_ID + 1,
                &bytes,
                REQUEST_LIMITS
            ),
            Err(ic_wire::Error::Identity)
        ));
        let mut trailing = bytes;
        trailing.push(0);
        assert!(matches!(
            ic_wire::native::decode_request::<PublishRequest>(METHOD_ID, &trailing, REQUEST_LIMITS),
            Err(ic_wire::Error::Trailing)
        ));
    }
}

#[test]
fn oversized_publication_refuses_the_frame_before_storage() {
    let request = PublishRequest {
        preparation: None,
        template_id: "template".into(),
        version: "current".into(),
        chunk_index: 1,
        bytes: vec![0; REQUEST_LIMITS.bytes],
    };
    assert!(ic_wire::native::encode_request(METHOD_ID, &request, REQUEST_LIMITS).is_err());
}

#[cfg(feature = "wire-schema")]
#[test]
fn publication_has_complete_native_metadata() {
    ic_wire::typescript::native_schema::<PublishRequest>().unwrap();
    ic_wire::typescript::native_schema::<Result<(), crate::dto::error::Error>>().unwrap();
}
