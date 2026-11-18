use super::{METHOD, REQUEST_LIMITS, RESPONSE_LIMITS, State};
use ic_wire::{Direction, Encoding};

#[test]
fn bounded_states_remain_distinct_and_are_not_requests() {
    let response = Encoding::Frame {
        method: METHOD,
        direction: Direction::Response,
    };
    let request = Encoding::Frame {
        method: METHOD,
        direction: Direction::Request,
    };
    for state in [State::Unavailable, State::Fenced, State::Ready] {
        let bytes = ic_wire::encode(&state, response, RESPONSE_LIMITS).unwrap();
        assert_eq!(
            ic_wire::decode::<State>(&bytes, response, RESPONSE_LIMITS).unwrap(),
            state
        );
        assert!(ic_wire::decode::<State>(&bytes, request, RESPONSE_LIMITS).is_err());
        let claimed = ic_wire::encode(&state, request, REQUEST_LIMITS).unwrap();
        assert!(ic_wire::decode::<()>(&claimed, request, REQUEST_LIMITS).is_err());
    }
}
