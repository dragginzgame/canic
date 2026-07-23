use candid::Principal;
use canic::{
    Error,
    api::call::{Call, CallBuilder, CallResult},
};

#[test]
fn public_call_builder_supports_maintained_construction_paths() {
    let target = Principal::anonymous();

    let single: CallBuilder<'_> = Call::bounded_wait(target, "single")
        .with_arg(7_u64)
        .expect("encode one argument")
        .with_cycles(1_000);
    drop(single);

    let tuple: CallBuilder<'_> = Call::unbounded_wait(target, "tuple")
        .with_args(("alpha", 9_u64))
        .expect("encode argument tuple");
    drop(tuple);

    let raw: CallBuilder<'_> =
        Call::bounded_wait(target, "raw").with_raw_args(vec![68, 73, 68, 76, 0, 0]);
    drop(raw);
}

#[test]
fn prelude_exposes_the_canonical_call_entrypoint() {
    use canic::prelude::Call;

    let builder: CallBuilder<'_> = Call::bounded_wait(Principal::anonymous(), "prelude");
    drop(builder);
}

#[test]
fn public_call_result_has_typed_candid_decoders() {
    fn assert_decoder_contract(result: &CallResult) {
        let _: Result<u64, Error> = result.candid();
        let _: Result<(u64, String), Error> = result.candid_tuple();
    }

    let _: fn(&CallResult) = assert_decoder_contract;
}

#[test]
fn public_call_builder_combines_execution_with_typed_candid_decoding() {
    let single = Call::bounded_wait(Principal::anonymous(), "single").execute_candid::<u64>();
    drop(single);

    let tuple =
        Call::bounded_wait(Principal::anonymous(), "tuple").execute_candid_tuple::<(u64, String)>();
    drop(tuple);
}
