//! Exact original-operation status of receiver-local caller authority.
pub const METHOD: &str = "canic_caller_authority_status";
pub const METHOD_ID: u64 = 0x4341_4e49_0000_010b;
pub const REQUEST_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 128,
    allocation: 4096,
    nodes: 256,
    depth: 8,
};
pub const RESPONSE_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 2_097_152,
    allocation: 67_108_864,
    nodes: 4_194_304,
    depth: 64,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto::role::OperationStatusRequest;

    #[test]
    fn operation_request_fits_budget_and_rejects_wrong_or_excess_bytes() {
        let request = OperationStatusRequest {
            operation_id: [255; 32],
        };
        let bytes = ic_wire::native::encode_request(METHOD_ID, &request, REQUEST_LIMITS).unwrap();
        assert_eq!(
            ic_wire::native::decode_request::<OperationStatusRequest>(
                METHOD_ID,
                &bytes,
                REQUEST_LIMITS
            )
            .unwrap(),
            request
        );
        assert_eq!(
            ic_wire::native::decode_request::<OperationStatusRequest>(
                METHOD_ID + 1,
                &bytes,
                REQUEST_LIMITS
            ),
            Err(ic_wire::Error::Identity)
        );
        let mut trailing = bytes;
        trailing.push(0);
        assert_eq!(
            ic_wire::native::decode_request::<OperationStatusRequest>(
                METHOD_ID,
                &trailing,
                REQUEST_LIMITS
            ),
            Err(ic_wire::Error::Trailing)
        );
        let oversized = vec![0; REQUEST_LIMITS.bytes + 1];
        assert_eq!(
            ic_wire::native::decode_request::<OperationStatusRequest>(
                METHOD_ID,
                &oversized,
                REQUEST_LIMITS
            ),
            Err(ic_wire::Error::Limit)
        );
    }

    #[cfg(feature = "wire-schema")]
    #[test]
    fn original_receipt_graph_has_native_schema() {
        ic_wire::typescript::native_schema::<crate::dto::role::OperationStatusRequest>().unwrap();
        ic_wire::typescript::native_schema::<
            Result<
                crate::dto::caller_authority_status::CallerAuthorityStatus,
                crate::dto::error::Error,
            >,
        >()
        .unwrap();
    }
}
