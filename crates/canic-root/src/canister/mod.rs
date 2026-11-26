mod backup;
mod placement;

use canic_management::{self as management, Readiness};
use canic_management_protocol::Failure;
use ic_platform_runtime as runtime;

// init
// Create the Root journal with the approved fleet ceiling and initialize enrollment.
// guard  : lifecycle_guard (IC lifecycle)
// limits : up to 64 install bytes and no reply
#[runtime::init(
    runtime = runtime,
    guard = lifecycle_guard,
    decode = bootstrap_decode,
    max_bytes = 64,
    on_error = endpoint_error,
)]
fn init(policy: Option<canic_management_protocol::stored::RootBootstrap>) {
    management::open(true, readiness());
    if let Some(policy) = policy {
        management::bootstrap(policy);
    }
    canic::logging::record(canic::logging::Level::Info, "root", "installed");

    crate::auth::fresh();
}

// post_upgrade
// Reopen the root state from the preceding executable.
// guard  : lifecycle_guard (IC lifecycle)
// limits : empty request and no reply
#[runtime::post_upgrade(
    runtime = runtime,
    guard = lifecycle_guard,
    decode = lifecycle_decode,
    max_bytes = 0,
    on_error = endpoint_error,
)]
fn post_upgrade((): ()) {
    management::open(false, readiness());
    if !management::auth_grant_issued() {
        crate::auth::fresh();
    }
    canic::logging::record(
        canic::logging::Level::Info,
        "root",
        "reopened after upgrade",
    );
}

management::mount_endpoints! {
    publish_store_artifact, observe_store, change_store_admission, remove_store_artifact,
    maintain_store, observe_store_maintenance, store_maintenance_status, recover_store_maintenance,
    observe_root_custody, observe_child_deployments,
    stage_child_creation, reconcile_child_creation,
    stage_child_upgrade, reconcile_child_upgrade,
    child_status, observe_child_status,
}

management::mount_endpoints! {
    activate_child_upgrade,
}

// redeliver_directory_revision
// Deliver one exact revision to a ready named service under the operator's approval.
// guard  : Root controller
// limits : default
#[canic::update(contract = canic::topology::directory::RedeliverDirectoryRevision, guard = canic::guard::require_controller)]
async fn redeliver_directory_revision(
    request: (String, u64),
) -> Result<u64, canic::topology::directory::RedeliveryError> {
    let (name, revision) = request;
    management::application::redeliver_directory_revision(&name, revision).await
}

// observe_child_metrics
// Read bounded performance observations from an installed, ready child of this Root.
// guard  : Root controller
// limits : ObserveChildMetrics::REQUEST and ObserveChildMetrics::REPLY (4,096-byte reply)
#[canic::update(contract = canic::metrics::ObserveChildMetrics, guard = canic::guard::require_controller)]
async fn observe_child_metrics(request: (u32, canic::metrics::Request)) -> canic::metrics::Report {
    use canic::metrics::ReadMetrics;
    use canic_management_protocol::{Phase, children::ChildId};
    let (slot, source) = request;
    let slot =
        ChildId::new(slot as usize).unwrap_or_else(|| runtime::system::trap("unknown child slot"));
    let status = management::children::status(slot);
    if status.phase != Phase::Ready {
        runtime::system::trap("child is not ready");
    }
    let target = status
        .worker
        .expect("ready child")
        .parse()
        .expect("retained child principal");
    let caller = runtime::system::caller();
    let report = canic::method::call::<ReadMetrics>(target, 5, &source)
        .await
        .unwrap_or_else(|error| canic::method::fail(error));
    assert!(
        runtime::system::is_controller(&caller),
        "controller required"
    );
    report
}

fn readiness() -> Readiness {
    Readiness {
        method: "readiness",
        request: ic_wire::encode(
            &(),
            ic_wire::Encoding::Frame {
                method: canic::readiness::METHOD,
                direction: ic_wire::Direction::Request,
            },
            canic::readiness::REQUEST_LIMITS,
        )
        .expect("readiness args"),
        accept: |reply| {
            let state: canic::readiness::State = ic_wire::decode(
                reply,
                ic_wire::Encoding::Frame {
                    method: canic::readiness::METHOD,
                    direction: ic_wire::Direction::Response,
                },
                canic::readiness::RESPONSE_LIMITS,
            )
            .map_err(|_| Failure::InvalidReply)?;
            match state {
                canic::readiness::State::Ready => Ok(()),
                canic::readiness::State::Unavailable | canic::readiness::State::Fenced => {
                    Err(Failure::Mismatch)
                }
            }
        },
    }
}

#[cfg(feature = "fault-injection")]
management::mount_endpoints! { inject_child_fault }

#[canic::update(contract = canic_management_protocol::stored::ConfigureSnapshotReaders, guard = canic::guard::require_controller)]
async fn configure_snapshot_readers(
    input: canic_management_protocol::stored::SnapshotReaders,
) -> Result<(), canic_management_protocol::Failure> {
    management::configure_snapshot_readers(input).await
}

fn endpoint_error(error: impl std::fmt::Debug) -> ! {
    ic_platform_runtime::system::trap(&format!("endpoint refused: {error:?}"))
}

/// The IC admits lifecycle calls. The declared argument budget still applies.
fn lifecycle_guard() -> Result<(), std::convert::Infallible> {
    Ok(())
}

fn bootstrap_decode(
    bytes: &[u8],
) -> Result<Option<canic_management_protocol::stored::RootBootstrap>, ic_wire::Error> {
    if bytes.is_empty() {
        return Ok(None);
    }
    ic_wire::decode(
        bytes,
        ic_wire::Encoding::Payload,
        canic_management_protocol::LIMITS,
    )
    .map(Some)
}

fn lifecycle_decode(_bytes: &[u8]) -> Result<(), std::convert::Infallible> {
    Ok(())
}
