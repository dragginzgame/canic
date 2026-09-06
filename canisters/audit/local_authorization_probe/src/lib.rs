//! Module: local_authorization_probe
//!
//! Responsibility: retain local authorization without token-issuer state.
//! Does not own: application policy or an independent lifecycle.
//! Boundary: generated Canic endpoints and restoration own the selected state.

#![expect(clippy::unused_async)]

canic::start!();

/// Run the ordinary application setup boundary.
async fn canic_setup() {}

/// Accept the ordinary managed-canister initialization payload.
async fn canic_install(_: Option<Vec<u8>>) {}

/// Run the ordinary same-release restoration boundary.
async fn canic_upgrade() {}

canic::finish!();
