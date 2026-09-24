//! Isolated audit harness: structural finalization and real PocketIC observations.
use candid::{CandidType, Principal};
use ic_testkit::pic::{PocketIcBuilder, PocketIcBuilderExt, PocketIcStartupConfig};
use pocket_ic::{PocketIc, RejectCode, common::rest::RawEffectivePrincipal};
use sha2::{Digest, Sha256};
use std::{fs, ops::Range, path::PathBuf, time::Duration};
use wasmparser::{DataKind, ExternalKind, Operator, Parser, Payload};

fn hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn slot(bytes: &[u8]) -> Result<Range<usize>, &'static str> {
    wasmparser::Validator::new()
        .validate_all(bytes)
        .map_err(|_| "invalid_wasm")?;
    let mut globals = Vec::new();
    let mut exported = None;
    let mut segments = Vec::new();
    for payload in Parser::new(0).parse_all(bytes) {
        match payload.map_err(|_| "parse")? {
            Payload::GlobalSection(reader) => {
                for global in reader {
                    let global = global.map_err(|_| "global")?;
                    let mut ops = global.init_expr.get_operators_reader();
                    let value = match ops.read().map_err(|_| "constant")? {
                        Operator::I32Const { value } if !global.ty.mutable => Some(value as u32),
                        _ => None,
                    };
                    globals.push(value);
                }
            }
            Payload::ExportSection(reader) => {
                for export in reader {
                    let export = export.map_err(|_| "export")?;
                    if export.name == "CANIC_BINDING_SLOT_V1" {
                        if export.kind != ExternalKind::Global || exported.is_some() {
                            return Err("descriptor");
                        }
                        exported = Some(export.index as usize);
                    }
                }
            }
            Payload::DataSection(reader) => {
                for data in reader {
                    let data = data.map_err(|_| "data")?;
                    let DataKind::Active {
                        memory_index: 0,
                        offset_expr,
                    } = data.kind
                    else {
                        return Err("unsupported_data");
                    };
                    let Operator::I32Const { value } = offset_expr
                        .get_operators_reader()
                        .read()
                        .map_err(|_| "offset")?
                    else {
                        return Err("offset");
                    };
                    segments.push((value as u32 as usize, data.data));
                }
            }
            _ => {}
        }
    }
    // This deliberately small fixture imports functions only. A product finalizer
    // must additionally resolve imported-global indices and the complete memory model.
    let address = globals
        .get(exported.ok_or("missing_descriptor")?)
        .copied()
        .flatten()
        .ok_or("address")? as usize;
    let end = address.checked_add(64).ok_or("overflow")?;
    let mut resolved = None;
    for (base, data) in segments {
        let data_end = base.checked_add(data.len()).ok_or("overflow")?;
        if address < data_end && base < end {
            if resolved.is_some() || base > address || data_end < end {
                return Err("overlap");
            }
            let start = data.as_ptr() as usize - bytes.as_ptr() as usize + address - base;
            resolved = Some(start..start + 64);
        }
    }
    resolved.ok_or("missing_slot")
}

fn bind(bytes: &[u8], expected_hash: &str, id: &[u8]) -> Result<Vec<u8>, &'static str> {
    if hash(bytes) != expected_hash {
        return Err("template_hash");
    }
    if id.len() != 64
        || !id
            .iter()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(byte))
    {
        return Err("identity");
    }
    let range = slot(bytes)?;
    if bytes[range.clone()] != [b'?'; 64] {
        return Err("already_bound");
    }
    let mut result = bytes.to_vec();
    result[range].copy_from_slice(id);
    wasmparser::Validator::new()
        .validate_all(&result)
        .map_err(|_| "invalid_result")?;
    Ok(result)
}

#[derive(CandidType)]
enum Mode {
    #[expect(non_camel_case_types, reason = "exact management protocol variant")]
    install,
}

#[derive(CandidType)]
struct InstallArgs {
    mode: Mode,
    canister_id: Principal,
    wasm_module: Vec<u8>,
    arg: Vec<u8>,
}

