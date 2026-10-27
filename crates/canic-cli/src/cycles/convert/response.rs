//! Module: cycles::convert::response
//!
//! Responsibility: decode and classify the typed response from a live ICP refill.
//! Does not own: command execution, pending-operation persistence, or request policy.
//! Boundary: validates native command receipts and retained Wire status evidence.

use crate::cycles::CyclesCommandError;
use canic_contracts::dto::{
    error::Error, funding_read::RefillResult, icp_refill::IcpRefillResponse, role::OperationReceipt,
};
use canic_core::{
    cdk::utils::hash::hex_bytes, shared_support::icp_refill::icp_refill_outcome_is_resumable,
};

/// One decoded, operation-bound live refill response.

#[derive(Debug)]
pub(super) struct DecodedIcpRefillResponse {
    response: IcpRefillResponse,
}

impl DecodedIcpRefillResponse {
    #[must_use]
    pub(super) const fn is_resumable(&self) -> bool {
        icp_refill_outcome_is_resumable(
            self.response.status,
            self.response.error_code,
            self.response.ledger_block_index.is_some(),
        )
    }

    #[must_use]
    pub(super) fn render(&self, json: bool) -> String {
        if json {
            return serde_json::json!({
                "operation_id": hex_bytes(self.response.operation_id),
                "status": format!("{:?}", self.response.status),
                "ledger_block_index": self.response.ledger_block_index,
                "cycles_sent": self.response.cycles_sent.as_ref().map(|cycles| cycles.0.to_string()),
                "error_code": self.response.error_code.map(|code| format!("{code:?}")),
                "error_message": self.response.error_message,
            })
            .to_string();
        }

        format!(
            "operation_id={}\nstatus={:?}\nledger_block_index={}\ncycles_sent={}\nerror_code={}\nerror_message={:?}",
            hex_bytes(self.response.operation_id),
            self.response.status,
            optional_display(self.response.ledger_block_index.as_ref()),
            optional_display(self.response.cycles_sent.as_ref()),
            optional_debug(self.response.error_code),
            self.response.error_message,
        )
    }
}

pub(super) fn validate_icp_refill_command_response(
    response: Result<OperationReceipt, Error>,
    expected_operation_id: [u8; 32],
) -> Result<(), CyclesCommandError> {
    let response = match response {
        Ok(response) => response,
        Err(error) => {
            let code = error.code();
            return Err(CyclesCommandError::IcpRefillRejected {
                code,
                diagnostic: canic_host::diagnostics::render_diagnostic(code),
            });
        }
    };
    verify_operation_id(expected_operation_id, response.operation_id)?;
    Ok(())
}

pub(super) fn validate_icp_refill_status_response(
    response: Result<RefillResult, Error>,
    expected_operation_id: [u8; 32],
) -> Result<DecodedIcpRefillResponse, CyclesCommandError> {
    let response = match response {
        Ok(response) => IcpRefillResponse::try_from(response)
            .map_err(canic_host::icp::IcpWireQueryError::from)?,
        Err(error) => {
            let code = error.code();
            return Err(CyclesCommandError::IcpRefillRejected {
                code,
                diagnostic: canic_host::diagnostics::render_diagnostic(code),
            });
        }
    };
    verify_operation_id(expected_operation_id, response.operation_id)?;
    Ok(DecodedIcpRefillResponse { response })
}

fn verify_operation_id(
    expected_operation_id: [u8; 32],
    actual_operation_id: [u8; 32],
) -> Result<(), CyclesCommandError> {
    if actual_operation_id != expected_operation_id {
        return Err(CyclesCommandError::IcpRefillOperationIdMismatch {
            expected: hex_bytes(expected_operation_id),
            actual: hex_bytes(actual_operation_id),
        });
    }
    Ok(())
}

fn optional_display<T: ToString>(value: Option<&T>) -> String {
    value.map_or_else(|| "none".to_string(), ToString::to_string)
}

