//! Module: fleet_ensure::ops::protocol
//!
//! Responsibility: execute one exact declarative Candid transition and its terminal query.
//! Does not own: sequencing, retry policy, desired-state validation, or durable intent.
//! Boundary: all artifacts and dynamic bindings were hashed into the reviewed action.

use crate::{
    fleet_ensure::model::EnsureAction,
    icp::{IcpCli, IcpCommandError, IcpJsonResponseError},
};
use candid_parser::{
    parse_idl_args,
    utils::{CandidSource, instantiate_candid},
};
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;
use std::{
    collections::BTreeMap,
    fmt::Write as _,
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};
use thiserror::Error as ThisError;

const MAX_PROTOCOL_ARTIFACT_BYTES: u64 = 1_048_576;
const MAX_PROTOCOL_ARGUMENT_BYTES: usize = 16 * 1_024;
const MAX_TEMP_ATTEMPTS: usize = 32;
static NEXT_TEMP_FILE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, ThisError)]
pub enum ProtocolEffectError {
    #[error("Fleet protocol action is not a protocol transition")]
    WrongAction,

    #[error("Fleet protocol artifact is unavailable or unsafe: {}", .0.display())]
    ArtifactUnavailable(PathBuf),

    #[error("Fleet protocol artifact {kind} changed: expected {expected}, observed {actual}")]
    ArtifactChanged {
        actual: String,
        expected: String,
        kind: &'static str,
    },

    #[error("Fleet protocol Candid contract is invalid: {0}")]
    CandidContract(String),

    #[error("Fleet protocol {kind} arguments are invalid for method {method}: {detail}")]
    CandidArguments {
        kind: &'static str,
        method: String,
        detail: String,
    },

    #[error("Fleet protocol template contains unresolved authority binding: {0}")]
    UnresolvedBinding(String),

    #[error("Fleet protocol operation identity is not an exact 32-byte hexadecimal value")]
    InvalidOperationIdentity,

    #[error("Fleet protocol argument exceeded {maximum} bytes: {actual}")]
    ArgumentTooLarge { actual: usize, maximum: usize },

