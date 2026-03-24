pub mod routes;
pub mod ws;

use std::sync::Arc;

use anyhow::Result;
use axum::Router;
use axum::routing::get;
use tower_http::services::ServeDir;

use crate::config::Config;
use crate::db::Db;
use crate::engine::PolicyEngine;
use crate::services::discord::health::HealthRegistry;

pub async fn run(config: Config, db: Db, engine: PolicyEngine, health_registry: Option<Arc<HealthRegistry>>) -> Result<()> {
    // Spawn periodic GitHub sync task
    let sync_interval = config.github.sync_interval_minutes;
    if sync_interval > 0 {
        let sync_db = db.clone();
        let sync_engine = engine.clone();
        tokio::spawn(async move {
            github_sync_loop(sync_db, sync_engine, sync_interval).await;
        });
    }

    // Spawn periodic policy tick (fires OnTick every 60s)
    {
        let tick_engine = engine.clone();
        let tick_db = db.clone();
        tokio::spawn(async move {
            policy_tick_loop(tick_engine, tick_db).await;
        });
    }

    // Spawn periodic rate-limit cache sync (every 120s)
    {
        let rl_db = db.clone();
        tokio::spawn(async move {
            rate_limit_sync_loop(rl_db).await;
        });
    }

    // Resolve dashboard dist path relative to runtime root or binary location
    let dashboard_dir = crate::cli::agentdesk_runtime_root()
        .map(|r| r.join("dashboard/dist"))
        .unwrap_or_else(|| std::path::PathBuf::from("dashboard/dist"));
    tracing::info!("Serving dashboard from {:?}", dashboard_dir);

    let broadcast_tx = ws::new_broadcast();

    // Store server port in kv_meta so policy JS can read it
    if let Ok(conn) = db.lock() {
        conn.execute(
            "INSERT OR REPLACE INTO kv_meta (key, value) VALUES ('server_port', ?1)",
            [config.server.port.to_string()],
        )
        .ok();
    }

    let app = Router::new()
        .route("/ws", get(ws::ws_handler).with_state(broadcast_tx.clone()))
        .nest("/api", routes::api_router(db.clone(), engine.clone(), health_registry))
        .fallback_service(ServeDir::new(&dashboard_dir));

    let addr = format!("{}:{}", config.server.host, config.server.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("HTTP server listening on {addr}");
    axum::serve(listener, app).await?;
    Ok(())
}

/// Background task that fires the OnTick policy hook at regular intervals.
async fn policy_tick_loop(engine: PolicyEngine, db: Db) {
    use std::time::Duration;

    let interval = Duration::from_secs(60);
    tracing::info!("[policy-tick] OnTick timer started (every 60s)");

    loop {
        tokio::time::sleep(interval).await;
        if let Err(e) = engine.fire_hook(crate::engine::hooks::Hook::OnTick, serde_json::json!({}))
        {
            tracing::warn!("[policy-tick] OnTick hook error: {e}");
            // Record failure
            if let Ok(conn) = db.lock() {
                conn.execute(
                    "INSERT OR REPLACE INTO kv_meta (key, value) VALUES ('last_tick_status', 'error')",
                    [],
                )
                .ok();
            }
        } else {
            // Record success
            if let Ok(conn) = db.lock() {
                conn.execute(
                    "INSERT OR REPLACE INTO kv_meta (key, value) VALUES ('last_tick_ms', ?1)",
                    [chrono::Utc::now().timestamp_millis().to_string()],
                )
                .ok();
                conn.execute(
                    "INSERT OR REPLACE INTO kv_meta (key, value) VALUES ('last_tick_status', 'ok')",
                    [],
                )
                .ok();
            }
        }
    }
}

/// Background task that periodically fetches rate-limit data from external providers
/// and caches it in the `rate_limit_cache` table for the dashboard API.
async fn rate_limit_sync_loop(db: Db) {
    use std::time::Duration;

    let interval = Duration::from_secs(120);
    // Run immediately on startup, then every 2 minutes
    let mut first = true;

    loop {
        if !first {
            tokio::time::sleep(interval).await;
        }
        first = false;

        // --- Claude (Anthropic API) rate limits ---
        if let Ok(api_key) = std::env::var("ANTHROPIC_API_KEY") {
            match fetch_anthropic_rate_limits(&api_key).await {
                Ok(buckets) => {
                    let data = serde_json::json!({ "buckets": buckets }).to_string();
                    let now = chrono::Utc::now().timestamp();
                    if let Ok(conn) = db.lock() {
                        conn.execute(
                            "INSERT OR REPLACE INTO rate_limit_cache (provider, data, fetched_at) VALUES (?1, ?2, ?3)",
                            rusqlite::params!["claude", data, now],
                        )
                        .ok();
                    }
                    tracing::debug!("[rate-limit-sync] Claude: {} buckets cached", buckets.len());
                }
                Err(e) => {
                    tracing::debug!("[rate-limit-sync] Claude rate_limit fetch failed: {e}");
                }
            }
        }

        // --- Codex (OpenAI API) rate limits ---
        if let Ok(api_key) = std::env::var("OPENAI_API_KEY") {
            match fetch_openai_rate_limits(&api_key).await {
                Ok(buckets) => {
                    let data = serde_json::json!({ "buckets": buckets }).to_string();
                    let now = chrono::Utc::now().timestamp();
                    if let Ok(conn) = db.lock() {
                        conn.execute(
                            "INSERT OR REPLACE INTO rate_limit_cache (provider, data, fetched_at) VALUES (?1, ?2, ?3)",
                            rusqlite::params!["codex", data, now],
                        )
                        .ok();
                    }
                    tracing::debug!("[rate-limit-sync] Codex: {} buckets cached", buckets.len());
                }
                Err(e) => {
                    tracing::debug!("[rate-limit-sync] Codex rate_limit fetch failed: {e}");
                }
            }
        }
    }
}

/// Fetch rate limits from the Anthropic API via the count_tokens endpoint (free, no token cost).
/// Parses `anthropic-ratelimit-*` response headers into bucket format.
async fn fetch_anthropic_rate_limits(
    api_key: &str,
) -> Result<Vec<serde_json::Value>, anyhow::Error> {
    let client = reqwest::Client::new();
    let resp = client
        .post("https://api.anthropic.com/v1/messages/count_tokens")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&serde_json::json!({
            "model": "claude-haiku-4-5-20251001",
            "messages": [{"role": "user", "content": "hi"}]
        }))
        .send()
        .await?;

    let headers = resp.headers().clone();
    let mut buckets = Vec::new();

    // Parse requests bucket
    if let Some(limit) = parse_header_i64(&headers, "anthropic-ratelimit-requests-limit") {
        let remaining =
            parse_header_i64(&headers, "anthropic-ratelimit-requests-remaining").unwrap_or(limit);
        let reset = parse_header_reset(&headers, "anthropic-ratelimit-requests-reset");
        buckets.push(serde_json::json!({
            "name": "requests",
            "limit": limit,
            "used": limit - remaining,
            "remaining": remaining,
            "reset": reset,
        }));
    }

    // Parse tokens bucket
    if let Some(limit) = parse_header_i64(&headers, "anthropic-ratelimit-tokens-limit") {
        let remaining =
            parse_header_i64(&headers, "anthropic-ratelimit-tokens-remaining").unwrap_or(limit);
        let reset = parse_header_reset(&headers, "anthropic-ratelimit-tokens-reset");
        buckets.push(serde_json::json!({
            "name": "tokens",
            "limit": limit,
            "used": limit - remaining,
            "remaining": remaining,
            "reset": reset,
        }));
    }

    Ok(buckets)
}

