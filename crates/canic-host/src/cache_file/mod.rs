use serde::{Deserialize, Serialize};
use std::{
    fs, io,
    io::Write,
    path::{Path, PathBuf},
};

///
/// CacheFileError
///
#[derive(Debug)]
pub enum CacheFileError {
    CreateDirectory {
        path: PathBuf,
        source: io::Error,
    },
    CreateRefreshLock {
        path: PathBuf,
        source: io::Error,
    },
    ReadRefreshLock {
        path: PathBuf,
        source: io::Error,
    },
    ParseRefreshLock {
        path: PathBuf,
        source: serde_json::Error,
    },
    WriteRefreshLock {
        path: PathBuf,
        source: io::Error,
    },
    RemoveRefreshLock {
        path: PathBuf,
        source: io::Error,
    },
    RefreshAlreadyInProgress {
        path: PathBuf,
        started_at_unix_ms: u64,
    },
    WriteTemp {
        path: PathBuf,
        source: io::Error,
    },
    SyncTemp {
        path: PathBuf,
        source: io::Error,
    },
    Replace {
        temp_path: PathBuf,
        target_path: PathBuf,
        source: io::Error,
    },
    SyncDirectory {
        path: PathBuf,
        source: io::Error,
    },
    WriteOutput {
        path: PathBuf,
        source: io::Error,
    },
    SyncOutput {
        path: PathBuf,
        source: io::Error,
    },
}

///
/// RefreshLockRequest
///
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RefreshLockRequest<'a> {
    pub lock_path: &'a Path,
    pub target_path: &'a Path,
    pub network: &'a str,
    pub now_unix_secs: u64,
    pub lock_stale_after_seconds: u64,
}

///
/// RefreshLockFile
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct RefreshLockFile {
    schema_version: u32,
    network: String,
    pid: u32,
    started_at_unix_ms: u64,
    #[serde(alias = "catalog_path", alias = "cache_path")]
    target_path: String,
}

///
/// RefreshLockGuard
///
#[derive(Debug)]
pub struct RefreshLockGuard {
    path: PathBuf,
    active: bool,
}

impl RefreshLockGuard {
    pub fn release(mut self) -> Result<(), CacheFileError> {
        fs::remove_file(&self.path).map_err(|source| CacheFileError::RemoveRefreshLock {
            path: self.path.clone(),
            source,
        })?;
        self.active = false;
        Ok(())
    }
}

impl Drop for RefreshLockGuard {
    fn drop(&mut self) {
        if self.active {
            let _ = fs::remove_file(&self.path);
        }
    }
}

pub fn create_directory(path: &Path) -> Result<(), CacheFileError> {
    fs::create_dir_all(path).map_err(|source| CacheFileError::CreateDirectory {
        path: path.to_path_buf(),
        source,
    })
}

