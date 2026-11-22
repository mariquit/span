use anyhow::{Context, Result};
use directories::BaseDirs;
use std::path::PathBuf;

#[derive(Debug, Clone, serde::Serialize)]
pub struct RuntimePaths {
    pub database: PathBuf,
    pub daemon_pid: PathBuf,
    pub model_cache: PathBuf,
}

impl RuntimePaths {
    pub fn discover() -> Result<Self> {
        let base =
            BaseDirs::new().context("could not determine the current user's home directories")?;
        let database = std::env::var_os("SPAN_DB")
            .map(PathBuf::from)
            .unwrap_or_else(|| base.data_local_dir().join("span").join("memory.db"));
        let model_cache = std::env::var_os("SPAN_MODEL_CACHE")
            .map(PathBuf::from)
            .unwrap_or_else(|| base.cache_dir().join("span").join("models"));
        Ok(Self {
            database,
            daemon_pid: base.data_local_dir().join("span/span.pid"),
            model_cache,
        })
    }
}
