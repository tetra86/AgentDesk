use axum::{Router, routing::get, Json};
use serde_json::json;

use crate::db::Db;
use crate::engine::PolicyEngine;

pub fn api_router(_db: Db, _engine: PolicyEngine) -> Router {
    Router::new()
        .route("/health", get(health))
}

async fn health() -> Json<serde_json::Value> {
    Json(json!({ "ok": true, "version": env!("CARGO_PKG_VERSION") }))
}