/// Fetch rate limits from the OpenAI API via the models endpoint (free, read-only).
/// Parses `x-ratelimit-*` response headers into bucket format.
async fn fetch_openai_rate_limits(
    api_key: &str,
) -> Result<Vec<serde_json::Value>, anyhow::Error> {
    let client = reqwest::Client::new();
    let resp = client
        .get("https://api.openai.com/v1/models")
        .header("authorization", format!("Bearer {api_key}"))
        .send()
        .await?;

    let headers = resp.headers().clone();
    let mut buckets = Vec::new();

    // OpenAI rate limit headers: x-ratelimit-limit-requests, x-ratelimit-remaining-requests, etc.
    if let Some(limit) = parse_header_i64(&headers, "x-ratelimit-limit-requests") {
        let remaining =
            parse_header_i64(&headers, "x-ratelimit-remaining-requests").unwrap_or(limit);
        let reset = parse_header_reset(&headers, "x-ratelimit-reset-requests");
        buckets.push(serde_json::json!({
            "name": "requests",
            "limit": limit,
            "used": limit - remaining,
            "remaining": remaining,
            "reset": reset,
        }));
    }

    if let Some(limit) = parse_header_i64(&headers, "x-ratelimit-limit-tokens") {
        let remaining =
            parse_header_i64(&headers, "x-ratelimit-remaining-tokens").unwrap_or(limit);
        let reset = parse_header_reset(&headers, "x-ratelimit-reset-tokens");
        buckets.push(serde_json::json!({
            "name": "tokens",
            "limit": limit,
            "used": limit - remaining,
            "remaining": remaining,
            "reset": reset,
        }));
    }

    Ok(buckets)
}

