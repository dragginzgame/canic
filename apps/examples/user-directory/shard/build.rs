//! Compile this role’s schema for its retained database.

// Link the entity registrations into the host build script.
use user_directory_schema as _;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let module = "user_directory_schema::user_shard";

    ic_db::build::write_schema(module, module)
}
