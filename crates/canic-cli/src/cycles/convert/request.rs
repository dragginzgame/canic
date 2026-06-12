use std::fmt::Write as _;

pub(super) const FABRICATE_MODE_MESSAGE: &str =
    "mode=fabricate (does not call canister refill endpoint)";

pub(super) const fn json_output_arg(json: bool) -> Option<&'static str> {
    if json { Some("json") } else { None }
}

pub(super) fn icp_refill_request_arg(
    operation_id: [u8; 32],
    source_canister: &str,
    source_subaccount: Option<[u8; 32]>,
    target_canister: &str,
    amount_e8s: u64,
    dry_run: bool,
) -> String {
    format!(
        "(record {{ operation_id = {}; source_canister = principal \"{}\"; source_subaccount = {}; \
         target_canister = principal \"{}\"; amount_e8s = {} : nat64; dry_run = {}; \
         mode = variant {{ Canister }} }})",
        idl_blob(&operation_id),
        source_canister,
        optional_idl_blob(source_subaccount),
        target_canister,
        amount_e8s,
        dry_run,
    )
}

pub(super) fn provisional_top_up_arg(canister_id: &str, amount_cycles: u128) -> String {
    format!(
        "(record {{ canister_id = principal \"{canister_id}\"; amount = {amount_cycles} : nat }})"
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
        let arg = icp_refill_request_arg(
            [1; 32],
            "source-principal",
            Some([2; 32]),
            "target-principal",
            100_000_000,
            true,
        );

        assert!(arg.contains(r#"operation_id = blob "\01\01\01"#));
        assert!(arg.contains(r#"source_canister = principal "source-principal""#));
        assert!(arg.contains(r#"source_subaccount = opt blob "\02\02\02"#));
        assert!(arg.contains(r#"target_canister = principal "target-principal""#));
        assert!(arg.contains("amount_e8s = 100000000 : nat64"));
        assert!(arg.contains("dry_run = true"));
        assert!(arg.contains("mode = variant { Canister }"));
    }

    #[test]
    fn renders_fabrication_arg_and_message() {
        assert_eq!(
            provisional_top_up_arg("target-principal", 4_000_000_000_000),
            r#"(record { canister_id = principal "target-principal"; amount = 4000000000000 : nat })"#
        );
        assert_eq!(
            FABRICATE_MODE_MESSAGE,
            "mode=fabricate (does not call canister refill endpoint)"
        );
    }
}
