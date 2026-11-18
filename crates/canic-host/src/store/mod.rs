use crate::{Error, Result, supplier};
use serde::{Serialize, de::DeserializeOwned};
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

pub(crate) const MAX_RECORD: usize = 12 * 1024 * 1024;

pub(crate) struct Store {
    pub path: PathBuf,
    _lock: ic_host_fs::Lock,
}

impl Store {
    pub fn open(root: &Path) -> Result<Self> {
        let directory = crate::output_publication::prepare(root, 0o700)?;
        let lock = directory
            .lock(
                std::ffi::OsStr::new("lock"),
                0o600,
                ic_host_fs::FilePolicy {
                    forbidden: 0,
                    single_link: false,
                },
            )
            .map_err(supplier)?;
        Ok(Self {
            path: root.to_path_buf(),
            _lock: lock,
        })
    }

    pub fn operation(&self, id: &str) -> Result<PathBuf> {
        if id.len() != 64
            || !id
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        {
            return Err(Error::Invalid(
                "operation must be a lowercase SHA-256 digest".into(),
            ));
        }
        Ok(self.path.join(id))
    }
}

pub(crate) fn put_in(parent: &ic_host_fs::Directory, name: &OsStr, bytes: &[u8]) -> Result<()> {
    if bytes.len() > MAX_RECORD {
        return Err(Error::Invalid("retained record too large".into()));
    }
    crate::output_publication::publish_in(
        parent,
        name,
        bytes,
        ic_host_fs::Destination::CreateNew,
        0o600,
    )
}

pub(crate) fn put_json_in(
    parent: &ic_host_fs::Directory,
    name: &OsStr,
    value: &impl Serialize,
) -> Result<()> {
    put_in(
        parent,
        name,
        &serde_json::to_vec_pretty(value).map_err(supplier)?,
    )
}

pub(crate) fn put(path: &Path, bytes: &[u8]) -> Result<()> {
    if bytes.len() > MAX_RECORD {
        return Err(Error::Invalid("retained record too large".into()));
    }
    crate::output_publication::publish(
        path,
        bytes,
        ic_host_fs::Destination::CreateNew,
        0o777,
        0o600,
    )
}

pub(crate) fn read(path: &Path, limit: usize) -> Result<Vec<u8>> {
    if path.as_os_str().as_encoded_bytes().ends_with(b"/") {
        return Err(Error::Invalid("record requires a file name".into()));
    }
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let name = path
        .file_name()
        .ok_or_else(|| Error::Invalid("record requires a file name".into()))?;
    let missing = || supplier(std::io::Error::from(std::io::ErrorKind::NotFound));
    let directory = ic_host_fs::Directory::open(&parent.canonicalize().map_err(supplier)?, 0)
        .map_err(supplier)?
        .ok_or_else(missing)?;
    let file = directory
        .open_file(
            name,
            ic_host_fs::Access::Read,
            ic_host_fs::FilePolicy {
                forbidden: 0,
                single_link: false,
            },
        )
        .map_err(supplier)?
        .ok_or_else(missing)?;
    if file.metadata().map_err(supplier)?.len() > limit as u64 {
        return Err(supplier(ic_host_artifacts::Error::Limit {
            limit: limit as u64,
        }));
    }
    let (bytes, _) = ic_host_artifacts::read(file, ic_host_artifacts::Length::AtMost(limit as u64))
        .map_err(supplier)?;
    Ok(bytes)
}

pub(crate) fn json<T: DeserializeOwned>(path: &Path) -> Result<T> {
    serde_json::from_slice(&read(path, MAX_RECORD)?).map_err(supplier)
}

pub(crate) fn put_json(path: &Path, value: &impl Serialize) -> Result<()> {
    put(path, &serde_json::to_vec_pretty(value).map_err(supplier)?)
}

#[cfg(test)]
mod parent_tests;
#[cfg(test)]
mod publication_tests;
#[cfg(test)]
mod read_tests;

#[cfg(test)]
mod lock_tests;
