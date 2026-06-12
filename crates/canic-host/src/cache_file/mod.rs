use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::{
    fs, io,
    io::Write,
    path::{Path, PathBuf},
};

///
/// CacheFileError
///
#[derive(Debug)]
pub(crate) enum CacheFileError {
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
/// CachedJsonReport
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CachedJsonReport<T> {
    pub(crate) path: PathBuf,
    pub(crate) report: T,
}

///
/// JsonCacheReport
///
pub(crate) trait JsonCacheReport {
    fn schema_version(&self) -> u32;
    fn network(&self) -> &str;
}

///
/// LoadJsonCacheRequest
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct LoadJsonCacheRequest<'a> {
    pub(crate) path: PathBuf,
    pub(crate) network: &'a str,
    pub(crate) expected_schema_version: u32,
}

///
/// LoadJsonCacheErrorHandlers
///
pub(crate) struct LoadJsonCacheErrorHandlers<Missing, Read, Parse, Unsupported, Mismatch> {
    pub(crate) missing_cache: Missing,
    pub(crate) read_cache: Read,
    pub(crate) parse_cache: Parse,
    pub(crate) unsupported_schema: Unsupported,
    pub(crate) network_mismatch: Mismatch,
}

///
/// RefreshCacheWriteRequest
///
#[derive(Clone, Copy, Debug)]
pub(crate) struct RefreshCacheWriteRequest<'a, T> {
    pub(crate) cache_path: &'a Path,
    pub(crate) lock_path: &'a Path,
    pub(crate) network: &'a str,
    pub(crate) now_unix_secs: u64,
    pub(crate) lock_stale_after_seconds: u64,
    pub(crate) dry_run: bool,
    pub(crate) output_path: Option<&'a Path>,
    pub(crate) report: &'a T,
}

///
/// RefreshCacheWriteResult
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RefreshCacheWriteResult {
    pub(crate) cache_path: String,
    pub(crate) refresh_lock_path: String,
    pub(crate) output_path: Option<String>,
    pub(crate) replaced_existing_cache: bool,
    pub(crate) wrote_cache: bool,
}

///
/// RefreshLockRequest
///
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct RefreshLockRequest<'a> {
    pub(crate) lock_path: &'a Path,
    pub(crate) target_path: &'a Path,
    pub(crate) network: &'a str,
    pub(crate) now_unix_secs: u64,
    pub(crate) lock_stale_after_seconds: u64,
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
pub(crate) struct RefreshLockGuard {
    path: PathBuf,
    active: bool,
}

impl RefreshLockGuard {
    pub(crate) fn release(mut self) -> Result<(), CacheFileError> {
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

pub(crate) fn create_directory(path: &Path) -> Result<(), CacheFileError> {
    fs::create_dir_all(path).map_err(|source| CacheFileError::CreateDirectory {
        path: path.to_path_buf(),
        source,
    })
}

pub(crate) fn load_json_cache<T, E, Missing, Read, Parse, Unsupported, Mismatch>(
    request: LoadJsonCacheRequest<'_>,
    errors: LoadJsonCacheErrorHandlers<Missing, Read, Parse, Unsupported, Mismatch>,
) -> Result<CachedJsonReport<T>, E>
where
    T: DeserializeOwned + JsonCacheReport,
    Missing: Fn(PathBuf) -> E,
    Read: Fn(PathBuf, io::Error) -> E,
    Parse: Fn(PathBuf, serde_json::Error) -> E,
    Unsupported: Fn(u32, u32) -> E,
    Mismatch: Fn(String, String) -> E,
{
    let path = request.path;
    if !path.is_file() {
        return Err((errors.missing_cache)(path));
    }
    let data =
        fs::read_to_string(&path).map_err(|source| (errors.read_cache)(path.clone(), source))?;
    let report = serde_json::from_str::<T>(&data)
        .map_err(|source| (errors.parse_cache)(path.clone(), source))?;
    let actual_schema_version = report.schema_version();
    if actual_schema_version != request.expected_schema_version {
        return Err((errors.unsupported_schema)(
            actual_schema_version,
            request.expected_schema_version,
        ));
    }
    let actual_network = report.network();
    if actual_network != request.network {
        return Err((errors.network_mismatch)(
            request.network.to_string(),
            actual_network.to_string(),
        ));
    }
    Ok(CachedJsonReport { path, report })
}

pub(crate) fn acquire_refresh_lock(
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

pub(crate) fn write_text_atomically(
    target_path: &Path,
    contents: &str,
) -> Result<(), CacheFileError> {
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

pub(crate) fn write_text_output(output_path: &Path, contents: &str) -> Result<(), CacheFileError> {
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

pub(crate) fn write_json_refresh_cache<T, E>(
    request: RefreshCacheWriteRequest<'_, T>,
    cache_error: impl Fn(CacheFileError) -> E,
    serialize_cache: impl Fn(PathBuf, serde_json::Error) -> E,
) -> Result<RefreshCacheWriteResult, E>
where
    T: Serialize,
{
    let cache_dir = request
        .cache_path
        .parent()
        .expect("cache target path always has parent")
        .to_path_buf();
    create_directory(&cache_dir).map_err(&cache_error)?;
    let lock = acquire_refresh_lock(RefreshLockRequest {
        lock_path: request.lock_path,
        target_path: request.cache_path,
        network: request.network,
        now_unix_secs: request.now_unix_secs,
        lock_stale_after_seconds: request.lock_stale_after_seconds,
    })
    .map_err(&cache_error)?;
    let replaced_existing_cache = request.cache_path.is_file();
    let report_json = serde_json::to_string_pretty(request.report)
        .map_err(|source| serialize_cache(request.cache_path.to_path_buf(), source))?;
    if let Some(output_path) = request.output_path {
        write_text_output(output_path, &report_json).map_err(&cache_error)?;
    }
    if !request.dry_run {
        write_text_atomically(request.cache_path, &report_json).map_err(&cache_error)?;
    }
    lock.release().map_err(cache_error)?;
    Ok(RefreshCacheWriteResult {
        cache_path: request.cache_path.display().to_string(),
        refresh_lock_path: request.lock_path.display().to_string(),
        output_path: request.output_path.map(|path| path.display().to_string()),
        replaced_existing_cache,
        wrote_cache: !request.dry_run,
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
