use super::*;
use crate::test_support::temp_dir;
use std::{collections::BTreeMap, env, fs, io::Write as _, process::Command};

#[test]
fn batch_extraction_preserves_order_and_duplicate_cache_entries() {
    let root = temp_dir("candid-batch-order");
    fs::create_dir_all(&root).unwrap();
    let cache = cache_fixture(&root);
    let inputs = (0..=(MAX_EXTRACTORS * 2))
        .map(|index| {
            let wasm = root.join(format!("{index}.wasm"));
            // Distinct paths deliberately share some content-addressed cache entries.
            fs::write(
                &wasm,
                declaration_module(&format!("service : {{ method_{} : () -> (); }}", index % 3)),
            )
            .unwrap();
            CandidExtractionInput {
                role: "fixture",
                wasm,
            }
        })
        .collect::<Vec<_>>();
    let expected = inputs
        .iter()
        .map(|input| extract_candid_with_tool(&input.wasm, &cache.extractor).unwrap())
        .collect::<Vec<_>>();
    assert!(
        extract_configured_candids(Some(&cache), &[])
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        extract_configured_candids(Some(&cache), &inputs).unwrap(),
        expected
    );
    assert_eq!(
        extract_configured_candids(Some(&cache), &inputs).unwrap(),
        expected
    );
    for (input, expected) in inputs.iter().zip(expected) {
        assert_eq!(cache.extract(&input.wasm).unwrap(), (expected, true));
    }
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn batch_extraction_drains_successes_before_first_error_and_stops_scheduling() {
    let root = temp_dir("candid-batch-failure");
    fs::create_dir_all(&root).unwrap();
    let cache = cache_fixture(&root);
    let inputs = (0..=MAX_EXTRACTORS)
        .map(|index| {
            let wasm = root.join(format!("{index}.wasm"));
            fs::write(
                &wasm,
                declaration_module(&format!("service : {{ method_{index} : () -> (); }}")),
            )
            .unwrap();
            CandidExtractionInput {
                role: if index == 0 { "first" } else { "later" },
                wasm,
            }
        })
        .collect::<Vec<_>>();
    fs::remove_file(&inputs[0].wasm).unwrap();
    fs::remove_file(&inputs[1].wasm).unwrap();
    assert!(
        matches!(extract_configured_candids(Some(&cache), &inputs), Err(CandidBatchError::Extraction { role, .. }) if role == "first")
    );
    for input in &inputs[2..MAX_EXTRACTORS] {
        assert!(cache.extract(&input.wasm).unwrap().1);
    }
    let unissued = file_hash(&inputs[MAX_EXTRACTORS].wasm).unwrap();
    assert!(!cache.record_path(&unissued).exists());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn shell_depth_and_credentials_preserve_extraction_reuse_and_other_inputs_invalidate() {
    const CHILD_ROOT: &str = "CANIC_TEST_EXTRACTION_ENVIRONMENT_ROOT";
    const BUILD_INPUT: &str = "CANIC_TEST_EXTRACTION_INPUT";
    const CREDENTIAL: &str = crate::icp::CANIC_ICP_IDENTITY_PASSWORD_FILE_ENV;
    if let Some(root) = env::var_os(CHILD_ROOT) {
        let root = PathBuf::from(root);
        let cache = CandidExtractionCache::with_extractor(
            root.join("cache"),
            root.join("extractor_fixture"),
        )
        .unwrap();
        let wasm = root.join("role.wasm");
        let fresh = extract_candid_with_tool(&wasm, &cache.extractor).unwrap();
        let (candid, reused) = cache.extract(&wasm).unwrap();
        assert_eq!(candid, fresh);
        fs::write(root.join("reused"), [u8::from(reused)]).unwrap();
        return;
    }
    let root = temp_dir("candid-cache-environment");
    fs::create_dir_all(&root).unwrap();
    cache_fixture(&root);
    fs::write(root.join("role.wasm"), declaration_module("service : {};")).unwrap();
    let thread = std::thread::current();
    let invoke = |credential: Option<&str>,
                  input: &str,
                  depth: &str,
                  session: Option<(&str, &str)>,
                  launcher_input: &str| {
        let mut command = Command::new(env::current_exe().unwrap());
        command
            .args(["--exact", thread.name().unwrap()])
            .env(CHILD_ROOT, &root)
            .env(BUILD_INPUT, input)
            .env("SHLVL", depth)
            .env("CODEX_BUILD_FIXTURE_INPUT", launcher_input)
            .env_remove("CODEX_SESSION_ID")
            .env_remove("CODEX_THREAD_ID")
            .env_remove(CREDENTIAL);
        if let Some((session, thread)) = session {
            command
                .env("CODEX_SESSION_ID", session)
                .env("CODEX_THREAD_ID", thread);
        }
        if let Some(value) = credential {
            command.env(CREDENTIAL, value);
        }
        let output = command.output().unwrap();
        assert!(
            output.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        fs::read(root.join("reused")).unwrap()
    };
    assert_eq!(
        invoke(Some("fixture-credential-a"), "alpha", "1", None, "alpha"),
        [0]
    );
    assert_eq!(
        invoke(Some("fixture-credential-b"), "alpha", "2", None, "alpha"),
        [1]
    );
    assert_eq!(invoke(None, "alpha", "7", None, "alpha"), [1]);
    for session in [
        Some(("session-a", "thread-a")),
        Some(("session-b", "thread-b")),
        None,
    ] {
        assert_eq!(invoke(None, "alpha", "1", session, "alpha"), [1]);
    }
    // Unknown CODEX keys remain ordinary child inputs and independently invalidate reuse.
    assert_eq!(invoke(None, "alpha", "1", None, "beta"), [0]);
    assert_eq!(invoke(None, "alpha", "1", None, "beta"), [1]);
    assert_eq!(
        invoke(Some("fixture-credential-a"), "beta", "1", None, "beta"),
        [0]
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
#[ignore = "requires installed candid-extractor; ordinary CI uses a native extractor fixture"]
fn real_extractor_reuse_matches_fresh_declarations() {
    let root = temp_dir("candid-cache-real-extractor");
    fs::create_dir_all(&root).unwrap();
    let inputs = if let Ok(paths) = env::var("CANIC_CANDID_QUALIFICATION_INPUTS") {
        serde_json::from_str::<Vec<PathBuf>>(&paths).unwrap()
    } else {
        let wasm = root.join("role.wasm");
        fs::write(
            &wasm,
            declaration_module("service : { read : () -> () query; }"),
        )
        .unwrap();
        vec![wasm]
    };
    assert!(!inputs.is_empty());
    let started = std::time::Instant::now();
    let cache = CandidExtractionCache::new(root.join("cache")).unwrap();
    let mut outputs = Vec::new();
    for path in &inputs {
        let (candid, reused) = cache.extract(path).unwrap();
        assert!(!reused);
        outputs.push(candid);
    }
    let first_millis = started.elapsed().as_millis();
    let started = std::time::Instant::now();
    let next = CandidExtractionCache::new(root.join("cache")).unwrap();
    for (path, expected) in inputs.iter().zip(&outputs) {
        assert_eq!(next.extract(path).unwrap(), (expected.clone(), true));
    }
    let reused_millis = started.elapsed().as_millis();
    let started = std::time::Instant::now();
    for (path, expected) in inputs.iter().zip(&outputs) {
        assert_eq!(
            extract_candid_with_tool(path, &cache.extractor).unwrap(),
            *expected
        );
    }
    let fresh_millis = started.elapsed().as_millis();
    let batch = inputs
        .iter()
        .map(|wasm| CandidExtractionInput {
            role: "qualification",
            wasm: wasm.clone(),
        })
        .collect::<Vec<_>>();
    let mut sequential_millis = Vec::new();
    let mut parallel_millis = Vec::new();
    // Alternate order with separate empty caches; every result must match fresh extraction.
    for round in 0..3 {
        for parallel in if round % 2 == 0 {
            [false, true]
        } else {
            [true, false]
        } {
            let selected =
                CandidExtractionCache::new(root.join(format!("cohort-{round}-{parallel}")))
                    .unwrap();
            let started = std::time::Instant::now();
            let observed = if parallel {
                extract_configured_candids(Some(&selected), &batch).unwrap()
            } else {
                inputs
                    .iter()
                    .map(|path| {
                        extract_configured_candid(Some(&selected), "qualification", path).unwrap()
                    })
                    .collect()
            };
            let elapsed = started.elapsed().as_millis();
            assert_eq!(observed, outputs);
            if parallel {
                parallel_millis.push(elapsed);
            } else {
                sequential_millis.push(elapsed);
            }
        }
    }
    println!(
        "{}",
        serde_json::json!({
            "inputs": inputs,
            "candid_sha256": outputs.iter().map(|bytes| format!("{:x}", Sha256::digest(bytes))).collect::<Vec<_>>(),
            "cold_cache_millis": first_millis,
            "reused_millis": reused_millis,
            "fresh_extraction_millis": fresh_millis,
            "sequential_cold_cache_millis": sequential_millis,
            "four_wide_cold_cache_millis": parallel_millis,
            "wasm_sha256": inputs.iter().map(|path| file_hash(path).unwrap()).collect::<Vec<_>>(),
            "extractor_sha256": cache.extractor_sha256,
        })
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn changed_role_and_shared_input_reuse_only_unchanged_compiled_declarations() {
    let root = declaration_workspace();
    let cache = cache_fixture(&root);
    build_declarations(&root);
    let left = declaration_wasm(&root, "left");
    let right = declaration_wasm(&root, "right");
    let (left_before, left_hit) = cache.extract(&left).unwrap();
    let (right_before, right_hit) = cache.extract(&right).unwrap();
    assert!(!left_hit && !right_hit);
    assert_eq!(cache.extract(&left).unwrap(), (left_before.clone(), true));
    assert_eq!(cache.extract(&right).unwrap(), (right_before.clone(), true));

    write_role(&root, "left", "changed_left");
    let fresh = build_declarations(&root);
    assert_eq!(fresh.get("left"), Some(&false));
    assert_eq!(fresh.get("right"), Some(&true));
    let (changed_left, hit) = cache.extract(&left).unwrap();
    assert!(!hit);
    assert_ne!(changed_left, left_before);
    assert_eq!(cache.extract(&right).unwrap(), (right_before.clone(), true));
    assert_eq!(
        changed_left,
        extract_candid_with_tool(&left, &cache.extractor).unwrap()
    );
    assert_eq!(
        right_before,
        extract_candid_with_tool(&right, &cache.extractor).unwrap()
    );

    fs::write(root.join("shared.did"), "shared_changed: () -> (); ").unwrap();
    let fresh = build_declarations(&root);
    assert!(fresh.values().all(|fresh| !fresh));
    for wasm in [&left, &right] {
        let (candid, hit) = cache.extract(wasm).unwrap();
        assert!(!hit);
        assert_eq!(
            candid,
            extract_candid_with_tool(wasm, &cache.extractor).unwrap()
        );
    }
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn corrupt_or_misbound_cache_records_reextract_current_candid() {
    let root = temp_dir("candid-cache-corrupt");
    fs::create_dir_all(&root).unwrap();
    let wasm = root.join("role.wasm");
    fs::write(
        &wasm,
        declaration_module("service : { read : () -> () query; }"),
    )
    .unwrap();
    let cache = cache_fixture(&root);
    let (expected, hit) = cache.extract(&wasm).unwrap();
    assert!(!hit);
    let path = cache.record_path(&file_hash(&wasm).unwrap());
    for field in ["identity", "wasm_sha256", "candid_sha256", "candid"] {
        let mut record: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        record[field] = "corrupted".into();
        fs::write(&path, serde_json::to_vec(&record).unwrap()).unwrap();
        assert_eq!(cache.extract(&wasm).unwrap(), (expected.clone(), false));
        assert_eq!(cache.extract(&wasm).unwrap(), (expected.clone(), true));
    }
    fs::write(&path, b"{").unwrap();
    assert_eq!(cache.extract(&wasm).unwrap(), (expected.clone(), false));
    fs::write(&path, vec![b' '; CACHE_RECORD_LIMIT + 1]).unwrap();
    assert_eq!(cache.extract(&wasm).unwrap(), (expected, false));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn changed_extractor_refuses_even_a_cached_result() {
    let root = temp_dir("candid-cache-tool-drift");
    fs::create_dir_all(&root).unwrap();
    let wasm = root.join("role.wasm");
    fs::write(&wasm, declaration_module("service : {} ")).unwrap();
    let mut cache = cache_fixture(&root);
    let tool = root.join("extractor");
    fs::copy(&cache.extractor, &tool).unwrap();
    cache.extractor = tool.clone();
    let (expected, _) = cache.extract(&wasm).unwrap();
    fs::OpenOptions::new()
        .append(true)
        .open(&tool)
        .unwrap()
        .write_all(b"different native executable identity")
        .unwrap();
    let error = cache.extract(&wasm).unwrap_err();
    assert!(
        matches!(error.downcast_ref::<BuildReuseError>(), Some(BuildReuseError::ChangedInput(path)) if path == &tool)
    );
    let changed = CandidExtractionCache::with_extractor(root.join("cache"), tool).unwrap();
    assert_ne!(changed.identity, cache.identity);
    assert_eq!(changed.extract(&wasm).unwrap(), (expected, false));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn failed_extraction_does_not_create_a_cache_record() {
    let root = temp_dir("candid-cache-extraction-failure");
    fs::create_dir_all(&root).unwrap();
    let wasm = root.join("role.wasm");
    fs::write(&wasm, b"not a Wasm module").unwrap();
    let cache = cache_fixture(&root);
    assert!(cache.extract(&wasm).is_err());
    assert!(!cache.record_path(&file_hash(&wasm).unwrap()).exists());
    fs::write(&wasm, declaration_module("service : {} ")).unwrap();
    assert!(!cache.extract(&wasm).unwrap().1);
    assert!(cache.extract(&wasm).unwrap().1);
    fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn symlink_inputs_are_refused_and_cache_symlinks_do_not_modify_their_target() {
    let root = temp_dir("candid-cache-symlink");
    fs::create_dir_all(&root).unwrap();
    let wasm = root.join("role.wasm");
    fs::write(&wasm, declaration_module("service : {} ")).unwrap();
    let cache = cache_fixture(&root);
    let (expected, _) = cache.extract(&wasm).unwrap();
    let redirected = root.join("redirected.wasm");
    std::os::unix::fs::symlink(&wasm, &redirected).unwrap();
    let error = cache.extract(&redirected).unwrap_err();
    assert!(
        matches!(error.downcast_ref::<BuildReuseError>(), Some(BuildReuseError::Unsupported(path)) if path == &redirected)
    );
    let record = cache.record_path(&file_hash(&wasm).unwrap());
    fs::remove_file(&record).unwrap();
    let foreign = root.join("foreign.json");
    fs::write(&foreign, b"untouched").unwrap();
    std::os::unix::fs::symlink(&foreign, &record).unwrap();
    assert_eq!(cache.extract(&wasm).unwrap(), (expected, false));
    assert_eq!(fs::read(foreign).unwrap(), b"untouched");
    fs::remove_dir_all(root).unwrap();
}

// Ordinary host tests need only Rust. This native extractor fixture reads the
// literal Candid in these tiny declaration Wasms; real-tool qualification is separate.
fn cache_fixture(root: &Path) -> CandidExtractionCache {
    let source = root.join("extractor_fixture.rs");
    let binary = root.join("extractor_fixture");
    fs::write(
        &source,
        r#"
fn main() {
    assert!(std::env::var_os("CANIC_ICP_IDENTITY_PASSWORD_FILE").is_none());
    assert!(std::env::var_os("CODEX_SESSION_ID").is_none());
    assert!(std::env::var_os("CODEX_THREAD_ID").is_none());
    let bytes = std::fs::read(std::env::args_os().nth(1).unwrap()).unwrap();
    if !bytes.starts_with(b"\0asm") { std::process::exit(1); }
    let marker = b"service : {";
    let start = bytes.windows(marker.len()).position(|part| part == marker).unwrap();
    let end = bytes[start..].iter().position(|byte| *byte == 0).unwrap() + start;
    println!("{}", std::str::from_utf8(&bytes[start..end]).unwrap());
}
"#,
    )
    .unwrap();
    let output = Command::new("rustc")
        .args(["--edition=2024", "--crate-name=extractor_fixture"])
        .arg(&source)
        .arg("-o")
        .arg(&binary)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    CandidExtractionCache::with_extractor(root.join("cache"), binary).unwrap()
}

fn declaration_workspace() -> PathBuf {
    let root = temp_dir("candid-cache-changed-closure");
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("Cargo.toml"),
        "[workspace]\nmembers=[\"left\",\"right\"]\nresolver=\"3\"\n",
    )
    .unwrap();
    fs::write(root.join("Cargo.lock"), "version=4\n[[package]]\nname=\"left\"\nversion=\"0.1.0\"\n[[package]]\nname=\"right\"\nversion=\"0.1.0\"\n").unwrap();
    fs::write(root.join("shared.did"), "shared: () -> (); ").unwrap();
    for name in ["left", "right"] {
        fs::create_dir_all(root.join(name).join("src")).unwrap();
        fs::write(root.join(name).join("Cargo.toml"), format!("[package]\nname=\"{name}\"\nversion=\"0.1.0\"\nedition=\"2024\"\n[lib]\ncrate-type=[\"cdylib\"]\n")).unwrap();
        write_role(&root, name, name);
    }
    root
}

fn write_role(root: &Path, name: &str, method: &str) {
    fs::write(root.join(name).join("src/lib.rs"), format!(r#"
static CANDID: &str = concat!("service : {{ ", include_str!("../../shared.did"), "{method}: () -> (); }}\0");
#[unsafe(no_mangle)]
pub extern "C" fn get_candid_pointer() -> *const u8 {{ CANDID.as_ptr() }}
"#)).unwrap();
}

fn declaration_wasm(root: &Path, name: &str) -> PathBuf {
    root.join("target/wasm32-unknown-unknown/debug")
        .join(format!("{name}.wasm"))
}

fn build_declarations(root: &Path) -> BTreeMap<String, bool> {
    let output = Command::new("cargo")
        .current_dir(root)
        .args([
            "build",
            "--locked",
            "--offline",
            "--workspace",
            "--target",
            "wasm32-unknown-unknown",
            "--message-format=json",
        ])
        .env("CARGO_TARGET_DIR", root.join("target"))
        .env_remove("CARGO_BUILD_BUILD_DIR")
        .env("RUSTC_WRAPPER", "")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let fresh = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
        .filter(|record| record["reason"] == "compiler-artifact")
        .map(|record| {
            (
                record["target"]["name"].as_str().unwrap().to_string(),
                record["fresh"].as_bool().unwrap(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    assert_eq!(
        fresh.keys().map(String::as_str).collect::<Vec<_>>(),
        ["left", "right"]
    );
    fresh
}

fn declaration_module(candid: &str) -> Vec<u8> {
    let mut wasm = b"\0asm\x01\0\0\0".to_vec();
    section(&mut wasm, 1, &[1, 0x60, 0, 1, 0x7f]);
    section(&mut wasm, 3, &[1, 0]);
    section(&mut wasm, 5, &[1, 0, 1]);
    let mut exports = vec![2];
    for (name, kind) in [("memory", 2), ("get_candid_pointer", 0)] {
        exports.push(u8::try_from(name.len()).unwrap());
        exports.extend(name.as_bytes());
        exports.extend([kind, 0]);
    }
    section(&mut wasm, 7, &exports);
    section(&mut wasm, 10, &[1, 4, 0, 0x41, 0, 0x0b]);
    let mut data = vec![1, 0, 0x41, 0, 0x0b];
    data.push(u8::try_from(candid.len() + 1).unwrap());
    data.extend(candid.as_bytes());
    data.push(0);
    section(&mut wasm, 11, &data);
    wasm
}

fn section(wasm: &mut Vec<u8>, id: u8, bytes: &[u8]) {
    assert!(bytes.len() < 128);
    wasm.extend([id, u8::try_from(bytes.len()).unwrap()]);
    wasm.extend(bytes);
}