    #[error("failed to manage Fleet protocol argument file: {0}")]
    ArgumentFile(#[source] io::Error),

    #[error(transparent)]
    Icp(#[from] IcpCommandError),

    #[error(transparent)]
    Response(#[from] IcpJsonResponseError),
}

pub(super) struct ProtocolObservation {
    pub applied: bool,
    pub progress_identity: String,
}

pub(super) fn observe(
    icp: &IcpCli,
    root: &Path,
    operation_id: &str,
    principals: &BTreeMap<String, String>,
    action: &EnsureAction,
) -> Result<ProtocolObservation, ProtocolEffectError> {
    let ProtocolAction {
        candid,
        candid_sha256,
        expected_status,
        expected_status_sha256,
        principal,
        status_args,
        status_args_sha256,
        status_method,
        ..
    } = ProtocolAction::from_action(action)?;
    let candid_path = verified_path(root, candid, candid_sha256, "Candid")?;
    let status_source = verified_text(root, status_args, status_args_sha256, "status arguments")?;
    let expected_source = verified_text(
        root,
        expected_status,
        expected_status_sha256,
        "expected status",
    )?;
    let status_source = render(&status_source, operation_id, principals)?;
    let expected_source = render(&expected_source, operation_id, principals)?;
    let candid_source = read_bounded(&candid_path)?;
    let candid_source = std::str::from_utf8(&candid_source)
        .map_err(|error| ProtocolEffectError::CandidContract(error.to_string()))?;
    let (status_input, expected_output) = encode_call(
        candid_source,
        status_method,
        &status_source,
        Some(&expected_source),
    )?;
    let actual = invoke(
        icp,
        principal,
        status_method,
        &status_input,
        &candid_path,
        true,
    )?;
    Ok(ProtocolObservation {
        applied: actual == expected_output.expect("expected status was encoded"),
        progress_identity: canic_core::cdk::utils::hash::sha256_hex(&actual),
    })
}

pub(super) fn apply(
    icp: &IcpCli,
    root: &Path,
    operation_id: &str,
    principals: &BTreeMap<String, String>,
    action: &EnsureAction,
) -> Result<(), ProtocolEffectError> {
    let ProtocolAction {
        candid,
        candid_sha256,
        command_args,
        command_args_sha256,
        command_method,
        principal,
        ..
    } = ProtocolAction::from_action(action)?;
    let candid_path = verified_path(root, candid, candid_sha256, "Candid")?;
    let command_source =
        verified_text(root, command_args, command_args_sha256, "command arguments")?;
    let command_source = render(&command_source, operation_id, principals)?;
    let candid_source = read_bounded(&candid_path)?;
    let candid_source = std::str::from_utf8(&candid_source)
        .map_err(|error| ProtocolEffectError::CandidContract(error.to_string()))?;
    let (command_input, _) = encode_call(candid_source, command_method, &command_source, None)?;
    let _ = invoke(
        icp,
        principal,
        command_method,
        &command_input,
        &candid_path,
        false,
    )?;
    Ok(())
}

pub(super) fn write_init_arguments(
    root: &Path,
    operation_id: &str,
    principals: &BTreeMap<String, String>,
    candid: &str,
    candid_sha256: &str,
    arguments: &str,
    arguments_sha256: &str,
) -> Result<PathBuf, ProtocolEffectError> {
    let candid_path = verified_path(root, candid, candid_sha256, "init Candid")?;
    let argument_source = verified_text(root, arguments, arguments_sha256, "init arguments")?;
    let argument_source = render(&argument_source, operation_id, principals)?;
    let candid_source = read_bounded(&candid_path)?;
    let candid_source = std::str::from_utf8(&candid_source)
        .map_err(|error| ProtocolEffectError::CandidContract(error.to_string()))?;
    let (types, (environment, _)) = instantiate_candid(CandidSource::Text(candid_source))
        .map_err(|error| ProtocolEffectError::CandidContract(error.to_string()))?;
    let arguments = parse_idl_args(&argument_source)
        .map_err(|error| ProtocolEffectError::CandidArguments {
            kind: "init",
            method: "<init>".to_string(),
            detail: error.to_string(),
        })?
        .to_bytes_with_types(&environment, &types)
        .map_err(|error| ProtocolEffectError::CandidArguments {
            kind: "init",
            method: "<init>".to_string(),
            detail: error.to_string(),
        })?;
    write_argument_file(&arguments)
}

struct ProtocolAction<'a> {
    candid: &'a str,
    candid_sha256: &'a str,
    command_args: &'a str,
    command_args_sha256: &'a str,
    command_method: &'a str,
    expected_status: &'a str,
    expected_status_sha256: &'a str,
    principal: &'a str,
    status_args: &'a str,
    status_args_sha256: &'a str,
    status_method: &'a str,
}

impl<'a> ProtocolAction<'a> {
    fn from_action(action: &'a EnsureAction) -> Result<Self, ProtocolEffectError> {
        let EnsureAction::Protocol {
            candid,
            candid_sha256,
            command_args,
            command_args_sha256,
            command_method,
            expected_status,
            expected_status_sha256,
            principal,
            status_args,
            status_args_sha256,
            status_method,
            ..
        } = action
        else {
            return Err(ProtocolEffectError::WrongAction);
        };
        Ok(Self {
            candid,
            candid_sha256,
            command_args,
            command_args_sha256,
            command_method,
            expected_status,
            expected_status_sha256,
            principal,
            status_args,
            status_args_sha256,
            status_method,
        })
    }
}

fn encode_call(
    candid_source: &str,
    method: &str,
    input_source: &str,
    expected_output_source: Option<&str>,
) -> Result<(Vec<u8>, Option<Vec<u8>>), ProtocolEffectError> {
    let (environment, actor) = CandidSource::Text(candid_source)
        .load()
        .map_err(|error| ProtocolEffectError::CandidContract(error.to_string()))?;
    let actor = actor.ok_or_else(|| {
        ProtocolEffectError::CandidContract("missing service declaration".to_string())
    })?;
    let service = environment
        .as_service(&actor)
        .map_err(|error| ProtocolEffectError::CandidContract(error.to_string()))?;
    let (_, method_type) = service
        .iter()
        .find(|(name, _)| name == method)
        .ok_or_else(|| {
            ProtocolEffectError::CandidContract(format!("method {method} is not declared"))
        })?;
    let function = environment
        .as_func(method_type)
        .map_err(|error| ProtocolEffectError::CandidContract(error.to_string()))?;
    let input =
        parse_idl_args(input_source).map_err(|error| ProtocolEffectError::CandidArguments {
            kind: "command/status",
            method: method.to_string(),
            detail: error.to_string(),
        })?;
    let input = input
        .to_bytes_with_types(&environment, &function.args)
        .map_err(|error| ProtocolEffectError::CandidArguments {
            kind: "command/status",
            method: method.to_string(),
            detail: error.to_string(),
        })?;
    require_argument_bound(&input)?;
    let expected_output = expected_output_source
        .map(|source| {
            parse_idl_args(source)
                .map_err(|error| ProtocolEffectError::CandidArguments {
                    kind: "expected status",
                    method: method.to_string(),
                    detail: error.to_string(),
                })?
                .to_bytes_with_types(&environment, &function.rets)
                .map_err(|error| ProtocolEffectError::CandidArguments {
                    kind: "expected status",
                    method: method.to_string(),
                    detail: error.to_string(),
                })
        })
        .transpose()?;
    Ok((input, expected_output))
}

fn render(
    source: &str,
    operation_id: &str,
    principals: &BTreeMap<String, String>,
) -> Result<String, ProtocolEffectError> {
    let operation_bytes = canic_core::cdk::utils::hash::decode_hex(operation_id)
        .map_err(|_| ProtocolEffectError::InvalidOperationIdentity)?;
    if operation_bytes.len() != 32 {
        return Err(ProtocolEffectError::InvalidOperationIdentity);
    }
    let mut operation_blob = String::with_capacity(operation_bytes.len() * 3);
    for byte in operation_bytes {
        write!(&mut operation_blob, "\\{byte:02x}")
            .map_err(|_| ProtocolEffectError::InvalidOperationIdentity)?;
    }
    let mut rendered = source
        .replace("{{operation_id}}", &format!("\"{operation_id}\""))
        .replace(
            "{{operation_id_blob}}",
            &format!("blob \"{operation_blob}\""),
        );
    for (name, principal) in principals {
        rendered = rendered.replace(
            &format!("{{{{principal:{name}}}}}"),
            &format!("principal \"{principal}\""),
        );
    }
    if let Some(start) = rendered.find("{{") {
        let end = rendered[start..]
            .find("}}")
            .map_or(rendered.len(), |offset| start + offset + 2);
        return Err(ProtocolEffectError::UnresolvedBinding(
            rendered[start..end].to_string(),
        ));
    }
    Ok(rendered)
}

fn invoke(
    icp: &IcpCli,
    principal: &str,
    method: &str,
    arguments: &[u8],
    candid: &Path,
    query: bool,
) -> Result<Vec<u8>, ProtocolEffectError> {
    let path = write_argument_file(arguments)?;
    let output = if query {
        icp.canister_query_binary_args_output_with_candid(
            principal,
            method,
            &path,
            Some("json"),
            Some(candid),
        )
    } else {
        icp.canister_call_binary_args_output_with_candid(
            principal,
            method,
            &path,
            Some("json"),
            Some(candid),
        )
    };
    let cleanup = fs::remove_file(&path);
    let output = output?;
    cleanup.map_err(ProtocolEffectError::ArgumentFile)?;
    crate::icp::response_bytes(&output).map_err(Into::into)
}

fn verified_text(
    root: &Path,
    configured: &str,
    expected: &str,
    kind: &'static str,
) -> Result<String, ProtocolEffectError> {
    let path = verified_path(root, configured, expected, kind)?;
    let bytes = read_bounded(&path)?;
    String::from_utf8(bytes).map_err(|_| ProtocolEffectError::ArtifactUnavailable(path))
}

fn verified_path(
    root: &Path,
    configured: &str,
    expected: &str,
    kind: &'static str,
) -> Result<PathBuf, ProtocolEffectError> {
    let configured = Path::new(configured);
    let path = if configured.is_absolute() {
        configured.to_path_buf()
    } else {
        root.join(configured)
    };
    let bytes = read_bounded(&path)?;
    let actual = canic_core::cdk::utils::hash::sha256_hex(&bytes);
    if actual != expected {
        return Err(ProtocolEffectError::ArtifactChanged {
            actual,
            expected: expected.to_string(),
            kind,
        });
    }
    Ok(path)
}

fn read_bounded(path: &Path) -> Result<Vec<u8>, ProtocolEffectError> {
    let metadata = fs::metadata(path)
        .map_err(|_| ProtocolEffectError::ArtifactUnavailable(path.to_path_buf()))?;
    if !metadata.is_file() || metadata.len() > MAX_PROTOCOL_ARTIFACT_BYTES {
        return Err(ProtocolEffectError::ArtifactUnavailable(path.to_path_buf()));
    }
    fs::read(path).map_err(|_| ProtocolEffectError::ArtifactUnavailable(path.to_path_buf()))
}

const fn require_argument_bound(bytes: &[u8]) -> Result<(), ProtocolEffectError> {
    if bytes.len() > MAX_PROTOCOL_ARGUMENT_BYTES {
        return Err(ProtocolEffectError::ArgumentTooLarge {
            actual: bytes.len(),
            maximum: MAX_PROTOCOL_ARGUMENT_BYTES,
        });
    }
    Ok(())
}

pub(super) fn write_argument_file(bytes: &[u8]) -> Result<PathBuf, ProtocolEffectError> {
    require_argument_bound(bytes)?;
    for _ in 0..MAX_TEMP_ATTEMPTS {
        let sequence = NEXT_TEMP_FILE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "canic-fleet-ensure-protocol-{}-{sequence}.bin",
            std::process::id()
        ));
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        options.mode(0o600);
        match options.open(&path) {
            Ok(mut file) => {
                file.write_all(bytes)
                    .and_then(|()| file.sync_all())
                    .map_err(ProtocolEffectError::ArgumentFile)?;
                return Ok(path);
            }
            Err(source) if source.kind() == io::ErrorKind::AlreadyExists => {}
            Err(source) => return Err(ProtocolEffectError::ArgumentFile(source)),
        }
    }
    Err(ProtocolEffectError::ArgumentFile(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "could not allocate a unique protocol argument file",
    )))
}

