//! Exercise the real ICP command assembly, native frames and argument-file cleanup.
use super::*;
use crate::{
    canister_protocol::{CanisterProtocolError, call_wire},
    test_support,
};
use canic_contracts::{
    dto::{error::Error, root_observation::Request},
    protocol::root_observation::cycle_balance as protocol,
};
use ic_platform::principal::ExternalPrincipal as Principal;
use std::fs;

#[test]
fn replicated_wire_read_uses_one_update_and_preserves_typed_failures() {
    let directory = test_support::temp_dir("outer-wire-icp");
    fs::create_dir_all(&directory).unwrap();
    let executable = directory.join("icp");
    ic_host_process::testing::write_executable_script(
        &executable,
        test_support::tool_script(
            r#"#!/bin/sh
set -eu
case " $* " in
  *" --version "*) echo 'icp @ICP_VERSION@'; exit 0;;
esac
printf '%s\n' "$@" > arguments
printf 'call\n' >> invocations
while [ "$#" -gt 0 ]; do
  if [ "$1" = '--args-file' ]; then
    shift
    cat "$1" > request.bin
    printf '%s' "$1" > argument-path
  fi
  shift
done
cat response.hex
"#,
        ),
    )
    .unwrap();
    let icp = IcpCli::new(executable.to_str().unwrap(), Some("sample".into())).with_cwd(&directory);
    let root = Principal::from_slice(&[1]);
    let input = Request {
        canister_id: ic_platform::Principal::from(root),
        input: (),
    };
    let write_reply = |id, reply: Result<u128, Error>| {
        let bytes =
            ic_wire::native::encode_response(id, &reply, protocol::RESPONSE_LIMITS).unwrap();
        fs::write(
            directory.join("response.hex"),
            canic_core::cdk::utils::hash::hex_bytes(bytes),
        )
        .unwrap();
    };
    let call = || {
        call_wire::<_, u128>(
            &icp,
            root,
            protocol::METHOD,
            protocol::METHOD_ID,
            &input,
            protocol::REQUEST_LIMITS,
            protocol::RESPONSE_LIMITS,
        )
    };
    write_reply(protocol::METHOD_ID, Ok(u128::MAX));
    assert_eq!(call().unwrap(), u128::MAX);
    let args = fs::read_to_string(directory.join("arguments")).unwrap();
    assert!(args.lines().any(|arg| arg == "--args-file"));
    assert!(args.lines().any(|arg| arg == "hex"));
    assert!(
        !args
            .lines()
            .any(|arg| matches!(arg, "--query" | "--candid"))
    );
    let decoded: Request<()> = ic_wire::native::decode_request(
        protocol::METHOD_ID,
        &fs::read(directory.join("request.bin")).unwrap(),
        protocol::REQUEST_LIMITS,
    )
    .unwrap();
    assert_eq!(decoded.canister_id, input.canister_id);
    let argument_path = fs::read_to_string(directory.join("argument-path")).unwrap();
    assert!(!std::path::Path::new(&argument_path).exists());
    let rejection =
        Error::from_registered(canic_contracts::diagnostics::codes::AUTHORITY_UNAVAILABLE);
    write_reply(protocol::METHOD_ID, Err(rejection));
    assert!(
        matches!(call(), Err(CanisterProtocolError::Response { source: IcpJsonResponseError::Rejected(error), .. }) if error == rejection)
    );
    write_reply(protocol::METHOD_ID + 1, Ok(1));
    assert!(matches!(
        call(),
        Err(CanisterProtocolError::WireUpdate {
            source: IcpWireQueryError::Wire(ic_wire::Error::Identity),
            ..
        })
    ));
    assert_eq!(
        fs::read_to_string(directory.join("invocations"))
            .unwrap()
            .lines()
            .count(),
        3
    );
    fs::remove_dir_all(directory).unwrap();
}
