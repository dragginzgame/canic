pub use crate::domain::subnet::{SubnetContextParams, SubnetIdentity};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{cdk::types::Principal, ids::SubnetSlotId};

    #[test]
    fn reexported_subnet_identity_roundtrips_through_candid() {
        let identity = SubnetIdentity::Standard(SubnetContextParams {
            subnet_type: SubnetSlotId::new("app"),
            prime_root_pid: Principal::from_slice(&[1; 29]),
        });

        let bytes = candid::encode_one(&identity).expect("encode subnet identity");
        let decoded: SubnetIdentity = candid::decode_one(&bytes).expect("decode subnet identity");

        match decoded {
            SubnetIdentity::Standard(params) => {
                assert_eq!(params.subnet_type, SubnetSlotId::new("app"));
                assert_eq!(params.prime_root_pid, Principal::from_slice(&[1; 29]));
            }
            SubnetIdentity::Prime
            | SubnetIdentity::PrimeWithModuleHash(_)
            | SubnetIdentity::Manual => panic!("decoded unexpected subnet identity"),
        }
    }
}
