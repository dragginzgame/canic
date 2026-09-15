//! Module: canister_protocol
//!
//! Responsibility: invoke typed Canic Candid methods through the maintained ICP CLI adapter.
//! Does not own: domain sequencing, endpoint authorization, or management-Canister effects.
//! Boundary: domain workflows supply exact Canister, method, and arguments through explicit
//! query or update operations.

pub mod inspection;

use crate::{
    icp::{IcpCli, IcpCommandError, IcpJsonResponseError, decode_json_result_response},
    protocol_binding::ResolvedProtocolBinding,
};
use candid::{CandidType, Principal};
use canic_core::diagnostics::RegisteredDiagnosticCode;
use serde::de::DeserializeOwned;
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;
use std::{
    fs::{self, OpenOptions},
    io::{self, Write},
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
use thiserror::Error as ThisError;

const ICP_JSON_OUTPUT: &str = "json";
const MAX_ARGUMENT_FILE_ATTEMPTS: usize = 32;
static NEXT_ARGUMENT_FILE: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy)]
enum ProtocolCallMode {
    Query,
    Update,
}

#[derive(Debug, ThisError)]
pub enum CanisterProtocolError {
    #[error("invalid bound inspection contract for Root {caller}: {detail}")]
    InspectionContract { caller: Principal, detail: String },

    #[error("Root {} inspection preflight for {} observed native={} cycles, liquid={} cycles, required outbound reserve={} cycles. Inspection was not attempted. Review Root funding or freezing reserve and retry with a fresh observation.", .0.caller, .0.canister_id, .0.native_cycles, .0.available_liquid_cycles, .0.required_liquid_cycles)]
    InspectionPreflightReserve(Box<canic_core::dto::canister::CanisterInspectionReserveResponse>),

    #[error("{}: Root {} cannot inspect {}: native={} cycles, liquid={} cycles, required outbound reserve={} cycles. Review Root funding or freezing reserve before retrying.", canic_core::diagnostics::codes::PLATFORM_INSUFFICIENT_LIQUID_CYCLES, .0.caller, .0.canister_id, .0.native_cycles, .0.available_liquid_cycles, .0.required_liquid_cycles)]
    InspectionReserve(Box<canic_core::dto::canister::CanisterInspectionReserveResponse>),

    #[error("Root {caller} returned invalid reserve evidence for inspection of {target}")]
    InvalidInspectionReserve {
        caller: Principal,
        target: Principal,
    },

    #[error("failed to encode Candid arguments for {method} on Canister {canister}: {source}")]
    ArgumentEncoding {
        canister: Principal,
        method: &'static str,
        #[source]
        source: candid::Error,
    },

    #[error(
        "failed to manage binary Candid arguments for {method} on Canister {canister}: {source}"
    )]
    ArgumentFile {
        canister: Principal,
        method: &'static str,
        #[source]
        source: io::Error,
    },

    #[error("failed to invoke {method} on Canister {canister}: {source}")]
    Invocation {
        canister: Principal,
        method: &'static str,
        #[source]
        source: IcpCommandError,
    },

    #[error("invalid {method} response from Canister {canister}: {source}")]
    Response {
        canister: Principal,
        method: &'static str,
        #[source]
        source: IcpJsonResponseError,
    },
}

impl CanisterProtocolError {
    pub(crate) fn inspection_reserve(
        caller: Principal,
        target: Principal,
        evidence: canic_core::dto::canister::CanisterInspectionReserveResponse,
    ) -> Self {
        if evidence.caller != caller
            || evidence.canister_id != target
            || evidence.available_liquid_cycles >= evidence.required_liquid_cycles
            || evidence.available_liquid_cycles > evidence.native_cycles
        {
            return Self::InvalidInspectionReserve { caller, target };
        }
        Self::InspectionReserve(Box::new(evidence))
    }

    pub(crate) fn is_rejected_with(&self, code: RegisteredDiagnosticCode) -> bool {
        if matches!(self, Self::InspectionReserve(_)) {
            return code == canic_core::diagnostics::codes::PLATFORM_INSUFFICIENT_LIQUID_CYCLES;
        }
        matches!(
            self,
            Self::Response {
                source: IcpJsonResponseError::Rejected(error),
                ..
            } if error.code() == code.raw_code()
        )
    }
}

