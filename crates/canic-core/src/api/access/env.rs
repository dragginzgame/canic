use crate::{PublicError, access};

pub async fn is_prime_root() -> Result<(), PublicError> {
    access::env::is_prime_root()
        .await
        .map_err(PublicError::from)
}

pub async fn is_prime_subnet() -> Result<(), PublicError> {
    access::env::is_prime_subnet()
        .await
        .map_err(PublicError::from)
}
