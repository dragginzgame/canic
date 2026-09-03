//! Tool: wasm-replica-function-count
//!
//! Responsibility: count the local functions subject to the IC replica limit.
//! Does not own: general WebAssembly validation or any other IC module limit.
//! Boundary: run only after an independently frozen WebAssembly validator.

use std::{env, fs, process::ExitCode};

const CODE_SECTION_ID: u8 = 10;
const FUNCTION_SECTION_ID: u8 = 3;
const IC_VALIDATOR_SOURCE_COMMIT: &str = "2f8dc21e2e5c37a4cae7f65d2a4230ac8f143e5a";
const MAX_DEFINED_FUNCTIONS: u32 = 50_000;
const WASM_HEADER: &[u8; 8] = b"\0asm\x01\0\0\0";

fn main() -> ExitCode {
    match run() {
        Ok(output) => {
            println!("{output}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<String, String> {
    let mut args = env::args_os().skip(1);
    let Some(argument) = args.next() else {
        return Err("usage: wasm-replica-function-count <validated.wasm> | --identity".into());
    };
    if args.next().is_some() {
        return Err("expected exactly one argument".into());
    }
    if argument == "--identity" {
        return Ok(format!(
            "canic-b1-replica-function-count/v1\tic_source_commit={IC_VALIDATOR_SOURCE_COMMIT}\tquantity=local-defined-functions\tlimit={MAX_DEFINED_FUNCTIONS}"
        ));
    }

    let wasm = fs::read(&argument)
        .map_err(|error| format!("failed to read {}: {error}", argument.to_string_lossy()))?;
    Ok(replica_limited_function_count(&wasm)?.to_string())
}

fn replica_limited_function_count(wasm: &[u8]) -> Result<u32, String> {
    if !wasm.starts_with(WASM_HEADER) {
        return Err("invalid WebAssembly module header".into());
    }

    let mut cursor = WASM_HEADER.len();
    let mut function_count = None;
    let mut code_count = None;
    while cursor < wasm.len() {
        let section_offset = cursor;
        let section_id = wasm[cursor];
        cursor += 1;
        let section_size = read_u32(wasm, &mut cursor)
            .ok_or_else(|| format!("invalid section size at byte {section_offset}"))?
            as usize;
        let section_end = cursor
            .checked_add(section_size)
            .filter(|end| *end <= wasm.len())
            .ok_or_else(|| format!("truncated section {section_id} at byte {section_offset}"))?;
        let payload = &wasm[cursor..section_end];
        match section_id {
            FUNCTION_SECTION_ID => {
                let count = read_function_section(payload, section_offset)?;
                if function_count.replace(count).is_some() {
                    return Err("duplicate WebAssembly function section".into());
                }
            }
            CODE_SECTION_ID => {
                let count = read_code_section(payload, section_offset)?;
                if code_count.replace(count).is_some() {
                    return Err("duplicate WebAssembly code section".into());
                }
            }
            _ => {}
        }
        cursor = section_end;
    }

    let function_count = function_count.unwrap_or(0);
    let code_count = code_count.unwrap_or(0);
    if function_count != code_count {
        return Err(format!(
            "function section declares {function_count} local functions but code section contains {code_count} bodies"
        ));
    }
    Ok(function_count)
}

fn read_function_section(payload: &[u8], section_offset: usize) -> Result<u32, String> {
    let mut cursor = 0;
    let count = read_u32(payload, &mut cursor)
        .ok_or_else(|| format!("invalid function vector at byte {section_offset}"))?;
    for _ in 0..count {
        read_u32(payload, &mut cursor)
            .ok_or_else(|| format!("invalid function type index at byte {section_offset}"))?;
    }
    if cursor != payload.len() {
        return Err(format!(
            "trailing function-section bytes at byte {section_offset}"
        ));
    }
    Ok(count)
}

fn read_code_section(payload: &[u8], section_offset: usize) -> Result<u32, String> {
    let mut cursor = 0;
    let count = read_u32(payload, &mut cursor)
        .ok_or_else(|| format!("invalid code vector at byte {section_offset}"))?;
    for _ in 0..count {
        let body_size = read_u32(payload, &mut cursor)
            .ok_or_else(|| format!("invalid function body size at byte {section_offset}"))?
            as usize;
        cursor = cursor
            .checked_add(body_size)
            .filter(|end| *end <= payload.len())
            .ok_or_else(|| format!("truncated function body at byte {section_offset}"))?;
    }
    if cursor != payload.len() {
        return Err(format!(
            "trailing code-section bytes at byte {section_offset}"
        ));
    }
    Ok(count)
}

fn read_u32(bytes: &[u8], cursor: &mut usize) -> Option<u32> {
    let mut value = 0_u32;
    for index in 0..5 {
        let byte = bytes.get(*cursor).copied()?;
        *cursor += 1;
        if index == 4 && byte & 0xf0 != 0 {
            return None;
        }
        value |= u32::from(byte & 0x7f) << (index * 7);
        if byte & 0x80 == 0 {
            return Some(value);
        }
    }
    None
}
