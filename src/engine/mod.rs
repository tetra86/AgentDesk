use anyhow::Result;
use crate::config::Config;
use crate::db::Db;

#[derive(Clone)]
pub struct PolicyEngine {
    _db: Db,
}

impl PolicyEngine {
    pub fn new(config: &Config, db: Db) -> Result<Self> {
        tracing::info!("Policy engine initialized (policies_dir={})", config.policies.dir.display());
        // TODO: Initialize QuickJS runtime and load policies
        Ok(Self { _db: db })
    }
}