fn optional_debug<T: std::fmt::Debug>(value: Option<T>) -> String {
    value.map_or_else(|| "none".to_string(), |value| format!("{value:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use canic_contracts::{
        diagnostics::codes,
        dto::{
            icp_refill::{IcpRefillErrorCode, IcpRefillStatus},
            role::OperationReceipt,
        },
    };
    use ic_platform::candid::contract::Nat;

    #[test]
    fn terminal_refill_response_is_typed_and_operation_bound() {
        let output = native_status_response(Ok(sample_response(
            [7; 32],
            IcpRefillStatus::Completed,
            Some(42),
            None,
        )));
        let response =
            validate_icp_refill_status_response(output, [7; 32]).expect("decode response");

        assert!(!response.is_resumable());
        assert!(response.render(false).contains("status=Completed"));
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&response.render(true))
                .expect("parse JSON response"),
            serde_json::json!({
                "operation_id": hex_bytes([7; 32]),
                "status": "Completed",
                "ledger_block_index": 42,
                "cycles_sent": "1000",
                "error_code": null,
                "error_message": null,
            })
        );
    }

    #[test]
    fn transferred_refill_response_remains_resumable() {
        let output = native_status_response(Ok(sample_response(
            [8; 32],
            IcpRefillStatus::Transferred,
            Some(42),
            None,
        )));
        let response =
            validate_icp_refill_status_response(output, [8; 32]).expect("decode response");

        assert!(response.is_resumable());
    }

    #[test]
    fn endpoint_error_preserves_typed_code() {
        let response = Err(Error::from_registered(
            canic_contracts::diagnostics::codes::STATE_CONFLICT,
        ));
        let error =
            validate_icp_refill_command_response(response, [7; 32]).expect_err("reject response");

        std::assert_matches!(
            error,
            CyclesCommandError::IcpRefillRejected {
                code,
                diagnostic,
            } if code == codes::STATE_CONFLICT.raw_code()
                && diagnostic == canic_host::diagnostics::render_diagnostic(code)
        );
    }

    #[test]
    fn accepted_receipt_is_operation_bound() {
        let response = Ok(OperationReceipt {
            operation_id: [7; 32],
        });

        validate_icp_refill_command_response(response, [7; 32]).expect("decode receipt");
    }

    #[test]
    fn mismatched_command_operation_id_is_rejected() {
        let error = validate_icp_refill_command_response(
            Ok(OperationReceipt {
                operation_id: [8; 32],
            }),
            [7; 32],
        )
        .unwrap_err();
        assert!(matches!(
            error,
            CyclesCommandError::IcpRefillOperationIdMismatch { .. }
        ));
    }

    #[test]
    fn mismatched_status_operation_id_is_rejected() {
        let output = native_status_response(Ok(sample_response(
            [8; 32],
            IcpRefillStatus::Completed,
            Some(42),
            None,
        )));
        let error =
            validate_icp_refill_status_response(output, [7; 32]).expect_err("reject mismatch");

        assert!(matches!(
            error,
            CyclesCommandError::IcpRefillOperationIdMismatch { .. }
        ));
    }

    #[test]
    fn native_refill_preserves_full_width_cycles_and_remote_refusals() {
        let mut response = sample_response([9; 32], IcpRefillStatus::Completed, Some(42), None);
        let cycles = Nat::from(1_u128 << 100);
        response.cycles_sent = Some(cycles.clone());
        let decoded =
            validate_icp_refill_status_response(native_status_response(Ok(response)), [9; 32])
                .unwrap();
        let json: serde_json::Value = serde_json::from_str(&decoded.render(true)).unwrap();
        assert_eq!(json["cycles_sent"], cycles.0.to_string());
        let error = validate_icp_refill_status_response(
            Err(Error::from_registered(codes::STATE_UNAVAILABLE)),
            [9; 32],
        )
        .unwrap_err();
        assert!(
            matches!(error, CyclesCommandError::IcpRefillRejected { code, .. } if code == codes::STATE_UNAVAILABLE.raw_code())
        );
    }

    fn native_status_response(
        response: Result<IcpRefillResponse, Error>,
    ) -> Result<RefillResult, Error> {
        response.map(|value| value.try_into().expect("bounded native refill evidence"))
    }

    fn sample_response(
        operation_id: [u8; 32],
        status: IcpRefillStatus,
        ledger_block_index: Option<u64>,
        error_code: Option<IcpRefillErrorCode>,
    ) -> IcpRefillResponse {
        IcpRefillResponse {
            operation_id,
            status,
            ledger_block_index,
            cycles_sent: Some(Nat::from(1_000_u64)),
            error_code,
            error_message: None,
        }
    }
}
