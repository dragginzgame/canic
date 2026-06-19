use canic_core::cdk::utils::hash::{decode_hex, hex_bytes, sha256_bytes};
use serde::{Deserialize, Serialize};
use std::{
    fmt, fs, io,
    io::Write,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

const PENDING_OPERATION_LOG_SCHEMA_VERSION: u32 = 1;
const PENDING_OPERATION_ENTRY_SCHEMA_VERSION: u32 = 1;
const ICP_REFILL_COMMAND_KIND: &str = "icp.refill.v1";
const CYCLES_CONVERT_COMMAND: &str = "canic cycles convert";
const OPERATION_ID_SOURCE_GENERATED: &str = "generated";
const STATUS_COMPLETED: &str = "completed";
const STATUS_PENDING_SEND: &str = "pending_send";

///
/// PendingIcpRefillOperationInput
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct PendingIcpRefillOperationInput<'a> {
    pub(super) icp_root: &'a Path,
    pub(super) network: &'a str,
    pub(super) deployment: &'a str,
    pub(super) source: Option<&'a str>,
    pub(super) source_canister_id: &'a str,
    pub(super) source_subaccount: Option<[u8; 32]>,
    pub(super) target: Option<&'a str>,
    pub(super) target_canister_id: &'a str,
    pub(super) amount_e8s: u64,
    pub(super) created_at_unix_nanos: u128,
}

///
/// PendingOperationReserveResult
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct PendingOperationReserveResult {
    pub(super) operation_id: [u8; 32],
    pub(super) operation_key: String,
    pub(super) reused: bool,
    pub(super) path: PathBuf,
}

///
/// PendingOperationLogError
///

#[derive(Debug)]
pub(super) struct PendingOperationLogError {
    path: PathBuf,
    message: String,
}

impl PendingOperationLogError {
    fn new(path: PathBuf, message: impl Into<String>) -> Self {
        Self {
            path,
            message: message.into(),
        }
    }
}

impl fmt::Display for PendingOperationLogError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} ({})", self.message, self.path.display())
    }
}

impl std::error::Error for PendingOperationLogError {}

///
/// PendingOperationLog
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct PendingOperationLog {
    schema_version: u32,
    operations: Vec<PendingOperationRecord>,
}

impl PendingOperationLog {
    const fn empty() -> Self {
        Self {
            schema_version: PENDING_OPERATION_LOG_SCHEMA_VERSION,
            operations: Vec::new(),
        }
    }
}

///
/// PendingOperationRecord
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct PendingOperationRecord {
    schema_version: u32,
    command_kind: String,
    operation_key: String,
    operation_id: String,
    operation_id_source: String,
    status: String,
    cli_command: String,
    network: String,
    deployment: String,
    source: Option<String>,
    source_canister_id: String,
    source_subaccount: Option<String>,
    target: Option<String>,
    target_canister_id: String,
    amount_e8s: u64,
    created_at_unix_nanos: String,
    completed_at_unix_nanos: Option<String>,
}

pub(super) fn reserve_pending_icp_refill_operation(
    input: &PendingIcpRefillOperationInput<'_>,
    generated_operation_id: [u8; 32],
) -> Result<PendingOperationReserveResult, PendingOperationLogError> {
    let path = pending_operation_log_path(input.icp_root);
    let mut log = read_pending_operation_log(&path)?;
    let operation_key = icp_refill_operation_key(input);

    if let Some(record) = log.operations.iter().rev().find(|record| {
        record.operation_key == operation_key && record.status == STATUS_PENDING_SEND
    }) {
        return Ok(PendingOperationReserveResult {
            operation_id: parse_logged_operation_id(&path, &record.operation_id)?,
            operation_key,
            reused: true,
            path,
        });
    }

    log.operations.push(PendingOperationRecord {
        schema_version: PENDING_OPERATION_ENTRY_SCHEMA_VERSION,
        command_kind: ICP_REFILL_COMMAND_KIND.to_string(),
        operation_key: operation_key.clone(),
        operation_id: hex_bytes(generated_operation_id),
        operation_id_source: OPERATION_ID_SOURCE_GENERATED.to_string(),
        status: STATUS_PENDING_SEND.to_string(),
        cli_command: CYCLES_CONVERT_COMMAND.to_string(),
        network: input.network.to_string(),
        deployment: input.deployment.to_string(),
        source: input.source.map(ToOwned::to_owned),
        source_canister_id: input.source_canister_id.to_string(),
        source_subaccount: input.source_subaccount.map(hex_bytes),
        target: input.target.map(ToOwned::to_owned),
        target_canister_id: input.target_canister_id.to_string(),
        amount_e8s: input.amount_e8s,
        created_at_unix_nanos: input.created_at_unix_nanos.to_string(),
        completed_at_unix_nanos: None,
    });
    write_pending_operation_log(&path, &log)?;

    Ok(PendingOperationReserveResult {
        operation_id: generated_operation_id,
        operation_key,
        reused: false,
        path,
    })
}

