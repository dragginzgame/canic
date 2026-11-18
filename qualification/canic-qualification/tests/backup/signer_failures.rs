//! A failed signing reservation must remain spent across Backup journal reopen.
use super::StoppedFixture;
use canic_host::backup::DirectSigner;
use ic_backup::store::{
    BackupConnection, BackupStore, CapturePlan, OperationAdvanceError, OperationOptions,
    OperationProgress, OperationStep,
};
use ic_host_sign::{
    Error, Signer,
    protocol::{Configuration, tool_hash},
};
use std::{fs, os::unix::fs::PermissionsExt, path::Path, time::Duration};

pub(super) async fn exercise(
    dir: &Path,
    config: &Configuration,
    connect: &impl Fn(DirectSigner) -> BackupConnection<DirectSigner, StoppedFixture>,
    original: &CapturePlan,
    options: OperationOptions,
) {
    let oversized = format!(
        "exec head -c {} /dev/zero",
        ic_host_sign::protocol::FRAME_BYTES + 1
    );
    for (name, behavior) in [
        ("partial", "printf '{'"),
        ("oversized", oversized.as_str()),
        ("death", "kill -KILL $$"),
        ("deadline", "exec sleep 60"),
        ("cancel", "exec sleep 60"),
    ] {
        let case = dir.join(name);
        fs::create_dir(&case).unwrap();
        let tool = case.join("sign");
        fs::write(
            &tool,
            format!("#!/bin/sh\ncat >/dev/null\necho $$ >> \"$0.pid\"\n{behavior}\n"),
        )
        .unwrap();
        fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
        let digest = tool_hash(&tool).unwrap();
        let open = || {
            connect(DirectSigner(
                Signer::new(&tool, digest, config.clone()).unwrap(),
            ))
        };
        let mut connection = open();
        let mut options = options;
        if name == "deadline" {
            options.prepare_duration_ns = 1_000_000_000;
        }
        let plan = connection
            .plan(original.request().clone(), options)
            .unwrap();
        let store_path = case.join("store");
        fs::create_dir(&store_path).unwrap();
        let store = BackupStore::initialize(&store_path).unwrap();
        let mut capture = store.create_capture(plan.clone(), plan.digest()).unwrap();
        let launched = tool.with_extension("pid");
        if name == "cancel" {
            // Drop the operation future only once the actual process has started.
            let wait = async {
                while !fs::read_to_string(&launched).is_ok_and(|pid| pid.ends_with('\n')) {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            };
            tokio::select! {
                result = capture.advance(&mut connection) => panic!("premature signer completion: {result:?}"),
                result = tokio::time::timeout(Duration::from_secs(5),wait) => result.unwrap(),
            }
        } else {
            let error = capture.advance(&mut connection).await.unwrap_err();
            match name {
                "partial" => assert!(matches!(
                    error,
                    OperationAdvanceError::Signer(Error::Output)
                )),
                "deadline" => assert!(matches!(
                    error,
                    OperationAdvanceError::Deadline
                        | OperationAdvanceError::Signer(Error::Deadline | Error::Execution { .. })
                )),
                _ => assert!(matches!(
                    error,
                    OperationAdvanceError::Signer(Error::Execution { .. })
                )),
            }
        }
        drop(connection);
        let invocation = fs::read_to_string(&launched).unwrap();
        let pid: u32 = invocation
            .trim()
            .parse()
            .expect("exactly one provider invocation");
        assert!(
            !Path::new("/proc").join(pid.to_string()).exists(),
            "provider still running after cleanup"
        );
        let view = capture.inspect().unwrap();
        assert_eq!(view.signing_requests_reserved, 1);
        assert_eq!(view.observations_reserved, 0);
        assert_eq!(view.request_id, None);
        drop(capture);
        let journal = store_path
            .join("operations")
            .join(&plan.request().operation_id)
            .join("journal.bin");
        let retained = fs::read(&journal).unwrap();
        let mut reopened = store
            .open_capture(&plan.request().operation_id, plan.digest())
            .unwrap();
        let mut connection = open();
        assert!(matches!(
            reopened.advance(&mut connection).await.unwrap(),
            OperationProgress::NeedsAttention(OperationStep::InitialStatus)
        ));
        assert_eq!(fs::read(&journal).unwrap(), retained);
        assert_eq!(fs::read_to_string(&launched).unwrap(), invocation);
    }
}
