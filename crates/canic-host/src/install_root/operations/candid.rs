//! Module: install_root::operations::candid
//!
//! Responsibility: invoke typed textual-Candid canister operations during installation.
//! Does not own: binary argument files, domain-specific methods, or workflow transitions.
//! Boundary: install workflows supply exact canister, method, and typed arguments.

use crate::icp::{IcpCli, decode_json_result_response};
use candid::{CandidType, IDLValue, Principal};

const ICP_JSON_OUTPUT: &str = "json";

pub(in crate::install_root) fn call_no_arg<O>(
    icp: &IcpCli,
    canister: Principal,
    method: &str,
) -> Result<O, Box<dyn std::error::Error>>
where
    O: CandidType + serde::de::DeserializeOwned,
{
    let output = icp.canister_call_arg_output_with_candid(
        &canister.to_text(),
        method,
        "()",
        Some(ICP_JSON_OUTPUT),
        None,
    )?;
    decode_json_result_response(&output).map_err(Into::into)
}

pub(in crate::install_root) fn call_with_arg<I, O>(
    icp: &IcpCli,
    canister: Principal,
    method: &str,
    input: &I,
    query: bool,
) -> Result<O, Box<dyn std::error::Error>>
where
    I: CandidType,
    O: CandidType + serde::de::DeserializeOwned,
{
    let value = IDLValue::try_from_candid_type(input)?;
    let args = format!("({value})");
    let output = if query {
        icp.canister_query_arg_output_with_candid(
            &canister.to_text(),
            method,
            &args,
            Some(ICP_JSON_OUTPUT),
            None,
        )?
    } else {
        icp.canister_call_arg_output_with_candid(
            &canister.to_text(),
            method,
            &args,
            Some(ICP_JSON_OUTPUT),
            None,
        )?
    };
    decode_json_result_response(&output).map_err(Into::into)
}

pub(in crate::install_root) fn query_no_arg<O>(
    icp: &IcpCli,
    canister: Principal,
    method: &str,
) -> Result<O, Box<dyn std::error::Error>>
where
    O: CandidType + serde::de::DeserializeOwned,
{
    let output = icp.canister_query_output_with_candid(
        &canister.to_text(),
        method,
        Some(ICP_JSON_OUTPUT),
        None,
    )?;
    decode_json_result_response(&output).map_err(Into::into)
}
