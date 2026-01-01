/// Initialize environment state for the root canister during init.
///
/// This must only be called from the IC `init` hook.
pub(crate) fn root_init_env(identity: SubnetIdentity) -> Result<(), Error> {
    let self_pid = canister_self();

    let (subnet_pid, subnet_role, prime_root_pid) = match identity {
        SubnetIdentity::Prime => {
            // Prime subnet: root == prime root == subnet
            (self_pid, SubnetRole::PRIME, self_pid)
        }

        SubnetIdentity::Standard(params) => {
            // Standard subnet syncing from prime
            (self_pid, params.subnet_type, params.prime_root_pid)
        }

        SubnetIdentity::Manual(pid) => {
            // Test/support only: explicit subnet override
            (pid, SubnetRole::MANUAL, pid)
        }
    };

    let env = EnvSnapshot {
        prime_root_pid: Some(prime_root_pid),
        root_pid: Some(self_pid),
        subnet_pid: Some(subnet_pid),
        subnet_role: Some(subnet_role),
        canister_role: Some(CanisterRole::ROOT),
        parent_pid: Some(prime_root_pid),
    };

    Self::import(env)
}
