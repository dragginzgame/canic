use std::fmt::Write as _;

pub(super) fn root_refill_command_arg(
    operation_id: [u8; 32],
    source_subaccount: Option<[u8; 32]>,
    amount_e8s: u64,
) -> String {
    format!(
        "(variant {{ RefillCycles = record {{ operation_id = {}; source_subaccount = {}; \
         amount_e8s = {} : nat64 }} }})",
        idl_blob(&operation_id),
        optional_idl_blob(source_subaccount),
        amount_e8s,
    )
}

pub(super) fn root_refill_status_arg(operation_id: [u8; 32]) -> String {
    format!(
        "(variant {{ Operation = record {{ operation_id = {} }} }})",
        idl_blob(&operation_id),
    )
}

fn optional_idl_blob(bytes: Option<[u8; 32]>) -> String {
    bytes.map_or_else(
        || "null".to_string(),
        |bytes| format!("opt {}", idl_blob(&bytes)),
    )
}

fn idl_blob(bytes: &[u8]) -> String {
    let mut encoded = String::from("blob \"");
    for byte in bytes {
        let _ = write!(encoded, "\\{byte:02X}");
    }
    encoded.push('"');
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_root_refill_command_arg() {
        let arg = root_refill_command_arg([1; 32], Some([2; 32]), 100_000_000);

        assert!(arg.starts_with("(variant { RefillCycles = record {"));
        assert!(arg.contains(r#"operation_id = blob "\01\01\01"#));
        assert!(arg.contains(r#"source_subaccount = opt blob "\02\02\02"#));
        assert!(arg.contains("amount_e8s = 100000000 : nat64"));
    }

    #[test]
    fn renders_operation_status_arg() {
        let arg = root_refill_status_arg([3; 32]);

        assert!(arg.starts_with("(variant { Operation = record {"));
        assert!(arg.contains(r#"operation_id = blob "\03\03\03"#));
    }
}