pub fn call_with_arg<I, O>(
    icp: &IcpCli,
    binding: &ResolvedProtocolBinding,
    canister: Principal,
    method: &'static str,
    input: &I,
) -> Result<O, CanisterProtocolError>
where
    I: CandidType,
    O: CandidType + DeserializeOwned,
{
    invoke_with_candid(
        icp,
        &binding.candid_path,
        canister,
        method,
        input,
        ProtocolCallMode::Update,
    )
}

pub fn query_with_arg<I, O>(
    icp: &IcpCli,
    binding: &ResolvedProtocolBinding,
    canister: Principal,
    method: &'static str,
    input: &I,
) -> Result<O, CanisterProtocolError>
where
    I: CandidType,
    O: CandidType + DeserializeOwned,
{
    invoke_with_candid(
        icp,
        &binding.candid_path,
        canister,
        method,
        input,
        ProtocolCallMode::Query,
    )
}

/// Invoke one typed current-generation call after its exact Candid bytes were
/// independently bound into the reviewed Fleet action.
pub fn call_with_candid<I, O>(
    icp: &IcpCli,
    candid_path: &std::path::Path,
    canister: Principal,
    method: &'static str,
    input: &I,
) -> Result<O, CanisterProtocolError>
where
    I: CandidType,
    O: CandidType + DeserializeOwned,
{
    invoke_with_candid(
        icp,
        candid_path,
        canister,
        method,
        input,
        ProtocolCallMode::Update,
    )
}

/// Query one typed current-generation status after its exact Candid bytes were
/// independently bound into the reviewed Fleet action.
pub fn query_with_candid<I, O>(
    icp: &IcpCli,
    candid_path: &std::path::Path,
    canister: Principal,
    method: &'static str,
    input: &I,
) -> Result<O, CanisterProtocolError>
where
    I: CandidType,
    O: CandidType + DeserializeOwned,
{
    invoke_with_candid(
        icp,
        candid_path,
        canister,
        method,
        input,
        ProtocolCallMode::Query,
    )
}

fn invoke_with_candid<I, O>(
    icp: &IcpCli,
    candid_path: &std::path::Path,
    canister: Principal,
    method: &'static str,
    input: &I,
    mode: ProtocolCallMode,
) -> Result<O, CanisterProtocolError>
where
    I: CandidType,
    O: CandidType + DeserializeOwned,
{
    let output = invoke_output(icp, Some(candid_path), canister, method, input, mode)?;
    decode_response(canister, method, &output)
}

fn invoke_output<I>(
    icp: &IcpCli,
    candid_path: Option<&std::path::Path>,
    canister: Principal,
    method: &'static str,
    input: &I,
    mode: ProtocolCallMode,
) -> Result<String, CanisterProtocolError>
where
    I: CandidType,
{
    let bytes =
        candid::encode_one(input).map_err(|source| CanisterProtocolError::ArgumentEncoding {
            canister,
            method,
            source,
        })?;
    let args_path =
        write_argument_file(&bytes).map_err(|source| CanisterProtocolError::ArgumentFile {
            canister,
            method,
            source,
        })?;
    let canister_text = canister.to_text();
    let output = match mode {
        ProtocolCallMode::Query => icp.canister_query_binary_args_output_with_candid(
            &canister_text,
            method,
            &args_path,
            Some(ICP_JSON_OUTPUT),
            candid_path,
        ),
        ProtocolCallMode::Update => icp.canister_call_binary_args_output_with_candid(
            &canister_text,
            method,
            &args_path,
            Some(ICP_JSON_OUTPUT),
            candid_path,
        ),
    };
    let cleanup = fs::remove_file(&args_path);
    let output = output.map_err(|source| CanisterProtocolError::Invocation {
        canister,
        method,
        source,
    })?;
    cleanup.map_err(|source| CanisterProtocolError::ArgumentFile {
        canister,
        method,
        source,
    })?;
    Ok(output)
}

