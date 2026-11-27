use super::*;

fn request() -> Request {
    let args = management::UploadDataArgs {
        canister_id: "rrkah-fqaaa-aaaaa-aaaaq-cai".parse().unwrap(),
        snapshot_id: vec![1; 32],
        chunk: vec![255; 1_048_576],
        kind: management::WriteKind::WasmMemory(management::Offset { offset: 0 }),
    };
    let bytes = args.encode(MAX_INNER).unwrap();
    Request {
        operation: Operation {
            id: [1; 16],
            kind: Kind::Restore,
            approval: [2; 32],
            network: [3; 32],
            manager: "r7inp-6aaaa-aaaaa-aaabq-cai".into(),
            caller: "2vxsx-fae".into(),
            generation: 1,
            target: args.canister_id.to_text(),
        },
        step: Step::Upload(300),
        effect: Effect::Upload(args),
        arguments_hash: digest(&bytes),
        arguments_bytes: bytes.len() as u32,
        expires_at_ns: 100,
        cycles: 0,
        cycle_limit: MAX_CYCLES,
        reply_bytes: 1024,
    }
}

#[test]
fn full_extent_fits_wire_and_preserves_exact_platform_bytes() {
    let request = request();
    let encoded = ic_wire::encode(
        &request,
        ic_wire::Encoding::Frame {
            method: EXECUTE,
            direction: ic_wire::Direction::Request,
        },
        LIMITS,
    )
    .unwrap();
    assert!(encoded.len() < 2 * 1_048_576);
    let decoded: Request = ic_wire::decode(
        &encoded,
        ic_wire::Encoding::Frame {
            method: EXECUTE,
            direction: ic_wire::Direction::Request,
        },
        LIMITS,
    )
    .unwrap();
    assert_eq!(decoded, request);
    let (binding, bytes) = decoded.prepare().unwrap();
    assert_eq!(binding.arguments_hash, digest(&bytes));
    assert_eq!(binding.arguments_bytes, bytes.len() as u32);
}

#[test]
fn changed_bytes_target_kind_and_bounds_refuse_before_dispatch() {
    let original = request();
    for case in 0..6 {
        let mut r = original.clone();
        match case {
            0 => r.arguments_hash = [0; 32],
            1 => r.arguments_bytes += 1,
            2 => r.operation.target = "aaaaa-aa".into(),
            3 => r.operation.kind = Kind::Capture,
            4 => r.reply_bytes = MAX_INNER as u32 + 1,
            _ => r.cycles = MAX_CYCLES + 1,
        }
        assert_eq!(r.prepare(), Err(Error::Invalid));
    }
}

#[test]
fn only_original_in_budget_inner_results_are_importable() {
    let (binding, _) = request().prepare().unwrap();
    let encode = |record: &Record| {
        ic_wire::encode(
            &Ok::<_, Error>(record),
            ic_wire::Encoding::Frame {
                method: EXECUTE,
                direction: ic_wire::Direction::Response,
            },
            LIMITS,
        )
        .unwrap()
    };
    let mut record = Record {
        binding: binding.clone(),
        outcome: Outcome::Rejected(5, "original".into()),
    };
    let bytes = encode(&record);
    assert_eq!(
        Exchange::Execute.decode_response(&binding, &bytes).unwrap(),
        Ok(record.clone())
    );
    assert!(matches!(
        Exchange::Lookup.decode_response(&binding, &bytes),
        Err(EvidenceError::Wire(_))
    ));
    record.binding.expires_at_ns += 1;
    assert!(matches!(
        Exchange::Execute.decode_response(&binding, &encode(&record)),
        Err(EvidenceError::Binding)
    ));
    record.binding = binding.clone();
    for outcome in [
        Outcome::Reply(vec![0; 1025]),
        Outcome::Rejected(5, "a".repeat(1025)),
    ] {
        record.outcome = outcome;
        assert!(matches!(
            Exchange::Execute.decode_response(&binding, &encode(&record)),
            Err(EvidenceError::Size)
        ));
    }
}
