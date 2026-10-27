use super::*;
use canic_contracts::dto::fixture_provisioning::{FixtureChunkDescriptor, FixtureDescriptor};

fn source() -> RootStoreFixture {
    let descriptor = FixtureDescriptor {
        schema_version: 1,
        format_hash: [1; 32],
        encoded_length: 12,
        chunks: vec![
            FixtureChunkDescriptor {
                digest: [2; 32],
                length: 9,
            },
            FixtureChunkDescriptor {
                digest: [3; 32],
                length: 3,
            },
        ],
        completion_summary: [4; 32],
    };
    RootStoreFixture {
        role: canic_contracts::ids::CanisterRole::new("app"),
        content_id: FixtureContentApi::content_id(&descriptor).unwrap(),
        descriptor,
    }
}

#[test]
fn publication_reconciles_each_prefix_and_completed_replay() {
    let source = source();
    let expected = FixtureSourceStatus {
        content_id: source.content_id,
        next_chunk: 1,
        chunk_count: 2,
        received_bytes: 9,
        complete: false,
    };
    let empty = FixtureSourceStatus {
        next_chunk: 0,
        received_bytes: 0,
        ..expected
    };
    let complete = FixtureSourceStatus {
        next_chunk: 2,
        received_bytes: 12,
        complete: true,
        ..expected
    };
    for status in [&empty, &expected, &complete] {
        verify_source_status(&source, status).unwrap();
    }
    assert!(!upload_applied(&expected, 12, &empty).unwrap());
    assert!(upload_applied(&expected, 12, &expected).unwrap());
    assert!(upload_applied(&expected, 12, &complete).unwrap());
    for wrong in [
        FixtureSourceStatus {
            content_id: [0; 32],
            ..expected
        },
        FixtureSourceStatus {
            chunk_count: 3,
            ..expected
        },
        FixtureSourceStatus {
            next_chunk: 3,
            ..expected
        },
        FixtureSourceStatus {
            received_bytes: 8,
            ..expected
        },
        FixtureSourceStatus {
            complete: true,
            ..expected
        },
        FixtureSourceStatus {
            received_bytes: 11,
            ..complete
        },
    ] {
        std::assert_matches!(
            verify_source_status(&source, &wrong),
            Err(CurrentProtocolError::ResponseMismatch)
        );
        std::assert_matches!(
            upload_applied(&expected, 12, &wrong),
            Err(CurrentProtocolError::ResponseMismatch)
        );
    }
}

#[cfg(unix)]
fn native_response_cli(root: &std::path::Path) -> IcpCli {
    let executable = root.join("icp");
    ic_host_process::testing::write_executable_script(
        &executable,
        crate::test_support::tool_script(
            r#"#!/bin/sh
set -eu
case " $* " in
  *" --version "*) echo 'icp @ICP_VERSION@'; exit 0;;
esac
printf '%s\n' "$@" > arguments
while [ "$#" -gt 0 ]; do
  if [ "$1" = '--args-file' ]; then shift; cat "$1" > request.bin; fi
  shift
done
cat response.hex
"#,
        ),
    )
    .unwrap();
    IcpCli::new(executable.to_str().unwrap(), Some("sample".into())).with_cwd(root)
}

