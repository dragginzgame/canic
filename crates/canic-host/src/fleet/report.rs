//! Local report publication, independent of rendering and browser policy.
use crate::{Result, output_publication, staging_directory};
use std::path::{Path, PathBuf};

/// Save a new private fleet report, allocating a directory when no path is selected.
/// # Errors
/// Refuses occupied destinations, unsafe paths or unconfirmed durable publication.
pub fn save_fleet_report(html: &str, destination: Option<&Path>) -> Result<PathBuf> {
    if let Some(path) = destination {
        output_publication::publish(
            path,
            html.as_bytes(),
            ic_host_fs::Destination::CreateNew,
            0o700,
        )?;
        return Ok(path.into());
    }
    let root = Path::new("target/fleet");
    let parent = output_publication::prepare(root, 0o700)?;
    let (name, directory) = staging_directory::allocate(&parent, "snapshot")?;
    output_publication::publish_in(
        &directory,
        "index.html".as_ref(),
        html.as_bytes(),
        ic_host_fs::Destination::CreateNew,
    )?;
    Ok(root.join(name).join("index.html"))
}
