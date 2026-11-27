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
            after_removed: true,
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
    let input = Request::Install(Install {
        approval: status.approval.clone(),
        wasm: vec![255; MAX_MODULE_BYTES],
    });
    let bytes = ic_wire::encode(
        &input,
        ic_wire::Encoding::Frame {
            method: MAINTAIN,
            direction: ic_wire::Direction::Request,
        },
        INSTALL_LIMITS,
    )
    .unwrap();
    let Request::Install(decoded) = ic_wire::decode::<Request>(
        &bytes,
        ic_wire::Encoding::Frame {
            method: MAINTAIN,
            direction: ic_wire::Direction::Request,
        },
        INSTALL_LIMITS,
    )
    .unwrap() else {
        panic!("expected installation")
    };
    assert_eq!(decoded.wasm, vec![255; MAX_MODULE_BYTES]);
    let old_install = ic_wire::encode(
        &decoded,
        ic_wire::Encoding::Frame {
            method: 53,
            direction: ic_wire::Direction::Request,
        },
        INSTALL_LIMITS,
    )
    .unwrap();
    assert!(
        ic_wire::decode::<Request>(
            &old_install,
            ic_wire::Encoding::Frame {
                method: MAINTAIN,
                direction: ic_wire::Direction::Request
            },
            INSTALL_LIMITS
        )
        .is_err()
    );
    for retired in [50, 51, 54] {
        let original = ic_wire::encode(
            &status.approval,
            ic_wire::Encoding::Frame {
                method: retired,
                direction: ic_wire::Direction::Request,
            },
            LIMITS,
        )
        .unwrap();
        assert!(
            ic_wire::decode::<Request>(
                &original,
                ic_wire::Encoding::Frame {
                    method: MAINTAIN,
                    direction: ic_wire::Direction::Request
                },
                INSTALL_LIMITS
            )
            .is_err()
        );
    }
    assert!(
        ic_wire::decode::<Request>(
            &bytes,
            ic_wire::Encoding::Frame {
                method: MAINTAIN,
                direction: ic_wire::Direction::Request
            },
            crate::LIMITS
        )
        .is_err()
    );
    let reply = Ok::<_, crate::Refusal>(status.clone());
    let bytes = ic_wire::encode(
        &reply,
        ic_wire::Encoding::Frame {
            method: MAINTAIN,
            direction: ic_wire::Direction::Response,
        },
        LIMITS,
    )
    .unwrap();
    assert_eq!(
        ic_wire::decode::<Result<Status, crate::Refusal>>(
            &bytes,
            ic_wire::Encoding::Frame {
                method: MAINTAIN,
                direction: ic_wire::Direction::Response
            },
            LIMITS
        )
        .unwrap(),
        reply
    );
    for method in [READ_ATTEMPT, STATUS] {
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
