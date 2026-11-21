//! Preserve the observed stage of one typed submission.
use super::{CallError, Contract, actor::decode_result};
use ic_platform_runtime::call::{Completion, SubmitError};

struct Echo;
impl Contract for Echo {
    const NAME: &'static str = "echo";
    const ID: u64 = 42;
    type Input = ();
    type Output = u64;
}

#[test]
fn typed_result_preserves_dispatch_rejection_uncertainty_and_invalid_reply() {
    assert!(matches!(
        decode_result::<Echo>(Err(SubmitError::CallbackCapacity)),
        Err(CallError::Dispatch(SubmitError::CallbackCapacity))
    ));
    for code in [4u32, 6] {
        let result = Completion::from_rejection(code.try_into().unwrap(), "original".into(), None);
        let reason = match (code, decode_result::<Echo>(Ok(result))) {
            (4, Err(CallError::Rejected(reason))) | (6, Err(CallError::Unknown(reason))) => reason,
            other => panic!("unexpected classification: {other:?}"),
        };
        assert_eq!(reason.code(), code);
        assert_eq!(reason.message(), "original");
    }
    assert!(matches!(
        decode_result::<Echo>(Ok(Completion::from_reply(vec![], None))),
        Err(CallError::Reply(_))
    ));
}
