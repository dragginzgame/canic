//! Module: current_inventory::tests::descendant_reads
//!
//! Responsibility: qualify bounded receipt reads and the pre-inspection barrier.
//! Boundary: synthetic ICP responses exercise host transport, not IC execution.

use super::*;
use std::os::unix::fs::PermissionsExt as _;

struct ReceiptTransport {
    root: PathBuf,
    icp: IcpCli,
}

impl ReceiptTransport {
    fn new() -> Self {
        let root = crate::test_support::temp_dir("terminal-descendant-receipts");
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("root.did"), "service : {}").unwrap();
        let executable = root.join("icp");
        fs::write(&executable, crate::test_support::tool_script(SCRIPT)).unwrap();
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
        Self {
            icp: IcpCli::new(executable.to_str().unwrap(), None).with_cwd(root.clone()),
            root,
        }
    }

    fn respond(
        &self,
        index: usize,
        request: [u8; 32],
        allocation: RootComponentChildAllocationResponse,
    ) {
        let input =
            RootInventoryStatusRequest::ComponentChildProvisioning(OperationStatusRequest {
                operation_id: request,
            });
        fs::write(
            self.root.join(format!("{index}.bin")),
            candid::encode_one(input).unwrap(),
        )
        .unwrap();
        let response = Ok::<_, canic_core::dto::error::Error>(
            RootInventoryStatusResponse::ComponentChildProvisioning(Box::new(allocation)),
        );
        fs::write(
            self.root.join(format!("{index}.json")),
            serde_json::json!({
                "response_bytes": hex_bytes(candid::encode_one(response).unwrap()),
            })
            .to_string(),
        )
        .unwrap();
    }
}

impl Drop for ReceiptTransport {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).unwrap();
    }
}

// Each group waits for all four requests. Serial collection fails at this barrier;
// later batches cannot start until every issued request in the previous one exits.
const SCRIPT: &str = r#"#!/bin/sh
set -eu
if [ "$1" = --version ]; then echo 'icp @ICP_VERSION@'; exit 0; fi
case " $* " in
  *" canic_root_operation_status "*" --query "*) ;;
  *) touch unexpected; exit 2;;
esac
while [ "$1" != --args-file ]; do shift; done
shift
for expected in *.bin; do
  if cmp -s "$1" "$expected"; then
    index=${expected%.bin}
    start=$((index / 4 * 4))
    for previous in $(seq 0 $((start - 1))); do
      test -e "finished-$previous" || exit 3
    done
    touch "started-$index"
    for sibling in $(seq "$start" $((start + 3))); do
      tries=0
      while [ ! -e "started-$sibling" ]; do
        tries=$((tries + 1))
        test "$tries" -lt 1000 || exit 4
        sleep 0.01
      done
    done
    touch "finished-$index"
    cat "$index.json"
    exit 0
  fi
done
exit 5
"#;

#[test]
fn descendant_receipts_overlap_drain_and_reject_before_inspection() {
    let fixture = ReceiptTransport::new();
    let (partition, member, protocol, _) = component_partition_fixture();
    let root = member.binding.fleet_subnet_root;
    let children = (0..9_u8)
        .map(|index| CanisterInfo {
            pid: Principal::from_slice(&[index + 40; 29]),
            role: protocol.binding.role.clone(),
            parent_pid: Some(member.binding.canister_id),
            module_hash: None,
            created_at: 1,
        })
        .collect::<Vec<_>>();
    let workloads = (0..9_u8)
        .map(|index| TerminalWorkloadAuthority {
            component: member.binding.component,
            operation_id: [index + 40; 32],
            root,
        })
        .collect::<Vec<_>>();
    let descendants = children
        .iter()
        .zip(&workloads)
        .enumerate()
        .map(|(index, (child, workload))| {
            let mut allocation =
                committed_descendant_allocation(&partition, &member, child, workload.operation_id);
            if index == 4 {
                allocation.operation_id = [99; 32];
            } else if index == 5 {
                allocation.parent_canister_id = Principal::anonymous();
            }
            fixture.respond(index, workload.operation_id, allocation);
            TerminalDescendantAuthority {
                child,
                component: &member.binding,
                parent_role: &member.binding.role,
                protocol: &protocol,
                release_set: partition.active_release_set,
                root,
                workload,
            }
        })
        .collect::<Vec<_>>();
    assert!(matches!(
        observe_terminal_descendants(&fixture.icp, &fixture.root.join("root.did"), &descendants),
        Err(CurrentProtocolError::TerminalInventoryField {
            field: "descendant.operation_id",
            ..
        })
    ));
    for index in 0..8 {
        assert!(fixture.root.join(format!("finished-{index}")).exists());
    }
    assert!(!fixture.root.join("started-8").exists());
    assert!(!fixture.root.join("unexpected").exists());
}
