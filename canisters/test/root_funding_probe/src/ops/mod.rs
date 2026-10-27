//! Project external transfer evidence at the native observation boundary.
impl From<crate::EmergencyRefillOperation> for crate::protocol::EmergencyOperation {
    fn from(value: crate::EmergencyRefillOperation) -> Self {
        let transfer = value.transfer;
        Self {
            transfer: crate::protocol::EmergencyTransfer {
                operation_id: transfer.operation_id,
                from_subaccount: transfer.from_subaccount,
                to_owner: transfer.to_owner.into(),
                to_subaccount: transfer.to_subaccount,
                amount_e8s: transfer.amount_e8s,
                fee_e8s: transfer.fee_e8s,
                memo: transfer.memo,
                created_at_time_ns: transfer.created_at_time_ns,
            },
            max_call_cost: value.max_call_cost,
            phase: value.phase,
        }
    }
}
