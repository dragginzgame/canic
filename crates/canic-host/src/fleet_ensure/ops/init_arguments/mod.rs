//! Module: fleet_ensure::ops::init_arguments
//!
//! Responsibility: encode lifecycle initialization arguments from exact reviewed artifacts.
//! Does not own: sequencing, retry policy, desired-state validation, or durable intent.
//! Boundary: all artifacts and dynamic bindings were hashed into the reviewed action.

use ic_platform::candid_parser::{CandidSource, instantiate_candid, parse_idl_args};
use std::{
    collections::BTreeMap,
    fmt::Write as _,
    fs, io,
    path::{Path, PathBuf},
};
use thiserror::Error as ThisError;

const MAX_INIT_ARTIFACT_BYTES: u64 = 1_048_576;
const MAX_INIT_ARGUMENT_BYTES: usize = 16 * 1_024;

#[derive(Debug, ThisError)]
pub enum InitArgumentError {
    #[error("Fleet initializer artifact is unavailable or unsafe: {}", .0.display())]
    ArtifactUnavailable(PathBuf),

    #[error("Fleet initializer artifact {kind} changed: expected {expected}, observed {actual}")]
    ArtifactChanged {
        actual: String,
        expected: String,
        kind: &'static str,
    },

    #[error("Fleet initializer Candid contract is invalid: {0}")]
    CandidContract(String),

    #[error("Fleet initializer {kind} arguments are invalid for method {method}: {detail}")]
    CandidArguments {
        kind: &'static str,
        method: String,
        detail: String,
    },

    #[error("Fleet initializer template contains unresolved authority binding: {0}")]
    UnresolvedBinding(String),

    #[error("Fleet initializer operation identity is not an exact 32-byte hexadecimal value")]
    InvalidOperationIdentity,

    #[error("Fleet initializer argument exceeded {maximum} bytes: {actual}")]
    ArgumentTooLarge { actual: usize, maximum: usize },

    #[error("failed to manage Fleet initializer argument file: {0}")]
    ArgumentFile(#[source] io::Error),
}

pub(super) fn write_init_arguments(
    root: &Path,
    operation_id: &str,
    principals: &BTreeMap<String, String>,
    candid: &str,
    candid_sha256: &str,
    arguments: &str,
    arguments_sha256: &str,
) -> Result<PathBuf, InitArgumentError> {
    let candid_path = verified_path(root, candid, candid_sha256, "init Candid")?;
    let argument_source = verified_text(root, arguments, arguments_sha256, "init arguments")?;
    let argument_source = render(&argument_source, operation_id, principals)?;
    let candid_source = read_bounded(&candid_path)?;
    let candid_source = std::str::from_utf8(&candid_source)
        .map_err(|error| InitArgumentError::CandidContract(error.to_string()))?;
    let (types, (environment, _)) = instantiate_candid(CandidSource::Text(candid_source))
        .map_err(|error| InitArgumentError::CandidContract(error.to_string()))?;
    let arguments = parse_idl_args(&argument_source)
        .map_err(|error| InitArgumentError::CandidArguments {
            kind: "init",
            method: "<init>".to_string(),
            detail: error.to_string(),
        })?
        .to_bytes_with_types(&environment, &types)
        .map_err(|error| InitArgumentError::CandidArguments {
            kind: "init",
            method: "<init>".to_string(),
            detail: error.to_string(),
        })?;
    write_argument_file(&arguments)
}

fn render(
    source: &str,
    operation_id: &str,
    principals: &BTreeMap<String, String>,
) -> Result<String, InitArgumentError> {
    let operation_bytes = canic_core::cdk::utils::hash::decode_hex(operation_id)
        .map_err(|_| InitArgumentError::InvalidOperationIdentity)?;
    if operation_bytes.len() != 32 {
        return Err(InitArgumentError::InvalidOperationIdentity);
    }
    let mut operation_blob = String::with_capacity(operation_bytes.len() * 3);
    for byte in operation_bytes {
        write!(&mut operation_blob, "\\{byte:02x}")
            .map_err(|_| InitArgumentError::InvalidOperationIdentity)?;
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
        return Err(InitArgumentError::UnresolvedBinding(
            rendered[start..end].to_string(),
        ));
    }
    Ok(rendered)
}

fn verified_text(
    root: &Path,
    configured: &str,
    expected: &str,
    kind: &'static str,
) -> Result<String, InitArgumentError> {
    let path = verified_path(root, configured, expected, kind)?;
    let bytes = read_bounded(&path)?;
    String::from_utf8(bytes).map_err(|_| InitArgumentError::ArtifactUnavailable(path))
}

fn verified_path(
    root: &Path,
    configured: &str,
    expected: &str,
    kind: &'static str,
) -> Result<PathBuf, InitArgumentError> {
    let configured = Path::new(configured);
    let path = if configured.is_absolute() {
        configured.to_path_buf()
    } else {
        root.join(configured)
    };
    let bytes = read_bounded(&path)?;
    let actual = canic_core::cdk::utils::hash::sha256_hex(&bytes);
    if actual != expected {
        return Err(InitArgumentError::ArtifactChanged {
            actual,
            expected: expected.to_string(),
            kind,
        });
    }
    Ok(path)
}

fn read_bounded(path: &Path) -> Result<Vec<u8>, InitArgumentError> {
    let metadata = fs::metadata(path)
        .map_err(|_| InitArgumentError::ArtifactUnavailable(path.to_path_buf()))?;
    if !metadata.is_file() || metadata.len() > MAX_INIT_ARTIFACT_BYTES {
        return Err(InitArgumentError::ArtifactUnavailable(path.to_path_buf()));
    }
    fs::read(path).map_err(|_| InitArgumentError::ArtifactUnavailable(path.to_path_buf()))
}

const fn require_argument_bound(bytes: &[u8]) -> Result<(), InitArgumentError> {
    if bytes.len() > MAX_INIT_ARGUMENT_BYTES {
        return Err(InitArgumentError::ArgumentTooLarge {
            actual: bytes.len(),
            maximum: MAX_INIT_ARGUMENT_BYTES,
        });
    }
    Ok(())
}

pub(super) fn write_argument_file(bytes: &[u8]) -> Result<PathBuf, InitArgumentError> {
    require_argument_bound(bytes)?;
    crate::icp::write_binary_argument_file(bytes).map_err(InitArgumentError::ArgumentFile)
}

#[cfg(test)]
mod tests {
    use super::{render, write_init_arguments};
    use canic_core::cdk::utils::hash::sha256_hex;
    use ic_platform::principal::ExternalPrincipal as Principal;
    use std::{collections::BTreeMap, fs};

    #[test]
    fn shared_transport_preserves_the_init_argument_bound() {
        let maximum = super::MAX_INIT_ARGUMENT_BYTES;
        let bytes = vec![0x5a; maximum];
        let path = super::write_argument_file(&bytes).unwrap();
        assert_eq!(fs::read(&path).unwrap(), bytes);
        fs::remove_file(path).unwrap();
        assert!(matches!(
            super::write_argument_file(&vec![0; maximum + 1]),
            Err(super::InitArgumentError::ArgumentTooLarge { actual, maximum: bound })
                if actual == maximum + 1 && bound == maximum
        ));
    }

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
            ic_platform::candid::decoder::decode_args::<(Principal, Vec<u8>)>(&encoded)
                .expect("decode init arguments");
        assert_eq!(principal, Principal::management_canister());
        assert_eq!(operation_bytes, vec![0x22; 32]);
        fs::remove_file(encoded_path).expect("remove encoded init arguments");
    }
}
