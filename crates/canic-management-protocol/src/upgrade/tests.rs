use super::*;

#[test]
fn full_upgrade_history_fits_original_journal_budget() {
    let approval = Approval {
        manager: "a".repeat(63),
        child: "b".repeat(63),
        previous_module: [255; 32],
        module_hash: [254; 32],
        execute_at_ns: u64::MAX - 1,
        expires_at_ns: u64::MAX,
    };
    let completed = Status {
        approval: Some(approval),
        phase: Phase::Ready,
        attempts: [1; 2],
        requests: [Some([255; 32]); 2],
        previous_version: Some(u64::MAX - 1),
        observed_version: Some(u64::MAX),
        failure: None,
    };
    let creation = crate::Status {
        approval: Some(crate::Approval {
            manager: "a".repeat(63),
            module_hash: [255; 32],
            creation_cycles: u64::MAX,
            execute_at_ns: u64::MAX - 1,
            expires_at_ns: u64::MAX,
        }),
        phase: crate::Phase::Ready,
        worker: Some("b".repeat(63)),
        attempts: [1; 3],
        requests: [Some([255; 32]); 3],
        observed_module: Some(vec![255; 32]),
        creation_refund: Some(u128::MAX),
        ..Default::default()
    };
    let history = vec![completed.clone(); MAX_UPGRADES - 1];
    let limits = ic_wire::Limits {
        bytes: 8188,
        allocation: (1 + MAX_UPGRADES) * crate::LIMITS.allocation,
        nodes: (1 + MAX_UPGRADES) * crate::LIMITS.nodes,
        depth: crate::LIMITS.depth + 2,
    };
    let payload = (&creation, &completed, &history);
    let bytes = ic_wire::encode(&payload, ic_wire::Encoding::Payload, limits).unwrap();
    let decoded: (crate::Status, Status, Vec<Status>) =
        ic_wire::decode(&bytes, ic_wire::Encoding::Payload, limits).unwrap();
    assert_eq!(decoded.2.len(), MAX_UPGRADES - 1);
    let custody = crate::custody::Custody {
        creation,
        upgrades: vec![completed; MAX_UPGRADES],
    };
    let response = ic_wire::encode(
        &custody,
        ic_wire::Encoding::Frame {
            method: crate::custody::OBSERVE,
            direction: ic_wire::Direction::Response,
        },
        crate::custody::LIMITS,
    )
    .unwrap();
    assert_eq!(
        ic_wire::decode::<crate::custody::Custody>(
            &response,
            ic_wire::Encoding::Frame {
                method: crate::custody::OBSERVE,
                direction: ic_wire::Direction::Response
            },
            crate::custody::LIMITS
        )
        .unwrap(),
        custody
    );
}

#[test]
fn upgrade_binds_original_child_old_and_new_code_and_time() {
    let input = Approval {
        manager: "root".into(),
        child: "child".into(),
        previous_module: [1; 32],
        module_hash: [2; 32],
        execute_at_ns: 100,
        expires_at_ns: 200,
    };
    assert_eq!(input.validate("root", "child", &[1; 32], 99), Ok(()));
    assert_eq!(
        input.validate("other", "child", &[1; 32], 99),
        Err(Refusal::Binding)
    );
    assert_eq!(
        input.validate("root", "other", &[1; 32], 99),
        Err(Refusal::Binding)
    );
    assert_eq!(
        input.validate("root", "child", &[2; 32], 99),
        Err(Refusal::Binding)
    );
    for now in [100, 101, 199] {
        assert_eq!(input.validate("root", "child", &[1; 32], now), Ok(()));
    }
    for now in [200, 201, u64::MAX] {
        assert_eq!(
            input.validate("root", "child", &[1; 32], now),
            Err(Refusal::Time)
        );
    }
    let mut too_long = input.clone();
    too_long.expires_at_ns = 3_600_000_000_101;
    assert_eq!(
        too_long.validate("root", "child", &[1; 32], 3_600_000_000_100),
        Err(Refusal::Time)
    );
    let mut changed = input.clone();
    changed.previous_module = changed.module_hash;
    assert_eq!(
        changed.validate("root", "child", &changed.previous_module, 99),
        Err(Refusal::Binding)
    );
    let status = Status {
        approval: Some(input),
        phase: Phase::Ready,
        attempts: [1; 2],
        requests: [Some([3; 32]); 2],
        previous_version: Some(1),
        observed_version: Some(2),
        failure: None,
    };
    let bytes = ic_wire::encode(
        &status,
        ic_wire::Encoding::Frame {
            method: STATUS,
            direction: ic_wire::Direction::Response,
        },
        crate::LIMITS,
    )
    .unwrap();
    assert_eq!(
        ic_wire::decode::<Status>(
            &bytes,
            ic_wire::Encoding::Frame {
                method: STATUS,
                direction: ic_wire::Direction::Response
            },
            crate::LIMITS
        )
        .unwrap(),
        status
    );
}
