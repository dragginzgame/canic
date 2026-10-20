//! Native qualification of the generated Candid codec boundary, without IC calls.

#[canic::canic_query(public)]
fn codec_scalar(value: u64) -> Result<u64, canic::Error> {
    Ok(value)
}

#[canic::canic_query(public)]
fn codec_pair(value: u64, label: String) -> Result<(u64, String), canic::Error> {
    Ok((value, label))
}

#[canic::canic_query(public)]
fn codec_empty() -> Result<(), canic::Error> {
    Ok(())
}

#[test]
fn generated_codecs_preserve_candid_argument_and_reply_shapes() {
    use ic_platform::candid::contract::{encode_args, encode_one};
    assert_eq!(__canic_decode_codec_scalar(encode_one(42_u64).unwrap()), 42);
    assert_eq!(
        __canic_decode_codec_pair(encode_args((42_u64, "label")).unwrap()),
        (42, "label".to_string())
    );
    __canic_decode_codec_empty(encode_args(()).unwrap());
    assert_eq!(
        __canic_encode_codec_scalar(Ok(42)),
        encode_one(Ok::<u64, canic::Error>(42)).unwrap()
    );
    assert_eq!(
        __canic_encode_codec_pair(Ok((42, "label".into()))),
        encode_one(Ok::<_, canic::Error>((42_u64, "label"))).unwrap()
    );
    assert_eq!(
        __canic_encode_codec_empty(Ok(())),
        encode_one(Ok::<(), canic::Error>(())).unwrap()
    );
}