#[cfg(test)]
mod tests {
    use super::{encode_call, render, write_init_arguments};
    use candid::Principal;
    use canic_core::cdk::utils::hash::sha256_hex;
    use std::collections::BTreeMap;
    use std::fs;

    #[test]
    fn template_binding_is_exact_and_rejects_unknown_authority() {
        let operation = "11".repeat(32);
        let principals = BTreeMap::from([("root".to_string(), "aaaaa-aa".to_string())]);
        let rendered = render(
            "({{principal:root}}, {{operation_id}}, {{operation_id_blob}})",
            &operation,
            &principals,
        )
        .expect("render exact bindings");
        assert!(rendered.contains("principal \"aaaaa-aa\""));
        assert!(rendered.contains(&format!("\"{operation}\"")));
        assert!(rendered.contains("blob \"\\11\\11"));
        assert!(render("({{principal:missing}})", &operation, &principals).is_err());
    }

    #[test]
    fn protocol_arguments_are_typed_by_the_exact_candid_method() {
        let candid = "service : { apply : (text) -> (); status : (text) -> (bool) query; };";
        let (status, expected) = encode_call(candid, "status", "(\"operation\")", Some("(true)"))
            .expect("encode exact status contract");
        assert_eq!(
            candid::decode_one::<String>(&status).expect("decode status input"),
            "operation"
        );
        assert!(
            candid::decode_one::<bool>(&expected.expect("expected response"))
                .expect("decode expected status")
        );
    }

