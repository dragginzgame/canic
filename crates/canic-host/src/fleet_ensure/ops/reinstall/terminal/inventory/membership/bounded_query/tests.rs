//! Exercise transport deadlines and Candid budgets without management effects.

use super::*;
use canic_core::diagnostics::codes;
use std::future::{pending, ready};

fn limits() -> QueryLimits {
    QueryLimits {
        timeout: Duration::from_millis(10),
        response_bytes: 1024,
    }
}

fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .unwrap()
}

#[test]
fn decodes_response_and_preserves_typed_rejection() {
    runtime().block_on(async {
        let bytes = candid::encode_one(Ok::<u64, Error>(42)).unwrap();
        assert_eq!(
            bounded_response::<u64>(ready(Ok(bytes)), limits())
                .await
                .unwrap(),
            42
        );
        let reason = Error::from_registered(codes::EVIDENCE_UNAVAILABLE);
        let bytes = candid::encode_one(Err::<u64, Error>(reason)).unwrap();
        let Err(QueryError::Rejected(observed)) =
            bounded_response::<u64>(ready(Ok(bytes)), limits()).await
        else {
            panic!("expected typed rejection");
        };
        assert_eq!(*observed, reason);
    });
}

#[test]
fn malformed_and_over_budget_responses_fail_decoding() {
    runtime().block_on(async {
        std::assert_matches!(
            bounded_response::<u64>(ready(Ok(vec![0])), limits()).await,
            Err(QueryError::Decode(_))
        );
        let bytes = candid::encode_one(Ok::<Vec<u64>, Error>(vec![42; 1000])).unwrap();
        let tight = QueryLimits {
            response_bytes: 1,
            ..limits()
        };
        std::assert_matches!(
            bounded_response::<Vec<u64>>(ready(Ok(bytes)), tight).await,
            Err(QueryError::Decode(_))
        );
    });
}

#[test]
fn stalled_query_expires_and_transport_failure_stays_typed() {
    runtime().block_on(async {
        std::assert_matches!(
            bounded_response::<u64>(pending(), limits()).await,
            Err(QueryError::Expired)
        );
        std::assert_matches!(
            bounded_response::<u64>(
                ready(Err(AgentError::TimeoutWaitingForResponse())),
                limits()
            )
            .await,
            Err(QueryError::Query(_))
        );
    });
}
