//! Actual Host drain routing: exact Wire bytes, reviewed schema and one attempt per call.

use super::*;
use crate::fleet_ensure::tests::drain_schema;
use canic_contracts::{diagnostics::codes, dto::error::Error};
use std::fs;

struct DrainFixture {
    owners: ProtocolOwnersFixture,
    schema_sha256: String,
}

impl DrainFixture {
    fn new() -> Self {
        let owners = ProtocolOwnersFixture::new();
        fs::write(
            owners.root.join("icp"),
            crate::test_support::tool_script(
                r#"#!/bin/sh
set -eu
case " $* " in *" --version "*) echo 'icp @ICP_VERSION@'; exit 0;; esac
printf '%s\n' "$*" >> drain-calls
while [ "$#" -gt 0 ]; do
  if [ "$1" = '--args-file' ]; then shift; cp "$1" drain-request; fi
  shift
done
if [ -f drain-failure ]; then exit 17; fi
cat drain-reply
"#,
            ),
        )
        .unwrap();
        let schema = drain_schema();
        fs::write(owners.root.join("drain.wire.json"), &schema).unwrap();
        Self {
            owners,
            schema_sha256: canic_core::cdk::utils::hash::sha256_hex(&schema),
        }
    }

    fn reply(&self, id: u64, response: Result<DrainResponse, Error>) {
        let bytes =
            ic_wire::native::encode_response(id, &response, drain_wire::RESPONSE_LIMITS).unwrap();
        fs::write(self.owners.root.join("drain-reply"), hex_bytes(bytes)).unwrap();
    }

    fn call(&self) -> Result<EffectOutcome, IcpEnsurePlatformError> {
        self.owners.platform.apply_transfer(
            u128::MAX,
            "drain.wire.json",
            &self.schema_sha256,
            "rrkah-fqaaa-aaaaa-aaaaq-cai",
            drain_wire::METHOD,
            "original-operation",
            "ryjl3-tyaaa-aaaaa-aaaba-cai",
        )
    }

    fn calls(&self) -> usize {
        fs::read_to_string(self.owners.root.join("drain-calls"))
            .unwrap_or_default()
            .lines()
            .count()
    }
}

#[test]
fn drain_transfer_preserves_original_wire_authority_and_typed_receipts() {
    let fixture = DrainFixture::new();
    for reply in [
        DrainResponse::Accepted {
            transferred_cycles: u128::MAX,
        },
        DrainResponse::Replayed {
            transferred_cycles: u128::MAX,
        },
    ] {
        fixture.reply(drain_wire::METHOD_ID, Ok(reply));
        assert_eq!(
            fixture.call().unwrap().receipt.as_deref(),
            Some("original-operation")
        );
        let request: DrainRequest = ic_wire::native::decode_request(
            drain_wire::METHOD_ID,
            &fs::read(fixture.owners.root.join("drain-request")).unwrap(),
            drain_wire::REQUEST_LIMITS,
        )
        .unwrap();
        assert_eq!(request.operation_id, "original-operation");
        assert_eq!(request.cycles, u128::MAX);
        assert_eq!(request.destination.to_text(), "rrkah-fqaaa-aaaaa-aaaaq-cai");
    }
    let arguments = fs::read_to_string(fixture.owners.root.join("drain-calls")).unwrap();
    assert!(
        arguments
            .lines()
            .all(|line| line.contains("canic_cycle_drain")
                && line.contains("--args-file")
                && !line.contains("--candid")
                && !line.contains("--query"))
    );
    fixture.reply(
        drain_wire::METHOD_ID,
        Ok(DrainResponse::Accepted {
            transferred_cycles: 7,
        }),
    );
    assert!(matches!(
        fixture.call(),
        Err(IcpEnsurePlatformError::WrongTransferAmount {
            actual: 7,
            expected: u128::MAX
        })
    ));
    let refusal = Error::from_registered(codes::AUTHORITY_UNAVAILABLE);
    fixture.reply(drain_wire::METHOD_ID, Err(refusal));
    assert!(
        matches!(fixture.call(), Err(IcpEnsurePlatformError::DrainRejected(error)) if error == refusal)
    );
    fixture.reply(
        drain_wire::METHOD_ID + 1,
        Ok(DrainResponse::Accepted {
            transferred_cycles: u128::MAX,
        }),
    );
    assert!(matches!(
        fixture.call(),
        Err(IcpEnsurePlatformError::DrainWire(
            crate::icp::IcpWireQueryError::Wire(ic_wire::Error::Identity)
        ))
    ));
    let calls = fixture.calls();
    fs::write(fixture.owners.root.join("drain-failure"), []).unwrap();
    assert!(matches!(
        fixture.call(),
        Err(IcpEnsurePlatformError::DrainWire(_))
    ));
    assert_eq!(fixture.calls(), calls + 1);
    fs::remove_dir_all(fixture.owners.root).unwrap();
}

#[test]
fn drain_schema_change_or_wrong_contract_refuses_before_transport() {
    let mut fixture = DrainFixture::new();
    fs::write(fixture.owners.root.join("drain.wire.json"), b"{}").unwrap();
    assert!(matches!(
        fixture.call(),
        Err(IcpEnsurePlatformError::ArtifactDigestMismatch { .. })
    ));
    fixture.schema_sha256 = canic_core::cdk::utils::hash::sha256_hex(b"{}");
    assert!(matches!(
        fixture.call(),
        Err(IcpEnsurePlatformError::CurrentProtocol(
            current_protocol::CurrentProtocolError::WireSchema(_)
        ))
    ));
    assert_eq!(fixture.calls(), 0);
    fs::remove_dir_all(fixture.owners.root).unwrap();
}
