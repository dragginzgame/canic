use crate::cycles::{
    CyclesCommandError,
    convert::{
        options::ConvertOptions,
        pending::{
            PendingIcpRefillOperationInput, PendingOperationLogError,
            complete_pending_icp_refill_operation, reserve_pending_icp_refill_operation,
        },
    },
    wallet::ResolvedCanisterTarget,
};
use canic_core::cdk::utils::hash::{hex_bytes, sha256_bytes};
use std::{
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

///
/// OperationIdSource
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum OperationIdSource {
    Provided,
    Generated,
    PendingLog,
}

impl OperationIdSource {
    const fn label(self) -> &'static str {
        match self {
            Self::Provided => "provided",
            Self::Generated => "generated",
            Self::PendingLog => "pending_log",
        }
    }
}

pub(super) fn pending_operation_input<'a>(
    root: &'a Path,
    options: &'a ConvertOptions,
    source: &'a ResolvedCanisterTarget,
    target: &'a ResolvedCanisterTarget,
    amount_e8s: u64,
    now_nanos: u128,
) -> PendingIcpRefillOperationInput<'a> {
    PendingIcpRefillOperationInput {
        icp_root: root,
        network: &options.target.network,
        deployment: &options.deployment,
        source: source.role.as_deref(),
        source_canister_id: &source.canister_id,
        source_subaccount: options.source_subaccount,
        target: target.role.as_deref(),
        target_canister_id: &target.canister_id,
        amount_e8s,
        created_at_unix_nanos: now_nanos,
    }
}

pub(super) fn mark_pending_operation_completed(
    root: &Path,
    operation_key: Option<&str>,
    operation_id: [u8; 32],
) -> Result<(), CyclesCommandError> {
    if let Some(operation_key) = operation_key {
        complete_pending_icp_refill_operation(
            root,
            operation_key,
            operation_id,
            current_unix_nanos(),
        )
        .map_err(pending_operation_log_error)?;
    }
    Ok(())
}

pub(super) fn resolve_operation_id(
    provided: Option<[u8; 32]>,
    pending_input: &PendingIcpRefillOperationInput<'_>,
    dry_run: bool,
    now_nanos: u128,
) -> Result<([u8; 32], OperationIdSource, Option<String>), CyclesCommandError> {
    if let Some(operation_id) = provided {
        return Ok((operation_id, OperationIdSource::Provided, None));
    }
    let generated = generated_operation_id(
        pending_input.deployment,
        pending_input.source_canister_id,
        pending_input.target_canister_id,
        pending_input.amount_e8s,
        now_nanos,
    );
    if dry_run {
        return Ok((generated, OperationIdSource::Generated, None));
    }
    let reserved = reserve_pending_icp_refill_operation(pending_input, generated)
        .map_err(pending_operation_log_error)?;
    let source = if reserved.reused {
        OperationIdSource::PendingLog
    } else {
        OperationIdSource::Generated
    };
    Ok((reserved.operation_id, source, Some(reserved.operation_key)))
}

pub(super) fn write_generated_operation_id_notice(
    json: bool,
    operation_id: [u8; 32],
    source: OperationIdSource,
) {
    if json {
        return;
    }
    if let Some(notice) = generated_operation_id_notice(operation_id, source) {
        println!("{notice}");
    }
}

pub(super) fn current_unix_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos())
}

fn generated_operation_id(
    deployment: &str,
    source_canister: &str,
    target_canister: &str,
    amount_e8s: u64,
    now_nanos: u128,
) -> [u8; 32] {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"canic:cycles-convert:icp-refill:v1");
    extend_operation_id_part(&mut bytes, deployment.as_bytes());
    extend_operation_id_part(&mut bytes, source_canister.as_bytes());
    extend_operation_id_part(&mut bytes, target_canister.as_bytes());
    extend_operation_id_part(&mut bytes, &amount_e8s.to_be_bytes());
    extend_operation_id_part(&mut bytes, &now_nanos.to_be_bytes());
    let digest = sha256_bytes(&bytes);
    let mut operation_id = [0; 32];
    operation_id.copy_from_slice(&digest);
    operation_id
}

fn pending_operation_log_error(err: PendingOperationLogError) -> CyclesCommandError {
    CyclesCommandError::PendingOperationLog(err.to_string())
}

