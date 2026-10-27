//! Distinct native identities preserve both canonical accounting cursor domains.
use super::*;
use crate::dto::release_intents::IntentReleaseKey;

#[test]
fn accounting_cursors_preserve_store_and_original_key() {
    for cursor in [
        None,
        Some(IntentReleaseKey::Local(u64::MAX)),
        Some(IntentReleaseKey::ReceiptBacked([255; 32])),
    ] {
        let bytes =
            ic_wire::native::encode_request(intents::METHOD_ID, &cursor, REQUEST_LIMITS).unwrap();
        let decoded: Option<IntentReleaseKey> =
            ic_wire::native::decode_request(intents::METHOD_ID, &bytes, REQUEST_LIMITS).unwrap();
        assert_eq!(decoded, cursor);
        assert_eq!(
            ic_wire::native::decode_request::<Option<IntentReleaseKey>>(
                receipts::METHOD_ID,
                &bytes,
                REQUEST_LIMITS
            ),
            Err(ic_wire::Error::Identity)
        );
        assert_eq!(
            ic_wire::native::decode_response::<Option<IntentReleaseKey>>(
                intents::METHOD_ID,
                &bytes,
                RESPONSE_LIMITS
            ),
            Err(ic_wire::Error::Identity)
        );
    }
}

#[test]
fn replay_cursor_requires_its_exact_complete_frame() {
    for cursor in [None, Some([255; 32])] {
        let bytes =
            ic_wire::native::encode_request(receipts::METHOD_ID, &cursor, REQUEST_LIMITS).unwrap();
        let decoded: Option<[u8; 32]> =
            ic_wire::native::decode_request(receipts::METHOD_ID, &bytes, REQUEST_LIMITS).unwrap();
        assert_eq!(decoded, cursor);
        let mut trailing = bytes;
        trailing.push(0);
        assert_eq!(
            ic_wire::native::decode_request::<Option<[u8; 32]>>(
                receipts::METHOD_ID,
                &trailing,
                REQUEST_LIMITS
            ),
            Err(ic_wire::Error::Trailing)
        );
    }
}