pub(super) fn complete_pending_icp_refill_operation(
    icp_root: &Path,
    operation_key: &str,
    operation_id: [u8; 32],
    completed_at_unix_nanos: u128,
) -> Result<(), PendingOperationLogError> {
    let path = pending_operation_log_path(icp_root);
    let mut log = read_pending_operation_log(&path)?;
    let operation_id = hex_bytes(operation_id);
    let Some(record) = log.operations.iter_mut().rev().find(|record| {
        record.operation_key == operation_key
            && record.operation_id == operation_id
            && record.status == STATUS_PENDING_SEND
    }) else {
        return Ok(());
    };
    record.status = STATUS_COMPLETED.to_string();
    record.completed_at_unix_nanos = Some(completed_at_unix_nanos.to_string());
    write_pending_operation_log(&path, &log)
}

fn pending_operation_log_path(icp_root: &Path) -> PathBuf {
    icp_root
        .join(".canic")
        .join("operations")
        .join("pending.json")
}

fn read_pending_operation_log(
    path: &Path,
) -> Result<PendingOperationLog, PendingOperationLogError> {
    match fs::read_to_string(path) {
        Ok(source) => {
            let log = serde_json::from_str::<PendingOperationLog>(&source).map_err(|err| {
                PendingOperationLogError::new(
                    path.to_path_buf(),
                    format!("pending operation log is not valid JSON: {err}"),
                )
            })?;
            if log.schema_version != PENDING_OPERATION_LOG_SCHEMA_VERSION {
                return Err(PendingOperationLogError::new(
                    path.to_path_buf(),
                    format!(
                        "unsupported pending operation log schema {}, expected {}",
                        log.schema_version, PENDING_OPERATION_LOG_SCHEMA_VERSION
                    ),
                ));
            }
            Ok(log)
        }
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(PendingOperationLog::empty()),
        Err(err) => Err(PendingOperationLogError::new(
            path.to_path_buf(),
            format!("failed to read pending operation log: {err}"),
        )),
    }
}

fn write_pending_operation_log(
    path: &Path,
    log: &PendingOperationLog,
) -> Result<(), PendingOperationLogError> {
    let Some(parent) = path.parent() else {
        return Err(PendingOperationLogError::new(
            path.to_path_buf(),
            "pending operation log path has no parent",
        ));
    };
    fs::create_dir_all(parent).map_err(|err| {
        PendingOperationLogError::new(
            parent.to_path_buf(),
            format!("failed to create pending operation log directory: {err}"),
        )
    })?;
    let data = serde_json::to_string_pretty(log).map_err(|err| {
        PendingOperationLogError::new(
            path.to_path_buf(),
            format!("failed to serialize pending operation log: {err}"),
        )
    })?;
    let temp_path = pending_temp_path(path);
    {
        let mut temp = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_path)
            .map_err(|err| {
                PendingOperationLogError::new(
                    temp_path.clone(),
                    format!("failed to create pending operation temp file: {err}"),
                )
            })?;
        temp.write_all(data.as_bytes()).map_err(|err| {
            PendingOperationLogError::new(
                temp_path.clone(),
                format!("failed to write pending operation temp file: {err}"),
            )
        })?;
        temp.sync_all().map_err(|err| {
            PendingOperationLogError::new(
                temp_path.clone(),
                format!("failed to sync pending operation temp file: {err}"),
            )
        })?;
    }
    fs::rename(&temp_path, path).map_err(|err| {
        PendingOperationLogError::new(
            path.to_path_buf(),
            format!("failed to replace pending operation log: {err}"),
        )
    })?;
    sync_directory(parent).map_err(|err| {
        PendingOperationLogError::new(
            parent.to_path_buf(),
            format!("failed to sync pending operation log directory: {err}"),
        )
    })
}

fn pending_temp_path(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .and_then(|file_name| file_name.to_str())
        .unwrap_or("pending.json");
    path.with_file_name(format!(
        "{file_name}.tmp.{}.{}",
        std::process::id(),
        current_unix_nanos()
    ))
}

fn sync_directory(path: &Path) -> io::Result<()> {
    fs::File::open(path).and_then(|dir| dir.sync_all())
}

fn parse_logged_operation_id(
    path: &Path,
    value: &str,
) -> Result<[u8; 32], PendingOperationLogError> {
    let bytes = decode_hex(value).map_err(|err| {
        PendingOperationLogError::new(
            path.to_path_buf(),
            format!("pending operation log has invalid operation_id: {err}"),
        )
    })?;
    <[u8; 32]>::try_from(bytes.as_slice()).map_err(|_| {
        PendingOperationLogError::new(
            path.to_path_buf(),
            format!(
                "pending operation log has invalid operation_id length: expected 32 bytes, got {}",
                bytes.len()
            ),
        )
    })
}

