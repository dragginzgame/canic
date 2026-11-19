//! Installed fixed greeting and optional metrics composition, with no persistent owner.
use canic::{metrics, readiness};
use canic_hello::{GREET, Greeting, LIMITS};
use ic_testkit::pocket_ic::{CanisterId, nonblocking::PocketIc};
use ic_wire::{Direction, Encoding};

fn greeting_request(method: u64, direction: Direction, name: &str) -> Vec<u8> {
    ic_wire::encode(&name, Encoding::Frame { method, direction }, LIMITS).unwrap()
}

pub async fn check(pic: &PocketIc, canister: CanisterId, greeting: &str) {
    let caller = CanisterId::self_authenticating(b"demo visitor");
    check_readiness(pic, canister, caller).await;
    let name = "é".repeat(64); // Exactly 128 UTF-8 bytes.
    for method in ["greet", "probe"] {
        let bytes = greeting_request(GREET, Direction::Request, &name);
        let reply = if method == "greet" {
            pic.query_call(canister, caller, method, bytes).await
        } else {
            pic.update_call(canister, caller, method, bytes).await
        }
        .unwrap();
        let result: Greeting = ic_wire::decode(
            &reply,
            Encoding::Frame {
                method: GREET,
                direction: Direction::Response,
            },
            LIMITS,
        )
        .unwrap();
        assert_eq!(result.message, format!("{greeting}, {name}!"));
        for invalid in [
            vec![],
            vec![0; LIMITS.bytes + 1],
            greeting_request(GREET + 1, Direction::Request, "visitor"),
            greeting_request(GREET, Direction::Response, "visitor"),
            greeting_request(GREET, Direction::Request, &(name.clone() + "!")),
        ] {
            let result = if method == "greet" {
                pic.query_call(canister, caller, method, invalid).await
            } else {
                pic.update_call(canister, caller, method, invalid).await
            };
            assert!(result.is_err(), "{method} accepted invalid input");
        }
    }
    let request = ic_wire::encode(
        &(),
        Encoding::Frame {
            method: metrics::METHOD,
            direction: Direction::Request,
        },
        metrics::REQUEST_LIMITS,
    )
    .unwrap();
    for caller in [caller, CanisterId::anonymous()] {
        let reply = pic
            .query_call(canister, caller, "metrics", request.clone())
            .await
            .unwrap();
        let snapshot: metrics::Snapshot = ic_wire::decode(
            &reply,
            Encoding::Frame {
                method: metrics::METHOD,
                direction: Direction::Response,
            },
            metrics::RESPONSE_LIMITS,
        )
        .unwrap();
        assert!(snapshot.observed_at_ns > 0);
        assert_eq!(snapshot.timers, None, "no provider is not zero counters");
    }
    for invalid in [
        vec![],
        vec![0; metrics::REQUEST_LIMITS.bytes + 1],
        greeting_request(GREET, Direction::Request, "visitor"),
    ] {
        assert!(
            pic.query_call(canister, caller, "metrics", invalid)
                .await
                .is_err()
        );
    }
    assert!(
        pic.get_stable_memory(canister).await.is_empty(),
        "mounting created stable state"
    );
}

async fn check_readiness(pic: &PocketIc, canister: CanisterId, caller: CanisterId) {
    let request_frame = Encoding::Frame {
        method: readiness::METHOD,
        direction: Direction::Request,
    };
    let response_frame = Encoding::Frame {
        method: readiness::METHOD,
        direction: Direction::Response,
    };
    let request = ic_wire::encode(&(), request_frame, readiness::REQUEST_LIMITS).unwrap();
    let mut invalid = vec![
        vec![],
        vec![0; readiness::REQUEST_LIMITS.bytes + 1],
        request[..request.len() - 1].to_vec(),
        ic_wire::encode(&(), response_frame, readiness::REQUEST_LIMITS).unwrap(),
        greeting_request(GREET, Direction::Request, "visitor"),
    ];
    for claimed in [
        readiness::State::Unavailable,
        readiness::State::Fenced,
        readiness::State::Ready,
    ] {
        invalid.push(ic_wire::encode(&claimed, request_frame, readiness::REQUEST_LIMITS).unwrap());
    }
    for caller in [caller, CanisterId::anonymous()] {
        for invalid in &invalid {
            assert!(
                pic.update_call(canister, caller, "readiness", invalid.clone())
                    .await
                    .is_err()
            );
            // An invalid observation cannot replace the application's actual state.
            let reply = pic
                .update_call(canister, caller, "readiness", request.clone())
                .await
                .unwrap();
            assert_eq!(
                ic_wire::decode::<readiness::State>(
                    &reply,
                    response_frame,
                    readiness::RESPONSE_LIMITS,
                )
                .unwrap(),
                readiness::State::Ready
            );
        }
    }
}
