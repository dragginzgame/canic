use std::fmt::Write as _;

pub(super) fn icp_refill_request_arg(
    operation_id: [u8; 32],
    source_subaccount: Option<[u8; 32]>,
    amount_e8s: u64,
    dry_run: bool,
) -> String {
    format!(
        "(record {{ operation_id = {}; source_subaccount = {}; amount_e8s = {} : nat64; \
         dry_run = {} }})",
        idl_blob(&operation_id),
        optional_idl_blob(source_subaccount),
        amount_e8s,
        dry_run,
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
    fn renders_icp_refill_request_arg() {
        let arg = icp_refill_request_arg([1; 32], Some([2; 32]), 100_000_000, true);

        assert!(arg.contains(r#"operation_id = blob "\01\01\01"#));
        assert!(arg.contains(r#"source_subaccount = opt blob "\02\02\02"#));
        assert!(arg.contains("amount_e8s = 100000000 : nat64"));
        assert!(arg.contains("dry_run = true"));
    }
}
