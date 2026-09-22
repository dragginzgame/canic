use super::*;

fn reply() -> BlocksReply {
    let operator = Principal::self_authenticating(b"operator");
    let destination = Principal::from_slice(&[1, 1]);
    let mut memo = Vec::new();
    ciborium::ser::into_writer(
        &ciborium::value::Value::Array(vec![ciborium::value::Value::Bytes(
            destination.as_slice().to_vec(),
        )]),
        &mut memo,
    )
    .unwrap();
    let tx = Value::Map(BTreeMap::from([
        ("op".into(), Value::Text("burn".into())),
        (
            "from".into(),
            Value::Array(vec![Value::Blob(operator.as_slice().to_vec().into())]),
        ),
        ("amt".into(), Value::Nat(1000_u64.into())),
        ("memo".into(), Value::Blob(memo.into())),
    ]));
    BlocksReply {
        log_length: 9_u8.into(),
        blocks: vec![CyclesLedgerBlock {
            id: 8_u8.into(),
            block: Value::Map(BTreeMap::from([
                ("phash".into(), Value::Blob(vec![1; 32].into())),
                ("ts".into(), Value::Nat(99_u8.into())),
                ("fee".into(), Value::Nat(10_u8.into())),
                ("tx".into(), tx),
            ])),
        }],
    }
}

fn decode(reply: BlocksReply) -> Result<RetirementWithdrawalRecord, RetirementDebitError> {
    decode_block(
        reply,
        Principal::from_slice(&[2, 1]),
        Principal::self_authenticating(b"operator"),
        8,
        "root-key-hash".into(),
    )
}

#[test]
fn retirement_debit_decodes_exact_external_withdrawal() {
    let record = decode(reply()).unwrap();
    assert_eq!(record.amount_cycles, 1000);
    assert_eq!(record.fee_cycles, 10);
    assert_eq!(record.block_index, 8);
    assert_eq!(record.destination, Principal::from_slice(&[1, 1]).to_text());
    assert_eq!(record.block_sha256.len(), 64);
}

#[test]
fn retirement_debit_rejects_wrong_block_account_and_transaction() {
    for case in 0..11 {
        let mut r = reply();
        match case {
            0 => r.blocks[0].id = 7_u8.into(),
            1 => r.blocks.clear(),
            2 => r.blocks.push(r.blocks[0].clone()),
            3 => r.log_length = 8_u8.into(),
            _ => {
                let Value::Map(fields) = &mut r.blocks[0].block else {
                    unreachable!()
                };
                if case == 4 {
                    fields.insert("fee".into(), Value::Nat(0_u8.into()));
                } else {
                    let Some(Value::Map(tx)) = fields.get_mut("tx") else {
                        unreachable!()
                    };
                    match case {
                        5 => {
                            tx.insert("op".into(), Value::Text("mint".into()));
                        }
                        6 => {
                            tx.insert(
                                "from".into(),
                                Value::Array(vec![Value::Blob(vec![1].into())]),
                            );
                        }
                        7 => {
                            tx.insert("spender".into(), Value::Array(vec![]));
                        }
                        8 => {
                            tx.insert("amt".into(), Value::Nat(u128::MAX.into()));
                        }
                        9 => {
                            tx.insert("memo".into(), Value::Blob(vec![0x81, 0x41, 1, 0].into()));
                        }
                        10 => {
                            tx.insert("memo".into(), Value::Blob(vec![0; 33].into()));
                        }
                        _ => unreachable!(),
                    }
                }
            }
        }
        assert!(decode(r).is_err(), "case {case}");
    }
}
