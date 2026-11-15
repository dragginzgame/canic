//! Root-owned Store maintenance intent, separate from application slots and Store-local admission.
use serde::{Deserialize, Serialize};

/// Retain one immutable maintenance approval before remote effects.
pub const STAGE: u64 = 50;
/// Advance only the exact retained attempt; issued stop effects are observed, never repeated.
pub const ADVANCE: u64 = 51;
/// Read the surviving Root journal without resuming work.
pub const OBSERVE: u64 = 52;
/// Explicitly install the approved bytes once, then permit restart with admission closed.
pub const INSTALL: u64 = 53;
/// Explicitly activate only the verified replacement and exact retained Store attempt.
pub const ACTIVATE: u64 = 54;
/// Public diagnostic query; never authenticated completion custody or current Store readiness.
pub const STATUS: u64 = 55;
/// Reconcile already-issued effects by observation; never issue fresh maintenance effects.
pub const RECOVER: u64 = 56;
/// Two bounded metadata commitments and issued-effect evidence; child observations keep their budget.
pub const LIMITS: ic_wire::Limits = ic_wire::Limits {
    allocation: 262_144,
    nodes: 2048,
    depth: 16,
    ..super::LIMITS
};
/// First infrastructure replacement fits one management-cache chunk, independently of worker limits.
pub const MAX_MODULE_BYTES: usize = 1_048_576;
/// One replacement request; bytes remain transient in Root and durable with Host.
pub const INSTALL_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: MAX_MODULE_BYTES + 4096,
    allocation: MAX_MODULE_BYTES * 9 + 69_632,
    ..LIMITS
};

/// Host supplies a bounded replacement without making Root its durable artifact repository.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Install {
    /// Exact existing maintenance approval, not fresh or extended authority.
    pub approval: Approval,
    /// Raw Wasm, at most `MAX_MODULE_BYTES` for this first infrastructure path.
    #[serde(with = "serde_bytes")]
    pub wasm: Vec<u8>,
}

/// Exact Store/code boundary and its already-reviewed replacement commitment.
/// Installation and activation each require their explicit controller request.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Approval {
    /// Store controlled solely by this Root; never Root or an application child.
    pub store: String,
    /// Exact current executable.
    pub previous_module: [u8; 32],
    /// Exact proposed different executable, whose bytes remain with Host.
    pub module_hash: [u8; 32],
    /// Store-local monotonic attempt, retained identically by both owners.
    pub attempt: ic_wasm_store_protocol::MaintenanceAttempt,
    /// Last dispatch time for every new effect; at most one hour from staging.
    pub expires_at_ns: u64,
}
/// Committed phases. Stopped is a drain observation, not a successful upgrade.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Phase {
    /// Root has fenced new child/publication mutations; no remote call issued yet.
    Reserved,
    /// Exact closure issued; observe Store to recover a lost reply.
    Closing,
    /// Matching Store closure is retained.
    Closed,
    /// One stop call issued; never reissue it after uncertainty.
    StopIssued,
    /// Sole-control, old code and stopped status observed after the issued call.
    Stopped,
    /// One upload to Store's management cache issued; lost replies recover by chunk observation.
    UploadIssued,
    /// Exact approved replacement chunk observed in Store's management cache.
    Uploaded,
    /// Installation was issued once; uncertainty never permits its replay.
    InstallIssued,
    /// Approved replacement observed under sole Root control, still stopped.
    Installed,
    /// Start was issued once; recover by observing physical status.
    StartIssued,
    /// Running with admission closed while comparing the complete metadata commitment.
    Verifying,
    /// Metadata preservation matches; explicit activation is still required.
    Verified,
    /// Matching activation issued once; observe Store to recover an uncertain result.
    ActivateIssued,
    /// Matching open admission observed; retain the completed operation without resetting it.
    Ready,
}
/// Surviving evidence outside Store; independent of Store availability.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Status {
    /// Original immutable maintenance approval.
    pub approval: Approval,
    /// Captured Root controller; continuation requires the same current controller.
    pub operator: String,
    /// Latest committed phase.
    pub phase: Phase,
    /// Exact encoded stop request hash, present only after issuance.
    pub stop_request: Option<[u8; 32]>,
    /// Latest failure; never evidence that the issued effect did not happen.
    pub failure: Option<super::Failure>,
    /// Closed catalog metadata captured before stopping. Prior CUJ5 records have no such evidence.
    pub preservation: Option<Preservation>,
    /// Installation custody, absent before explicit installation or in migrated CUJ5 records.
    pub installation: Option<Installation>,
}

/// Exact issued effects and replacement verification. Contains no Wasm bytes.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Installation {
    /// Hash of the exact issued upload request; bytes remain with Host.
    pub upload_request: [u8; 32],
    /// Hash of the exact encoded upgrade request, including its original sender version.
    pub request: Option<[u8; 32]>,
    /// Root version included in that original request, never renewed on recovery.
    pub sender_version: Option<u64>,
    /// Start request commitment; absent until issued.
    pub start_request: Option<[u8; 32]>,
    /// Independent post-upgrade scan, compared with the retained pre-stop scan.
    pub verification: Option<Preservation>,
    /// Exact Store activation frame commitment; absent until explicitly issued.
    pub activation_request: Option<[u8; 32]>,
}