#[cfg(unix)]
#[test]
fn upload_uses_native_bytes_and_keeps_exact_progress_validation() {
    use std::fs;
    let root = crate::test_support::temp_dir("native-fixture-upload");
    fs::create_dir_all(&root).unwrap();
    let icp = native_response_cli(&root);
    let request = FixtureChunkUpload {
        content_id: [3; 32],
        index: 0,
        bytes: vec![1, 2, 3],
    };
    let expected = FixtureSourceStatus {
        content_id: request.content_id,
        next_chunk: 1,
        chunk_count: 1,
        received_bytes: 3,
        complete: true,
    };
    let action = CurrentFleetProtocolAction::PublishStoreFixtureChunk {
        maximum_attempts: 1,
        request: request.clone(),
        expected: expected.clone(),
        source_bytes: 3,
    };
    let resolved = ResolvedProtocolAction {
        action: &action,
        wire_schema_path: root.join("retained-action.wire.json"),
        wire_schema_bytes: crate::test_support::WIRE_SCHEMA.to_vec(),
        wire_schema_sha256: "retained-action-authority",
        target: ic_platform::principal::ExternalPrincipal::from_slice(&[1]),
    };
    let write_reply = |reply: Result<
        Result<FixtureSourceStatus, FixtureStoreError>,
        canic_contracts::dto::error::Error,
    >| {
        let frame = ic_wire::native::encode_response(
            protocol::fixture_bytes::upload::METHOD_ID,
            &reply,
            protocol::fixture_bytes::UPLOAD_RESPONSE_LIMITS,
        )
        .unwrap();
        fs::write(
            root.join("response.hex"),
            canic_core::cdk::utils::hash::hex_bytes(frame),
        )
        .unwrap();
    };
    write_reply(Ok(Ok(expected.clone())));
    assert_eq!(
        upload(&icp, &resolved, &request, &expected, 3).unwrap(),
        canic_core::cdk::utils::hash::wasm_hash(&request.bytes)
    );
    let bytes = fs::read(root.join("request.bin")).unwrap();
    let sent: canic_contracts::dto::fixture_bytes::UploadRequest = ic_wire::native::decode_request(
        protocol::fixture_bytes::upload::METHOD_ID,
        &bytes,
        protocol::fixture_bytes::CHUNK_LIMITS,
    )
    .unwrap();
    assert_eq!(FixtureChunkUpload::from(sent), request);
    let arguments = fs::read_to_string(root.join("arguments")).unwrap();
    assert!(arguments.lines().any(|arg| arg == "hex"));
    assert!(
        !arguments
            .lines()
            .any(|arg| arg == "--query" || arg == "--candid")
    );
    let mut wrong = expected.clone();
    wrong.content_id = [4; 32];
    write_reply(Ok(Ok(wrong)));
    assert!(matches!(
        upload(&icp, &resolved, &request, &expected, 3),
        Err(CurrentProtocolError::ResponseMismatch)
    ));
    write_reply(Ok(Err(FixtureStoreError::Conflict)));
    assert!(matches!(
        upload(&icp, &resolved, &request, &expected, 3),
        Err(CurrentProtocolError::Fixture(FixtureStoreError::Conflict))
    ));
    fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn preparation_observation_uses_native_query_and_preserves_uncertainty() {
    use std::fs;
    let root = crate::test_support::temp_dir("native-root-fixture-observation");
    fs::create_dir_all(&root).unwrap();
    let icp = native_response_cli(&root);
    let source = source();
    let action = CurrentFleetProtocolAction::PrepareStoreFixture {
        maximum_attempts: 1,
        request: RootStoreFixturePrepareRequest {
            bootstrap: RootStoreBootstrapRequest {
                operation_id: [5; 32],
                manifest_payload_size_bytes: 12,
            },
            role: source.role.clone(),
        },
        source: source.clone(),
        store: ic_platform::principal::ExternalPrincipal::from_slice(&[2]),
    };
    let resolved = ResolvedProtocolAction {
        action: &action,
        wire_schema_path: root.join("retained-action.wire.json"),
        wire_schema_bytes: crate::test_support::WIRE_SCHEMA.to_vec(),
        wire_schema_sha256: "retained-action-authority",
        target: ic_platform::principal::ExternalPrincipal::from_slice(&[1]),
    };
    let expected = FixtureSourceStatus {
        content_id: source.content_id,
        next_chunk: 1,
        chunk_count: 2,
        received_bytes: 9,
        complete: false,
    };
    let write_reply = |reply: Result<
        Result<FixtureSourceStatus, FixtureStoreError>,
        canic_contracts::dto::error::Error,
    >| {
        let frame = ic_wire::native::encode_response(
            protocol::root_store_reporting::fixture::METHOD_ID,
            &reply,
            protocol::root_store_reporting::RESPONSE_LIMITS,
        )
        .unwrap();
        fs::write(
            root.join("response.hex"),
            canic_core::cdk::utils::hash::hex_bytes(frame),
        )
        .unwrap();
    };
    write_reply(Ok(Ok(expected.clone())));
    assert!(
        observe_preparation(&icp, &resolved, &source)
            .unwrap()
            .applied
    );
    let frame = fs::read(root.join("request.bin")).unwrap();
    let content: [u8; 32] = ic_wire::native::decode_request(
        protocol::root_store_reporting::fixture::METHOD_ID,
        &frame,
        protocol::root_store_reporting::REQUEST_LIMITS,
    )
    .unwrap();
    assert_eq!(content, source.content_id);
    let args = fs::read_to_string(root.join("arguments")).unwrap();
    assert!(args.lines().any(|arg| arg == "--query"));
    assert!(!args.lines().any(|arg| arg == "--candid"));
    write_reply(Ok(Err(FixtureStoreError::NotFound)));
    assert!(
        !observe_preparation(&icp, &resolved, &source)
            .unwrap()
            .applied
    );
    let mut wrong = expected;
    wrong.content_id = [0; 32];
    write_reply(Ok(Ok(wrong)));
    assert!(matches!(
        observe_preparation(&icp, &resolved, &source),
        Err(CurrentProtocolError::ResponseMismatch)
    ));
    write_reply(Ok(Err(FixtureStoreError::Authority)));
    assert!(matches!(
        observe_preparation(&icp, &resolved, &source),
        Err(CurrentProtocolError::Fixture(FixtureStoreError::Authority))
    ));
    fs::remove_dir_all(root).unwrap();
}
