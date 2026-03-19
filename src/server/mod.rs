pub mod routes;

use anyhow::Result;
use axum::Router;
use tower_http::services::ServeDir;

use crate::config::Config;
use crate::db::Db;
use crate::engine::PolicyEngine;

pub async fn run(config: Config, db: Db, engine: PolicyEngine) -> Result<()> {
    let app = Router::new()
        .nest("/api", routes::api_router(db.clone(), engine.clone()))
        .fallback_service(ServeDir::new("dashboard/dist"));

    let addr = format!("{}:{}", config.server.host, config.server.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("HTTP server listening on {addr}");
    axum::serve(listener, app).await?;
    Ok(())
}