fn generated_operation_id_notice(
    operation_id: [u8; 32],
    source: OperationIdSource,
) -> Option<String> {
    matches!(
        source,
        OperationIdSource::Generated | OperationIdSource::PendingLog
    )
    .then(|| {
        format!(
            "operation_id={}\noperation_id_source={}",
            hex_bytes(operation_id),
            source.label()
        )
    })
}

fn extend_operation_id_part(bytes: &mut Vec<u8>, part: &[u8]) {
    bytes.extend_from_slice(&(part.len() as u64).to_be_bytes());
    bytes.extend_from_slice(part);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::temp_dir;

    #[test]
    fn generated_operation_id_binds_input() {
        let left = generated_operation_id("demo", "source", "target", 1, 10);
        let right = generated_operation_id("demo", "source", "target", 2, 10);
        let next_time = generated_operation_id("demo", "source", "target", 1, 11);

        assert_ne!(left, right);
        assert_ne!(left, next_time);
    }

    #[test]
    fn resolves_provided_operation_id_without_generation_notice() {
        let root = temp_dir("canic-cli-convert-provided-operation-id");
        let pending_input = pending_input(&root);
        let operation_id = [7; 32];
        let (resolved, source, pending_key) =
            resolve_operation_id(Some(operation_id), &pending_input, false, 10)
                .expect("resolve operation id");

        assert_eq!(resolved, operation_id);
        assert_eq!(source, OperationIdSource::Provided);
        assert_eq!(pending_key, None);
        assert_eq!(generated_operation_id_notice(resolved, source), None);
    }

    #[test]
    fn generated_operation_id_notice_is_operator_visible() {
        let root = temp_dir("canic-cli-convert-generated-operation-id");
        let pending_input = pending_input(&root);
        let (operation_id, source, pending_key) =
            resolve_operation_id(None, &pending_input, true, 10).expect("resolve operation id");
        let notice = generated_operation_id_notice(operation_id, source)
            .expect("generated operation id should be printed");

        assert_eq!(source, OperationIdSource::Generated);
        assert_eq!(pending_key, None);
        assert!(notice.contains(&format!("operation_id={}", hex_bytes(operation_id))));
        assert!(notice.contains("operation_id_source=generated"));
    }

    #[test]
    fn pending_log_reuse_notice_is_operator_visible() {
        let root = temp_dir("canic-cli-convert-pending-operation-id");
        let pending_input = pending_input(&root);
        let (first_id, first_source, first_key) =
            resolve_operation_id(None, &pending_input, false, 10).expect("resolve first id");
        let (second_id, second_source, second_key) =
            resolve_operation_id(None, &pending_input, false, 11).expect("resolve second id");
        let notice = generated_operation_id_notice(second_id, second_source)
            .expect("pending operation id should be printed");

        assert_eq!(first_source, OperationIdSource::Generated);
        assert_eq!(second_source, OperationIdSource::PendingLog);
        assert_eq!(first_id, second_id);
        assert_eq!(first_key, second_key);
        assert!(notice.contains(&format!("operation_id={}", hex_bytes(second_id))));
        assert!(notice.contains("operation_id_source=pending_log"));
    }

    #[test]
    fn pending_completion_failure_is_returned() {
        let root = temp_dir("canic-cli-convert-pending-completion-error");
        let pending_input = pending_input(&root);
        let (operation_id, _, pending_key) =
            resolve_operation_id(None, &pending_input, false, 10).expect("reserve operation id");
        let pending_path = root.join(".canic/operations/pending.json");
        std::fs::write(&pending_path, "not json").expect("corrupt pending log");

        let error = mark_pending_operation_completed(&root, pending_key.as_deref(), operation_id)
            .expect_err("completion error must propagate");

        assert!(matches!(error, CyclesCommandError::PendingOperationLog(_)));
    }

    fn pending_input(root: &Path) -> PendingIcpRefillOperationInput<'_> {
        PendingIcpRefillOperationInput {
            icp_root: root,
            network: "ic",
            deployment: "demo",
            source: Some("funding_hub"),
            source_canister_id: "source",
            source_subaccount: None,
            target: Some("app"),
            target_canister_id: "target",
            amount_e8s: 1,
            created_at_unix_nanos: 10,
        }
    }
}
