mod config;
mod db;
mod discord;
mod dispatch;
mod engine;
mod github;
mod server;
mod session;

use anyhow::Result;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("agentdesk=info".parse()?))
        .init();

    let config = config::load()?;
    let db = db::init(&config)?;
    let engine = engine::PolicyEngine::new(&config, db.clone())?;

    tracing::info!(
        "AgentDesk v{} starting on {}:{}",
        env!("CARGO_PKG_VERSION"),
        config.server.host,
        config.server.port
    );

    // Start subsystems concurrently
    tokio::try_join!(
        server::run(config.clone(), db.clone(), engine.clone()),
        // discord::run(config.clone(), db.clone(), engine.clone()),
        // session::run(config.clone(), db.clone()),
    )?;

    Ok(())
}