/// Bounded resumable catalog commitment, without retaining a second catalog in Root.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Preservation {
    /// Exact counters and quota observed under closed admission.
    pub catalog: ic_wasm_store_protocol::preservation::Catalog,
    /// Last committed release in Store key order.
    pub after: Option<ic_wasm_store_protocol::Release>,
    /// Number of release records included, including aliases and incomplete uploads.
    pub observed: u64,
    /// Ordered metadata commitment; excludes physical payload bytes and management cache.
    pub digest: [u8; 32],
    /// True only after an explicit end page and matching total count.
    pub complete: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn store_maintenance_evidence_fits_its_scoped_metadata_budget() {
        let mut status = Status {
            approval: Approval {
                store: "a".repeat(63),
                previous_module: [254; 32],
                module_hash: [255; 32],
                attempt: ic_wasm_store_protocol::MaintenanceAttempt {
                    sequence: u64::MAX,
                    operation: [255; 32],
                },
                expires_at_ns: u64::MAX,
            },
            operator: "b".repeat(63),
            phase: Phase::StopIssued,
            stop_request: Some([255; 32]),
            failure: Some(crate::Failure::Rejected(u32::MAX)),
            preservation: Some(Preservation {
                catalog: ic_wasm_store_protocol::preservation::Catalog {
                    quota: ic_wasm_store_protocol::Quota {
                        reserved_bytes: u64::MAX,
                        releases: u64::MAX,
                    },
                    reserved_bytes: u64::MAX,
                    releases: u64::MAX,
                    artifacts: u64::MAX,
                    chunks: u64::MAX,
                },
                after: Some(ic_wasm_store_protocol::Release {
                    name: "a".repeat(64),
                    version: "v".repeat(64),
                }),
                observed: u64::MAX,
                digest: [255; 32],
                complete: true,
            }),
            installation: None,
        };
        status.installation = Some(Installation {
            upload_request: [255; 32],
            request: Some([255; 32]),
            sender_version: Some(u64::MAX),
            start_request: Some([255; 32]),
            verification: status.preservation.clone(),
            activation_request: Some([255; 32]),
        });
        let input = Install {
            approval: status.approval.clone(),
            wasm: vec![255; MAX_MODULE_BYTES],
        };
        let bytes = ic_wire::encode(
            &input,
            ic_wire::Encoding::Frame {
                method: INSTALL,
                direction: ic_wire::Direction::Request,
            },
            INSTALL_LIMITS,
        )
        .unwrap();
        assert_eq!(
            ic_wire::decode::<Install>(
                &bytes,
                ic_wire::Encoding::Frame {
                    method: INSTALL,
                    direction: ic_wire::Direction::Request
                },
                INSTALL_LIMITS
            )
            .unwrap()
            .wasm,
            input.wasm
        );
        assert!(
            ic_wire::decode::<Install>(
                &bytes,
                ic_wire::Encoding::Frame {
                    method: INSTALL,
                    direction: ic_wire::Direction::Request
                },
                crate::STAGE_LIMITS
            )
            .is_err()
        );
        let reply = Ok::<_, crate::Refusal>(status.clone());
        let bytes = ic_wire::encode(
            &reply,
            ic_wire::Encoding::Frame {
                method: STAGE,
                direction: ic_wire::Direction::Response,
            },
            LIMITS,
        )
        .unwrap();
        assert_eq!(
            ic_wire::decode::<Result<Status, crate::Refusal>>(
                &bytes,
                ic_wire::Encoding::Frame {
                    method: STAGE,
                    direction: ic_wire::Direction::Response
                },
                LIMITS
            )
            .unwrap(),
            reply
        );
        for method in [OBSERVE, STATUS] {
            let bytes = ic_wire::encode(
                &Some(status.clone()),
                ic_wire::Encoding::Frame {
                    method,
                    direction: ic_wire::Direction::Response,
                },
                LIMITS,
            )
            .unwrap();
            assert_eq!(
                ic_wire::decode::<Option<Status>>(
                    &bytes,
                    ic_wire::Encoding::Frame {
                        method,
                        direction: ic_wire::Direction::Response
                    },
                    LIMITS
                )
                .unwrap(),
                Some(status.clone())
            );
        }
        let bytes = ic_wire::encode(
            &status,
            ic_wire::Encoding::Frame {
                method: RECOVER,
                direction: ic_wire::Direction::Response,
            },
            LIMITS,
        )
        .unwrap();
        assert_eq!(
            ic_wire::decode::<Status>(
                &bytes,
                ic_wire::Encoding::Frame {
                    method: RECOVER,
                    direction: ic_wire::Direction::Response
                },
                LIMITS
            )
            .unwrap(),
            status
        );
    }
}
