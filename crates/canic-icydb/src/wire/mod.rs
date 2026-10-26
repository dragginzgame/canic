//! IcyDB endpoint identities and IC Wire envelopes owned by the host adapter.
//!
//! Native payload validation and resource policy remain with the database.
//! Callers carry its admitted limits into these transport operations.

use serde::{Serialize, de::DeserializeOwned};

pub use ic_wire::{Error, Limits};

const ID_BASE: u64 = u64::from_le_bytes(*b"ICYDBW00");

// One declaration owns every projection of the fixed method identity roster.
macro_rules! define_endpoints {
    ($( $variant:ident => ($name:literal, $id:literal), )*) => {
        /// A maintained IcyDB endpoint and its exact byte-contract identity.
        #[derive(Clone, Copy, Debug, Eq, PartialEq)]
        pub enum Endpoint {
            $( #[doc = $name] $variant, )*
        }

        impl Endpoint {
            /// Exact public IC method name.
            #[must_use]
            pub const fn name(self) -> &'static str {
                match self { $( Self::$variant => $name, )* }
            }

            /// Current protocol identity. Payload changes require explicit coordination.
            #[must_use]
            pub const fn id(self) -> u64 {
                ID_BASE + match self { $( Self::$variant => $id, )* }
            }

            /// Resolve one exact public method name.
            #[must_use]
            pub fn from_name(name: &str) -> Option<Self> {
                match name { $( $name => Some(Self::$variant), )* _ => None }
            }

            /// Resolve one declared protocol identity.
            #[must_use]
            pub const fn from_id(id: u64) -> Option<Self> {
                match id.wrapping_sub(ID_BASE) {
                    $( $id => Some(Self::$variant), )* _ => None,
                }
            }
        }
    };
}

define_endpoints! {
    Query => ("icydb_query", 1),
    Ddl => ("icydb_ddl", 2),
    Update => ("icydb_update", 3),
    Integrity => ("icydb_integrity", 4),
    FixturesReset => ("icydb_fixtures_reset", 5),
    FixturesLoad => ("icydb_fixtures_load", 6),
    Metrics => ("icydb_metrics", 7),
    MetricsReset => ("icydb_metrics_reset", 8),
    Snapshot => ("icydb_snapshot", 9),
    Schema => ("icydb_schema", 10),
    SchemaMigrate => ("icydb_schema_migrate", 11),
    SchemaMigration => ("icydb_schema_migration", 12),
}

impl Endpoint {
    /// Encode one canonical request for this endpoint.
    pub fn encode_request<T: Serialize>(self, value: T, limits: Limits) -> Result<Vec<u8>, Error> {
        ic_wire::native::encode_request(self.id(), &value, limits)
    }
    /// Admit identity, direction, resources and request payload primitives.
    /// Collection ordering and uniqueness remain the caller's semantic contract.
    pub fn decode_request<T: DeserializeOwned>(
        self,
        bytes: &[u8],
        limits: Limits,
    ) -> Result<T, Error> {
        ic_wire::native::decode_request(self.id(), bytes, limits)
    }
    /// Encode one canonical response for this endpoint.
    pub fn encode_response<T: Serialize>(self, value: T, limits: Limits) -> Result<Vec<u8>, Error> {
        ic_wire::native::encode_response(self.id(), &value, limits)
    }
    /// Admit identity, direction, resources and response payload primitives.
    /// Retain received bytes when exact frame identity is needed after decoding.
    pub fn decode_response<T: DeserializeOwned>(
        self,
        bytes: &[u8],
        limits: Limits,
    ) -> Result<T, Error> {
        ic_wire::native::decode_response(self.id(), bytes, limits)
    }
}

/// Encode arguments for a generated composed init hook.
pub fn encode_init_arguments<T: Serialize>(arguments: T, limits: Limits) -> Result<Vec<u8>, Error> {
    ic_wire::native::encode_request(ID_BASE + 13, &arguments, limits)
}

/// Encode arguments for a generated composed post-upgrade hook.
pub fn encode_post_upgrade_arguments<T: Serialize>(
    arguments: T,
    limits: Limits,
) -> Result<Vec<u8>, Error> {
    ic_wire::native::encode_request(ID_BASE + 14, &arguments, limits)
}

/// Decode arguments for a generated composed init hook before bootstrap.
#[doc(hidden)]
pub fn decode_init_arguments<T: DeserializeOwned>(
    bytes: &[u8],
    limits: Limits,
) -> Result<T, Error> {
    ic_wire::native::decode_request(ID_BASE + 13, bytes, limits)
}

/// Decode arguments for a generated composed post-upgrade hook before bootstrap.
#[doc(hidden)]
pub fn decode_post_upgrade_arguments<T: DeserializeOwned>(
    bytes: &[u8],
    limits: Limits,
) -> Result<T, Error> {
    ic_wire::native::decode_request(ID_BASE + 14, bytes, limits)
}

/// Export one synchronous application IC Wire method with its contract identity.
///
/// The native contract owner supplies bounded decoding/encoding in the body.
/// This adapter owns the IC export, executor context and artifact declaration.
/// `mode` is `query` or `update`; the body must explicitly reply or reject.
#[macro_export]
macro_rules! wire_endpoint {
    ($(#[$attribute:meta])* $mode:ident, $name:literal, $contract:expr, $body:block) => {
        $(#[$attribute])*
        const _: () = {
            use $crate::__private::cdk as __canic_wire_sdk;

            #[$crate::__private::native_entry($mode, name = $name, sdk = __canic_wire_sdk)]
            fn __canic_wire_export() {
                #[used]
                #[expect(unsafe_code, reason = "the host export carries its native contract identity")]
                #[unsafe(link_section = "icydb:wire")]
                static CONTRACT: [u8; 8] = ($contract).to_le_bytes();
                $body
            }
        };
    };
}

#[cfg(test)]
mod tests;
