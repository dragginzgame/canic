use super::*;
use flate2::{Compression, write::GzEncoder};
use std::io::Write;

fn gzip(bytes: &[u8]) -> Vec<u8> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(bytes).unwrap();
    encoder.finish().unwrap()
}

#[test]
fn qualified_facts_bind_both_exact_representations() {
    let wasm = b"\0asm\x01\0\0\0";
    let compressed = gzip(wasm);
    let facts = qualify_representation(wasm, &compressed).unwrap();
    assert_eq!(facts.wasm_size_bytes, wasm.len() as u64);
    assert_eq!(facts.wasm_gz_size_bytes, compressed.len() as u64);
    assert_eq!(facts.wasm_sha256_hex, sha256_hex(wasm));
    assert_eq!(facts.wasm_gz_sha256_hex, sha256_hex(&compressed));
}

#[test]
fn empty_invalid_and_truncated_representations_have_typed_failures() {
    let wasm = b"\0asm\x01\0\0\0";
    std::assert_matches!(
        qualify_representation(&[], &[]),
        Err(RepresentationError::EmptyArtifact { kind: "raw Wasm" })
    );
    std::assert_matches!(
        qualify_representation(b"invalid", &[]),
        Err(RepresentationError::InvalidWasm)
    );
    std::assert_matches!(
        qualify_representation(wasm, &[]),
        Err(RepresentationError::EmptyArtifact { kind: "gzip Wasm" })
    );
    std::assert_matches!(
        qualify_representation(wasm, b"invalid"),
        Err(RepresentationError::InvalidGzip { .. })
    );
    let mut compressed = gzip(wasm);
    compressed.truncate(compressed.len() / 2);
    std::assert_matches!(
        qualify_representation(wasm, &compressed),
        Err(RepresentationError::InvalidGzip { .. })
    );
}

#[test]
fn decoded_difference_and_expansion_beyond_raw_size_are_rejected() {
    let wasm = b"\0asm\x01\0\0\0";
    for different in [b"different".to_vec(), vec![0; 1_000_000]] {
        std::assert_matches!(
            qualify_representation(wasm, &gzip(&different)),
            Err(RepresentationError::RepresentationMismatch)
        );
    }
}

#[test]
fn shorter_and_equal_length_differences_are_mismatches() {
    let wasm = b"\0asm\x01\0\0\0";
    for different in [b"short".as_slice(), b"\0asm\x01\0\0\x01".as_slice()] {
        std::assert_matches!(
            qualify_representation(wasm, &gzip(different)),
            Err(RepresentationError::RepresentationMismatch)
        );
    }
}

#[test]
fn a_content_mismatch_does_not_hide_a_corrupt_trailer() {
    let wasm = b"\0asm\x01\0\0\0";
    let mut compressed = gzip(b"\0asm\x01\0\0\x01");
    let crc_offset = compressed.len() - 8;
    compressed[crc_offset] ^= 1;
    std::assert_matches!(
        qualify_representation(wasm, &compressed),
        Err(RepresentationError::InvalidGzip { .. })
    );
}

#[test]
fn exact_raw_match_does_not_admit_trailing_data_or_an_extra_member() {
    let wasm = b"\0asm\x01\0\0\0";
    for trailing in [b"trailing".to_vec(), gzip(&[])] {
        let mut compressed = gzip(wasm);
        compressed.extend(trailing);
        std::assert_matches!(
            qualify_representation(wasm, &compressed),
            Err(RepresentationError::InvalidGzip { .. })
        );
    }
}
