//! Canic output comparison only; never an authority for opening or publication.
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};

pub(crate) fn resolve(path: &Path, base: &Path) -> io::Result<PathBuf> {
    resolve_at(path, base, 0)
}

fn resolve_at(path: &Path, base: &Path, depth: usize) -> io::Result<PathBuf> {
    let absolute = if path.is_absolute() {
        path.to_owned()
    } else if base.is_absolute() {
        base.join(path)
    } else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "output base must be absolute",
        ));
    };
    match absolute.canonicalize() {
        Ok(path) => return Ok(path),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    let mut resolved = PathBuf::from("/");
    // Retain separators and dots: file/., file/.. and file/ must not become files.
    for part in absolute
        .as_os_str()
        .as_bytes()
        .split(|byte| *byte == b'/')
        .skip(1)
    {
        match fs::metadata(&resolved) {
            Ok(metadata) if !metadata.is_dir() => return Err(io::ErrorKind::NotADirectory.into()),
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
        match part {
            b"" | b"." => continue,
            b".." => {
                resolved.pop();
                continue;
            }
            _ => {}
        }
        let candidate = resolved.join(OsStr::from_bytes(part));
        match candidate.canonicalize() {
            Ok(path) => resolved = path,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                match fs::symlink_metadata(&candidate) {
                    Ok(metadata) if metadata.file_type().is_symlink() => {
                        if depth == 40 {
                            return Err(io::Error::new(
                                io::ErrorKind::InvalidInput,
                                "output alias depth exceeded",
                            ));
                        }
                        resolved = resolve_at(&fs::read_link(&candidate)?, &resolved, depth + 1)?;
                    }
                    Ok(_) => return Err(error),
                    Err(error) if error.kind() == io::ErrorKind::NotFound => resolved = candidate,
                    Err(error) => return Err(error),
                }
            }
            Err(error) => return Err(error),
        }
    }
    Ok(resolved)
}

#[cfg(test)]
mod tests;
