use super::*;

const LIMITS: ArgumentLimits = ArgumentLimits {
    max_bytes: 1024,
    decoding_quota: 10_000,
    skipping_quota: 100,
    max_type_len: 16,
    max_header_len: 128,
};

#[test]
fn argument_limits_accept_valid_input_and_bound_each_decoder_dimension() {
    let bytes = candid::encode_args(("hello",)).unwrap();
    assert_eq!(LIMITS.decode::<(String,)>(&bytes).unwrap().0, "hello");
    let exact = ArgumentLimits {
        max_bytes: bytes.len(),
        ..LIMITS
    };
    assert!(exact.decode::<(String,)>(&bytes).is_ok());
    let short = ArgumentLimits {
        max_bytes: bytes.len() - 1,
        ..LIMITS
    };
    assert_eq!(
        short.decode::<(String,)>(&bytes),
        Err(ArgumentDecodeError::TooLarge {
            actual: bytes.len(),
            maximum: bytes.len() - 1,
        })
    );
    for bad in [b"invalid".as_slice(), &bytes[..bytes.len() - 1]] {
        assert_eq!(
            LIMITS.decode::<(String,)>(bad),
            Err(ArgumentDecodeError::InvalidCandid)
        );
    }
    let work = ArgumentLimits {
        decoding_quota: 1,
        ..LIMITS
    };
    assert_eq!(
        work.decode::<(String,)>(&bytes),
        Err(ArgumentDecodeError::InvalidCandid)
    );

    // Two nested vector types: reject the type table independently of value work.
    let types = candid::encode_args((Vec::<Vec<u8>>::new(),)).unwrap();
    assert!(LIMITS.decode::<(Vec<Vec<u8>>,)>(&types).is_ok());
    let one_type = ArgumentLimits {
        max_type_len: 1,
        ..LIMITS
    };
    assert_eq!(
        one_type.decode::<(Vec<Vec<u8>>,)>(&types),
        Err(ArgumentDecodeError::InvalidCandid)
    );
    let header = ArgumentLimits {
        max_header_len: 1,
        ..LIMITS
    };
    assert_eq!(
        header.decode::<(Vec<Vec<u8>>,)>(&types),
        Err(ArgumentDecodeError::InvalidCandid)
    );

    // A compact zero-byte-element sequence still consumes skipping work.
    let skipped = candid::encode_args((Vec::<()>::from([(); 40]),)).unwrap();
    let ample = ArgumentLimits {
        skipping_quota: 10_000,
        decoding_quota: 100_000,
        ..LIMITS
    };
    assert_eq!(ample.decode::<()>(&skipped), Ok(()));
    let no_skip = ArgumentLimits {
        skipping_quota: 0,
        ..ample
    };
    assert_eq!(
        no_skip.decode::<()>(&skipped),
        Err(ArgumentDecodeError::InvalidCandid)
    );
}

#[test]
fn bounded_zero_argument_envelopes_reject_malformed_or_excessive_trailing_data() {
    assert_eq!(LIMITS.decode::<()>(&[]), Ok(()));
    assert_eq!(LIMITS.decode::<()>(b"DIDL\x00\x00"), Ok(()));
    assert_eq!(
        LIMITS.decode::<()>(b"DIDL\x00\x00\xff"),
        Err(ArgumentDecodeError::InvalidCandid)
    );
    // A record declaring billions of fields must reject within the header bound.
    assert_eq!(
        LIMITS.decode::<()>(b"DIDL\x01\x6c\xff\xff\xff\xff\x0f"),
        Err(ArgumentDecodeError::InvalidCandid)
    );
}
