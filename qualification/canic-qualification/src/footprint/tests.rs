use super::measure;

#[test]
fn complete_metrics_and_malformed_or_oversize_refusal() {
    // () -> () function, no imports, one empty body.
    let module = b"\0asm\x01\0\0\0\x01\x04\x01\x60\0\0\x03\x02\x01\0\x0a\x04\x01\x02\0\x0b";
    let facts = measure(module).unwrap();
    assert_eq!(facts.raw_bytes, module.len());
    assert_eq!(facts.defined_functions, 1);
    assert_eq!(facts.code_section_bytes, 4);
    assert!(measure(&module[..module.len() - 1]).is_err());
    assert!(measure(&vec![0; 1_900_001]).is_err());
    let mut invalid = module.to_vec();
    *invalid.last_mut().unwrap() = 0xff;
    assert!(measure(&invalid).is_err());
}
