//! Export the DB-mounted actor's actual endpoint schemas.
use canic_schema_role as _;

fn main() -> Result<(), String> {
    canic::inspection::print_manifest(&Default::default())
}
