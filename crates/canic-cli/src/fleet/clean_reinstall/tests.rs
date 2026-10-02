//! Continuations preserve selection without allowing different publication destinations.

use super::*;
use std::{ffi::OsString, path::PathBuf};

#[test]
fn omitted_inputs_inherit_retained_authority_and_explicit_inputs_must_match() {
    let policy = Path::new("/workspace/.tools/operator policy.toml");
    let seed = Path::new("/workspace/.tools/operator estate.toml");
    let mut options = EnsureOptions::parse(["ensure", "demo"].map(OsString::from)).unwrap();
    retain_inputs(Path::new("/workspace"), &mut options, policy, seed).unwrap();
    assert_eq!(options.source, policy);
    assert_eq!(options.seed, seed);
    options.explicit_inputs.source_explicit = true;
    options.explicit_inputs.seed_explicit = true;
    options.source = PathBuf::from(".tools/operator policy.toml");
    options.seed = PathBuf::from(".tools/operator estate.toml");
    retain_inputs(Path::new("/workspace"), &mut options, policy, seed).unwrap();
    options.source = PathBuf::from("wrong.toml");
    assert!(retain_inputs(Path::new("/workspace"), &mut options, policy, seed).is_err());
    options.source = policy.into();
    options.seed = PathBuf::from("wrong.toml");
    assert!(retain_inputs(Path::new("/workspace"), &mut options, policy, seed).is_err());
}