pub fn write_argument_file(bytes: &[u8]) -> io::Result<PathBuf> {
    let directory = std::env::temp_dir();
    for _ in 0..MAX_ARGUMENT_FILE_ATTEMPTS {
        let sequence = NEXT_ARGUMENT_FILE.fetch_add(1, Ordering::Relaxed);
        let path = directory.join(format!(
            "canic-candid-args-{}-{sequence}.bin",
            std::process::id()
        ));
        match create_argument_file(&path, bytes) {
            Ok(()) => return Ok(path),
            Err(source) if source.kind() == io::ErrorKind::AlreadyExists => {}
            Err(source) => return Err(source),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "could not allocate a unique Candid argument file",
    ))
}

fn create_argument_file(path: &std::path::Path, bytes: &[u8]) -> io::Result<()> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    options.mode(0o600);
    let mut file = options.open(path)?;
    if let Err(source) = file.write_all(bytes).and_then(|()| file.sync_all()) {
        drop(file);
        let _ = fs::remove_file(path);
        return Err(source);
    }
    Ok(())
}

fn decode_response<O>(
    canister: Principal,
    method: &'static str,
    output: &str,
) -> Result<O, CanisterProtocolError>
where
    O: CandidType + DeserializeOwned,
{
    decode_json_result_response(output).map_err(|source| CanisterProtocolError::Response {
        canister,
        method,
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(CandidType, Debug, Deserialize, Eq, PartialEq)]
    struct EmptyVectorArgument {
        values: Vec<u64>,
    }

    #[test]
    fn inspection_reserve_requires_exact_authority_and_retains_numeric_cause() {
        use canic_core::{diagnostics::codes, dto::canister::CanisterInspectionReserveResponse};
        let root = Principal::from_slice(&[1]);
        let target = Principal::from_slice(&[2]);
        let evidence = CanisterInspectionReserveResponse {
            caller: root,
            canister_id: target,
            native_cycles: 1000,
            available_liquid_cycles: 50,
            required_liquid_cycles: 100,
        };
        let error = CanisterProtocolError::inspection_reserve(root, target, evidence.clone());
        assert!(error.is_rejected_with(codes::PLATFORM_INSUFFICIENT_LIQUID_CYCLES));
        assert!(!error.is_rejected_with(codes::PLATFORM_UNAVAILABLE));
        let CanisterProtocolError::InspectionReserve(actual) = &error else {
            panic!("reserve evidence");
        };
        assert_eq!(**actual, evidence);
        assert!(error.to_string().contains(
            "native=1000 cycles, liquid=50 cycles, required outbound reserve=100 cycles"
        ));
        for invalid in [
            CanisterInspectionReserveResponse {
                caller: target,
                ..evidence.clone()
            },
            CanisterInspectionReserveResponse {
                canister_id: root,
                ..evidence.clone()
            },
            CanisterInspectionReserveResponse {
                available_liquid_cycles: 100,
                ..evidence.clone()
            },
            CanisterInspectionReserveResponse {
                native_cycles: 49,
                ..evidence
            },
        ] {
            let error = CanisterProtocolError::inspection_reserve(root, target, invalid);
            assert!(matches!(
                error,
                CanisterProtocolError::InvalidInspectionReserve { .. }
            ));
            assert!(!error.is_rejected_with(codes::PLATFORM_INSUFFICIENT_LIQUID_CYCLES));
        }
    }

    #[test]
    fn typed_arguments_are_written_to_private_binary_files() {
        let argument = EmptyVectorArgument { values: Vec::new() };
        let path = write_argument_file(&candid::encode_one(&argument).expect("encode argument"))
            .expect("write argument file");

        let decoded: EmptyVectorArgument =
            candid::decode_one(&fs::read(&path).expect("read argument file"))
                .expect("decode argument");
        assert_eq!(decoded, argument);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            assert_eq!(
                fs::metadata(&path)
                    .expect("argument metadata")
                    .permissions()
                    .mode()
                    & 0o777,
                0o600
            );
        }
        fs::remove_file(path).expect("remove argument file");
    }
}