fn install(
    pic: &PocketIc,
    wasm: &[u8],
    arg: &[u8],
) -> Result<Principal, pocket_ic::RejectResponse> {
    let id = pic.create_canister();
    pic.add_cycles(id, 10_000_000_000_000);
    pic.update_call_with_effective_principal(
        Principal::management_canister(),
        RawEffectivePrincipal::CanisterId(id.as_slice().to_vec()),
        Principal::anonymous(),
        "install_code",
        candid::encode_one(InstallArgs {
            mode: Mode::install,
            canister_id: id,
            wasm_module: wasm.to_vec(),
            arg: arg.to_vec(),
        })
        .unwrap(),
    )?;
    Ok(id)
}

fn main() {
    let root = PathBuf::from(std::env::args().nth(1).unwrap());
    let binary = PathBuf::from(std::env::args().nth(2).unwrap());
    let template = fs::read(root.join("optimized.wasm")).unwrap();
    let template_hash = hash(&template);
    let range = slot(&template).unwrap();
    let ids = [b'a'; 64].into_iter().collect::<Vec<_>>();
    let a = bind(&template, &template_hash, &ids).unwrap();
    let b = bind(&template, &template_hash, &[b'b'; 64]).unwrap();
    assert_ne!(hash(&a), hash(&b));
    assert_eq!(bind(&template, &template_hash, &ids).unwrap(), a);
    for candidate in [&a, &b] {
        assert_eq!(candidate[..range.start], template[..range.start]);
        assert_eq!(candidate[range.end..], template[range.end..]);
    }
    assert_eq!(bind(&a, &hash(&a), &ids), Err("already_bound"));
    let mut changed = template.clone();
    changed[range.start] = b'!';
    assert_eq!(bind(&changed, &template_hash, &ids), Err("template_hash"));
    assert_eq!(bind(&template, &template_hash, b"wrong"), Err("identity"));
    fs::write(root.join("bound-a.wasm"), &a).unwrap();
    fs::write(root.join("bound-b.wasm"), &b).unwrap();
    let server = PocketIcStartupConfig::spawn(binary, Duration::from_secs(30))
        .start_managed_server()
        .unwrap();
    let pic = PocketIcBuilder::new()
        .with_application_subnet()
        .try_build(PocketIcStartupConfig::connect(
            server.url(),
            Duration::from_secs(30),
        ))
        .unwrap();
    for (wasm, id) in [(&a, [b'a'; 64]), (&b, [b'b'; 64])] {
        let canister = install(&pic, wasm, &id).unwrap();
        assert_eq!(
            pic.query_call(canister, Principal::anonymous(), "binding", vec![])
                .unwrap(),
            id
        );
        pic.upgrade_canister(canister, wasm.clone(), id.to_vec(), None)
            .unwrap();
        assert_eq!(
            pic.query_call(canister, Principal::anonymous(), "binding", vec![])
                .unwrap(),
            id
        );
    }
    for (wasm, arg) in [(&template, [b'a'; 64]), (&a, [b'b'; 64])] {
        let rejected = install(&pic, wasm, &arg).unwrap_err();
        assert_eq!(rejected.reject_code, RejectCode::CanisterError);
    }
    let result = serde_json::json!({
        "schema_version": 1, "scope": "synthetic_fixture_only", "template_sha256": template_hash,
        "template_bytes": template.len(), "slot_start": range.start, "slot_bytes": range.len(),
        "bound_a_sha256": hash(&a), "bound_b_sha256": hash(&b),
        "non_binding_bytes_identical": true, "exact_repeat": true,
        "pocketic": {"two_distinct_bindings": true, "same_wasm_restore": true, "unbound_rejected": true, "mismatched_rejected": true},
        "production_role_qualification": false, "speedup_measured": false,
    });
    fs::write(
        root.join("result.json"),
        serde_json::to_vec_pretty(&result).unwrap(),
    )
    .unwrap();
    println!("{}", serde_json::to_string_pretty(&result).unwrap());
}
