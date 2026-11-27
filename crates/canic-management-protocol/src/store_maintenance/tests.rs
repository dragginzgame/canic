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
        preservation: Scan::Entries(Some(Preservation {
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
        })),
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

#[test]
fn scan_codec_retains_historical_bytes_and_refuses_unknown_contracts() {
    // Historical Option<struct> and current enum tags must be identical, including
    // when embedded before another field rather than at the end of a frame.
    let catalog = ic_wasm_store_protocol::preservation::Catalog {
        quota: ic_wasm_store_protocol::Quota {
            reserved_bytes: 8,
            releases: 2,
        },
        reserved_bytes: 8,
        releases: 1,
        artifacts: 1,
        chunks: 1,
    };
    let old = Some((
        catalog,
        None::<ic_wasm_store_protocol::Release>,
        0_u64,
        [7_u8; 32],
        false,
    ));
    let input = (old, 42_u8);
    let bytes = ic_wire::encode(&input, ic_wire::Encoding::Payload, LIMITS).unwrap();
    let (scan, tail): (Scan, u8) =
        ic_wire::decode(&bytes, ic_wire::Encoding::Payload, LIMITS).unwrap();
    assert!(matches!(scan, Scan::Legacy(Some(_))));
    assert_eq!(tail, 42);
    assert_eq!(
        ic_wire::encode(&(scan.clone(), tail), ic_wire::Encoding::Payload, LIMITS).unwrap(),
        bytes
    );
    let entries = Scan::Entries(scan.as_ref().cloned());
    let bytes =
        ic_wire::encode(&(entries.clone(), tail), ic_wire::Encoding::Payload, LIMITS).unwrap();
    assert_eq!(
        ic_wire::decode::<(Scan, u8)>(&bytes, ic_wire::Encoding::Payload, LIMITS).unwrap(),
        (entries, tail)
    );
    type Historical = Option<(
        ic_wasm_store_protocol::preservation::Catalog,
        Option<ic_wasm_store_protocol::Release>,
        u64,
        [u8; 32],
        bool,
    )>;
    assert!(
        ic_wire::decode::<(Historical, u8)>(&bytes, ic_wire::Encoding::Payload, LIMITS).is_err()
    );
    for bytes in [&[3_u8][..], &[2, 2][..], &[2][..]] {
        assert!(ic_wire::decode::<Scan>(bytes, ic_wire::Encoding::Payload, LIMITS).is_err());
    }
    let mut invalid = scan.as_ref().unwrap().clone();
    invalid.after_removed = true;
    assert!(
        ic_wire::encode(
            &Scan::Legacy(Some(invalid)),
            ic_wire::Encoding::Payload,
            LIMITS
        )
        .is_err()
    );
    for scan in [Scan::Legacy(None), Scan::Entries(None)] {
        let bytes = ic_wire::encode(&scan, ic_wire::Encoding::Payload, LIMITS).unwrap();
        assert_eq!(
            ic_wire::decode::<Scan>(&bytes, ic_wire::Encoding::Payload, LIMITS).unwrap(),
            scan
        );
    }
}
