use crate::{Error, Result, supplier};
use ic_host_fs::durable::{self, PublicationMode, WriteOptions};
use serde::{Serialize, de::DeserializeOwned};
use std::{
    fs::File,
    path::{Path, PathBuf},
};

pub(crate) const MAX_RECORD: usize = 12 * 1024 * 1024;

pub(crate) struct Store {
    pub path: PathBuf,
    _lock: File,
}

impl Store {
    pub fn open(root: &Path) -> Result<Self> {
        durable::prepare_private_parents(root).map_err(supplier)?;
        let lock = durable::open_private_regular_lock_file(&root.join("lock")).map_err(supplier)?;
        durable::try_lock_exclusive(&lock).map_err(supplier)?;
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

pub(crate) fn put(path: &Path, bytes: &[u8]) -> Result<()> {
    if bytes.len() > MAX_RECORD {
        return Err(Error::Invalid("retained record too large".into()));
    }
    durable::write_bytes(
        path,
        WriteOptions {
            mode: PublicationMode::CreateNew,
            permissions: 0o600,
        },
        bytes,
    )
    .map_err(supplier)
}

pub(crate) fn read(path: &Path, limit: usize) -> Result<Vec<u8>> {
    ic_host_fs::read::read_file_no_follow(path, limit).map_err(supplier)
}

pub(crate) fn json<T: DeserializeOwned>(path: &Path) -> Result<T> {
    serde_json::from_slice(&read(path, MAX_RECORD)?).map_err(supplier)
}

pub(crate) fn put_json(path: &Path, value: &impl Serialize) -> Result<()> {
    put(path, &serde_json::to_vec_pretty(value).map_err(supplier)?)
}
