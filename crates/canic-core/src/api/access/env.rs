use crate::{Error, access};

///
/// EnvApi
///

pub struct EnvApi;

impl EnvApi {
    pub async fn is_prime_root() -> Result<(), Error> {
        access::env::is_prime_root().await.map_err(Error::from)
    }

    pub async fn is_prime_subnet() -> Result<(), Error> {
        access::env::is_prime_subnet().await.map_err(Error::from)
    }
}