pub fn acquire_refresh_lock(
    request: RefreshLockRequest<'_>,
) -> Result<RefreshLockGuard, CacheFileError> {
    let now_unix_ms = request.now_unix_secs.saturating_mul(1_000);
    for attempt in 0..2 {
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(request.lock_path)
        {
            Ok(mut file) => {
                let lock = RefreshLockFile {
                    schema_version: 1,
                    network: request.network.to_string(),
                    pid: std::process::id(),
                    started_at_unix_ms: now_unix_ms,
                    target_path: request.target_path.display().to_string(),
                };
                let data = serde_json::to_vec_pretty(&lock).map_err(|source| {
                    CacheFileError::ParseRefreshLock {
                        path: request.lock_path.to_path_buf(),
                        source,
                    }
                })?;
                file.write_all(&data)
                    .map_err(|source| CacheFileError::WriteRefreshLock {
                        path: request.lock_path.to_path_buf(),
                        source,
                    })?;
                file.sync_all()
                    .map_err(|source| CacheFileError::WriteRefreshLock {
                        path: request.lock_path.to_path_buf(),
                        source,
                    })?;
                return Ok(RefreshLockGuard {
                    path: request.lock_path.to_path_buf(),
                    active: true,
                });
            }
            Err(err) if err.kind() == io::ErrorKind::AlreadyExists => {
                let existing = read_refresh_lock(request.lock_path)?;
                if lock_is_stale(
                    existing.started_at_unix_ms,
                    now_unix_ms,
                    request.lock_stale_after_seconds,
                ) && attempt == 0
                {
                    fs::remove_file(request.lock_path).map_err(|source| {
                        CacheFileError::RemoveRefreshLock {
                            path: request.lock_path.to_path_buf(),
                            source,
                        }
                    })?;
                    continue;
                }
                return Err(CacheFileError::RefreshAlreadyInProgress {
                    path: request.lock_path.to_path_buf(),
                    started_at_unix_ms: existing.started_at_unix_ms,
                });
            }
            Err(source) => {
                return Err(CacheFileError::CreateRefreshLock {
                    path: request.lock_path.to_path_buf(),
                    source,
                });
            }
        }
    }
    Err(CacheFileError::CreateRefreshLock {
        path: request.lock_path.to_path_buf(),
        source: io::Error::new(io::ErrorKind::AlreadyExists, "refresh lock retry exhausted"),
    })
}

pub fn write_text_atomically(target_path: &Path, contents: &str) -> Result<(), CacheFileError> {
    let target_dir = target_path
        .parent()
        .expect("cache target path always has parent");
    let target_file = target_path
        .file_name()
        .and_then(|file| file.to_str())
        .unwrap_or("cache");
    let temp_path = target_dir.join(format!("{target_file}.tmp.{}", std::process::id()));
    {
        let mut temp = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_path)
            .map_err(|source| CacheFileError::WriteTemp {
                path: temp_path.clone(),
                source,
            })?;
        temp.write_all(contents.as_bytes())
            .map_err(|source| CacheFileError::WriteTemp {
                path: temp_path.clone(),
                source,
            })?;
        temp.sync_all().map_err(|source| CacheFileError::SyncTemp {
            path: temp_path.clone(),
            source,
        })?;
    }
    fs::rename(&temp_path, target_path).map_err(|source| CacheFileError::Replace {
        temp_path: temp_path.clone(),
        target_path: target_path.to_path_buf(),
        source,
    })?;
    sync_directory(target_dir)
}

pub fn write_text_output(output_path: &Path, contents: &str) -> Result<(), CacheFileError> {
    if let Some(parent) = output_path.parent() {
        create_directory(parent)?;
    }
    let mut output =
        fs::File::create(output_path).map_err(|source| CacheFileError::WriteOutput {
            path: output_path.to_path_buf(),
            source,
        })?;
    output
        .write_all(contents.as_bytes())
        .map_err(|source| CacheFileError::WriteOutput {
            path: output_path.to_path_buf(),
            source,
        })?;
    output
        .sync_all()
        .map_err(|source| CacheFileError::SyncOutput {
            path: output_path.to_path_buf(),
            source,
        })
}

fn read_refresh_lock(lock_path: &Path) -> Result<RefreshLockFile, CacheFileError> {
    let data = fs::read(lock_path).map_err(|source| CacheFileError::ReadRefreshLock {
        path: lock_path.to_path_buf(),
        source,
    })?;
    serde_json::from_slice(&data).map_err(|source| CacheFileError::ParseRefreshLock {
        path: lock_path.to_path_buf(),
        source,
    })
}

fn lock_is_stale(started_at_unix_ms: u64, now_unix_ms: u64, stale_after_seconds: u64) -> bool {
    now_unix_ms
        .saturating_sub(started_at_unix_ms)
        .gt(&stale_after_seconds.saturating_mul(1_000))
}

fn sync_directory(path: &Path) -> Result<(), CacheFileError> {
    fs::File::open(path)
        .and_then(|dir| dir.sync_all())
        .map_err(|source| CacheFileError::SyncDirectory {
            path: path.to_path_buf(),
            source,
        })
}
