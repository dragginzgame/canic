use super::*;

#[test]
fn cargo_paths_preserve_escaped_names_and_continuations() {
    assert_eq!(
        dependency_paths("/target/role.wasm: /source/lib.rs /outside/with\\ space.txt \\\n/outside/$$value\\#1.txt\n").unwrap(),
        ["/source/lib.rs", "/outside/with space.txt", "/outside/$value#1.txt"]
    );
    assert!(dependency_paths("/target/role.wasm: /outside/$(UNKNOWN)").is_err());
    assert!(dependency_paths("missing target separator").is_err());
}