fn icp_refill_operation_key(input: &PendingIcpRefillOperationInput<'_>) -> String {
    let mut bytes = Vec::new();
    extend_key_part(&mut bytes, b"canic:pending-operation:icp-refill:v1");
    extend_key_part(&mut bytes, input.network.as_bytes());
    extend_key_part(&mut bytes, input.deployment.as_bytes());
    extend_key_part(&mut bytes, input.source_canister_id.as_bytes());
    extend_key_part(&mut bytes, input.target_canister_id.as_bytes());
    extend_optional_key_part(
        &mut bytes,
        input.source_subaccount.as_ref().map(AsRef::as_ref),
    );
    extend_key_part(&mut bytes, &input.amount_e8s.to_be_bytes());
    hex_bytes(sha256_bytes(&bytes))
}

fn extend_optional_key_part(bytes: &mut Vec<u8>, part: Option<&[u8]>) {
    match part {
        Some(part) => {
            bytes.push(1);
            extend_key_part(bytes, part);
        }
        None => bytes.push(0),
    }
}

fn extend_key_part(bytes: &mut Vec<u8>, part: &[u8]) {
    bytes.extend_from_slice(&(part.len() as u64).to_be_bytes());
    bytes.extend_from_slice(part);
}

fn current_unix_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::temp_dir;

    #[test]
    fn pending_log_path_is_project_local() {
        assert_eq!(
            pending_operation_log_path(Path::new("/tmp/canic")),
            PathBuf::from("/tmp/canic/.canic/operations/pending.json")
        );
    }

    #[test]
    fn reserve_writes_generated_operation_before_send() {
        let root = temp_dir("canic-cli-pending-operation-write");
        let input = sample_input(&root);
        let operation_id = [7; 32];
        let result = reserve_pending_icp_refill_operation(&input, operation_id)
            .expect("reserve pending operation");

        assert!(!result.reused);
        assert_eq!(result.operation_id, operation_id);
        assert!(result.path.is_file());

        let log = read_pending_operation_log(&result.path).expect("read log");
        assert_eq!(log.schema_version, 1);
        assert_eq!(log.operations.len(), 1);
        let record = &log.operations[0];
        assert_eq!(record.command_kind, ICP_REFILL_COMMAND_KIND);
        assert_eq!(record.operation_id, hex_bytes(operation_id));
        assert_eq!(record.operation_id_source, OPERATION_ID_SOURCE_GENERATED);
        assert_eq!(record.status, STATUS_PENDING_SEND);
        assert_eq!(record.cli_command, CYCLES_CONVERT_COMMAND);
        assert_eq!(record.network, "ic");
        assert_eq!(record.deployment, "demo");
        assert_eq!(record.source_canister_id, "source-canister");
        assert_eq!(record.target_canister_id, "target-canister");
        assert_eq!(record.amount_e8s, 100_000_000);
    }

    #[test]
    fn reserve_reuses_matching_pending_send_operation() {
        let root = temp_dir("canic-cli-pending-operation-reuse");
        let input = sample_input(&root);
        let first =
            reserve_pending_icp_refill_operation(&input, [3; 32]).expect("reserve first operation");
        let second = reserve_pending_icp_refill_operation(&input, [4; 32])
            .expect("reserve second operation");

        assert!(!first.reused);
        assert!(second.reused);
        assert_eq!(second.operation_id, [3; 32]);

        let log = read_pending_operation_log(&second.path).expect("read log");
        assert_eq!(log.operations.len(), 1);
    }

    #[test]
    fn completed_pending_operation_is_not_reused() {
        let root = temp_dir("canic-cli-pending-operation-completed");
        let input = sample_input(&root);
        let first =
            reserve_pending_icp_refill_operation(&input, [3; 32]).expect("reserve first operation");
        complete_pending_icp_refill_operation(&root, &first.operation_key, [3; 32], 222)
            .expect("complete operation");

        let second = reserve_pending_icp_refill_operation(&input, [4; 32])
            .expect("reserve second operation");

        assert!(!second.reused);
        assert_eq!(second.operation_id, [4; 32]);

        let log = read_pending_operation_log(&second.path).expect("read log");
        assert_eq!(log.operations.len(), 2);
        assert_eq!(log.operations[0].status, STATUS_COMPLETED);
        assert_eq!(
            log.operations[0].completed_at_unix_nanos.as_deref(),
            Some("222")
        );
        assert_eq!(log.operations[1].status, STATUS_PENDING_SEND);
    }

    fn sample_input(root: &Path) -> PendingIcpRefillOperationInput<'_> {
        PendingIcpRefillOperationInput {
            icp_root: root,
            network: "ic",
            deployment: "demo",
            source: Some("funding_hub"),
            source_canister_id: "source-canister",
            source_subaccount: None,
            target: Some("app"),
            target_canister_id: "target-canister",
            amount_e8s: 100_000_000,
            created_at_unix_nanos: 111,
        }
    }
}