fn parse_header_i64(headers: &reqwest::header::HeaderMap, name: &str) -> Option<i64> {
    headers.get(name)?.to_str().ok()?.parse().ok()
}

/// Parse ISO 8601 reset timestamp from header into unix epoch seconds.
fn parse_header_reset(headers: &reqwest::header::HeaderMap, name: &str) -> i64 {
    headers
        .get(name)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| {
            chrono::DateTime::parse_from_rfc3339(s)
                .ok()
                .map(|dt| dt.timestamp())
        })
        .unwrap_or(0)
}

/// Background task that periodically syncs GitHub issues for all registered repos.
async fn github_sync_loop(db: Db, engine: crate::engine::PolicyEngine, interval_minutes: u64) {
    use std::time::Duration;

    if !crate::github::gh_available() {
        tracing::warn!("[github-sync] gh CLI not available — periodic sync disabled");
        return;
    }

    tracing::info!(
        "[github-sync] Periodic sync enabled (every {} minutes)",
        interval_minutes
    );

    let interval = Duration::from_secs(interval_minutes * 60);

    loop {
        tokio::time::sleep(interval).await;

        tracing::debug!("[github-sync] Running periodic sync...");

        // Fetch repos
        let repos = match crate::github::list_repos(&db) {
            Ok(r) => r,
            Err(e) => {
                tracing::error!("[github-sync] Failed to list repos: {e}");
                continue;
            }
        };

        for repo in &repos {
            if !repo.sync_enabled {
                continue;
            }

            let issues = match crate::github::sync::fetch_issues(&repo.id) {
                Ok(i) => i,
                Err(e) => {
                    tracing::warn!("[github-sync] Fetch failed for {}: {e}", repo.id);
                    continue;
                }
            };

            // Triage new issues
            match crate::github::triage::triage_new_issues(&db, &repo.id, &issues) {
                Ok(n) if n > 0 => {
                    tracing::info!("[github-sync] Triaged {n} new issues for {}", repo.id);
                }
                Err(e) => {
                    tracing::warn!("[github-sync] Triage failed for {}: {e}", repo.id);
                }
                _ => {}
            }

            // Sync state
            match crate::github::sync::sync_github_issues_for_repo(&db, &engine, &repo.id, &issues) {
                Ok(r) => {
                    if r.closed_count > 0 || r.inconsistency_count > 0 {
                        tracing::info!(
                            "[github-sync] {}: closed={}, inconsistencies={}",
                            repo.id,
                            r.closed_count,
                            r.inconsistency_count
                        );
                    }
                }
                Err(e) => {
                    tracing::error!("[github-sync] Sync failed for {}: {e}", repo.id);
                }
            }
        }
    }
}
