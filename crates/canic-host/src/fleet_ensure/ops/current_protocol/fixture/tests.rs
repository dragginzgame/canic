use super::*;
use canic_core::dto::fixture_provisioning::{FixtureChunkDescriptor, FixtureDescriptor};

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
        role: canic_core::ids::CanisterRole::new("app"),
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
