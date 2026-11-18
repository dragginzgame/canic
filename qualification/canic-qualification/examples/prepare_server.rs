//! Explicit server preparation, separate from offline test admission.

use ic_testkit::server::{PreparedServer, host_asset, prepare};
use std::{os::unix::fs::DirBuilderExt, path::Path};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let directory = canic_qualification::server_directory();
    if args.as_slice() == ["--asset"] {
        let asset = host_asset()?;
        println!("{}\n{}", asset.url, asset.archive_sha256);
    } else if args.as_slice() == ["--check"] {
        println!("{}", PreparedServer::check(&directory)?.path().display());
    } else if args.len() == 2 {
        std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(&directory)?;
        println!(
            "{}",
            prepare(Path::new(&args[0]), &directory, Path::new(&args[1]))?
                .path()
                .display()
        );
    } else {
        return Err("usage: prepare_server --asset | --check | ARCHIVE ABSOLUTE_GZIP".into());
    }
    Ok(())
}
