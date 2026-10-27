//! Exercise exact native request framing and typed terminal observation failures.
use super::*;
use canic_contracts::{
    dto::{component_read::ComponentAllocation, error::Error},
    protocol::component_read as wire,
};
use std::os::unix::fs::PermissionsExt as _;

#[test]
fn allocation_read_preserves_absence_and_rejects_foreign_frames() {
    let directory = crate::test_support::temp_dir("ordinary-allocation-read");
    std::fs::create_dir_all(&directory).unwrap();
    let executable = directory.join("icp");
    std::fs::write(
        &executable,
        crate::test_support::tool_script(
            r#"#!/bin/sh
set -eu
if [ "$1" = --version ]; then echo 'icp @ICP_VERSION@'; exit 0; fi
case " $* " in
  *" canic_root_component_allocation "*" --query "*) ;;
  *) exit 1 ;;
esac
while [ "$1" != --args-file ]; do shift; done
shift
cp "$1" request.bin
cat response.hex
"#,
        ),
    )
    .unwrap();
    std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o700)).unwrap();
    let icp = IcpCli::new(executable.to_str().unwrap(), None).with_cwd(&directory);
    let root = Principal::from_slice(&[7]);
    let operation = [13_u8; 32];
    let response = Err::<ComponentAllocation, _>(Error::from_registered(
        canic_contracts::diagnostics::codes::STATE_UNAVAILABLE,
    ));
    for (method, expected_absence) in [
        (wire::allocation::METHOD_ID, true),
        (wire::child_allocation::METHOD_ID, false),
    ] {
        let bytes =
            ic_wire::native::encode_response(method, &response, wire::RESPONSE_LIMITS).unwrap();
        std::fs::write(directory.join("response.hex"), hex_bytes(bytes)).unwrap();
        let CurrentProtocolError::TerminalInventoryObservation { stage, source } =
            read(&icp, root, operation).unwrap_err()
        else {
            panic!("observation must fail at its typed boundary");
        };
        assert_eq!(stage, "ordinary_component");
        let CurrentProtocolError::Transport(error) = *source else {
            panic!("transport evidence must be retained");
        };
        assert_eq!(
            error.is_rejected_with(canic_contracts::diagnostics::codes::STATE_UNAVAILABLE),
            expected_absence
        );
        if !expected_absence {
            assert!(matches!(
                error,
                crate::canister_protocol::CanisterProtocolError::WireQuery { .. }
            ));
        }
        let request = std::fs::read(directory.join("request.bin")).unwrap();
        assert_eq!(
            ic_wire::native::decode_request::<[u8; 32]>(
                wire::allocation::METHOD_ID,
                &request,
                wire::REQUEST_LIMITS
            )
            .unwrap(),
            operation
        );
    }
    std::fs::remove_dir_all(directory).unwrap();
}