    #[test]
    fn init_template_binds_created_principal_and_operation_identity() {
        let root = crate::test_support::temp_dir("canic-fleet-ensure-init-template");
        fs::create_dir_all(&root).expect("create init-template fixture");
        let candid_path = root.join("role.did");
        let arguments_path = root.join("role.args");
        let candid = b"service : (principal, blob) -> {};";
        let arguments = b"({{principal:root}}, {{operation_id_blob}})";
        fs::write(&candid_path, candid).expect("write init Candid");
        fs::write(&arguments_path, arguments).expect("write init arguments");
        let operation = "22".repeat(32);
        let principals = BTreeMap::from([("root".to_string(), "aaaaa-aa".to_string())]);
        let encoded_path = write_init_arguments(
            &root,
            &operation,
            &principals,
            candid_path.to_str().expect("Candid path UTF-8"),
            &sha256_hex(candid),
            arguments_path.to_str().expect("argument path UTF-8"),
            &sha256_hex(arguments),
        )
        .expect("render typed init arguments");
        let encoded = fs::read(&encoded_path).expect("read encoded init arguments");
        let (principal, operation_bytes) =
            candid::decode_args::<(Principal, Vec<u8>)>(&encoded).expect("decode init arguments");
        assert_eq!(principal, Principal::management_canister());
        assert_eq!(operation_bytes, vec![0x22; 32]);
        fs::remove_file(encoded_path).expect("remove encoded init arguments");
    }
}
