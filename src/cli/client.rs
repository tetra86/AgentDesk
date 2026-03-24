//! CLI client subcommands that call the AgentDesk HTTP API.

use crate::config;

/// Resolve the API base URL from config or environment.
pub fn api_base() -> String {
    if let Ok(url) = std::env::var("AGENTDESK_API_URL") {
        return url.trim_end_matches('/').to_string();
    }
    let cfg = config::load_graceful();
    format!("http://127.0.0.1:{}", cfg.server.port)
}

/// Build a ureq agent (shared across calls).
fn agent() -> ureq::Agent {
    ureq::Agent::new()
}

/// Get the auth token from config.
fn auth_token() -> Option<String> {
    let cfg = config::load_graceful();
    cfg.server.auth_token.clone()
}

/// Make a GET request and print JSON.
fn get(path: &str) -> Result<(), String> {
    let url = format!("{}{}", api_base(), path);
    let mut req = agent().get(&url);
    if let Some(token) = auth_token() {
        req = req.set("Authorization", &format!("Bearer {token}"));
    }
    let resp = req.call().map_err(|e| format!("Request failed: {e}"))?;
    let body: serde_json::Value = resp.into_json().map_err(|e| format!("Parse error: {e}"))?;
    println!("{}", serde_json::to_string_pretty(&body).unwrap());
    Ok(())
}

/// Make a POST request with optional JSON body and print response.
fn post(path: &str, body: Option<serde_json::Value>) -> Result<(), String> {
    let url = format!("{}{}", api_base(), path);
    let mut req = agent().post(&url);
    if let Some(token) = auth_token() {
        req = req.set("Authorization", &format!("Bearer {token}"));
    }
    let resp = match body {
        Some(b) => req
            .set("Content-Type", "application/json")
            .send_string(&b.to_string())
            .map_err(|e| format!("Request failed: {e}"))?,
        None => req
            .set("Content-Type", "application/json")
            .send_string("{}")
            .map_err(|e| format!("Request failed: {e}"))?,
    };
    let body: serde_json::Value = resp.into_json().map_err(|e| format!("Parse error: {e}"))?;
    println!("{}", serde_json::to_string_pretty(&body).unwrap());
    Ok(())
}

/// Generic API call (for `agentdesk api <method> <path>`)
fn api_call(method: &str, path: &str, body: Option<&str>) -> Result<(), String> {
    let url = if path.starts_with('/') {
        format!("{}{}", api_base(), path)
    } else {
        format!("{}/{}", api_base(), path)
    };

    let a = agent();
    let mut req = match method.to_uppercase().as_str() {
        "GET" => a.get(&url),
        "POST" => a.post(&url),
        "PATCH" => a.patch(&url),
        "PUT" => a.put(&url),
        "DELETE" => a.delete(&url),
        other => return Err(format!("Unsupported method: {other}")),
    };
    if let Some(token) = auth_token() {
        req = req.set("Authorization", &format!("Bearer {token}"));
    }

    let resp = if let Some(b) = body {
        req.set("Content-Type", "application/json")
            .send_string(b)
            .map_err(|e| format!("Request failed: {e}"))?
    } else if matches!(method.to_uppercase().as_str(), "POST" | "PATCH" | "PUT") {
        req.set("Content-Type", "application/json")
            .send_string("{}")
            .map_err(|e| format!("Request failed: {e}"))?
    } else {
        req.call().map_err(|e| format!("Request failed: {e}"))?
    };

    let body_val: serde_json::Value =
        resp.into_json().map_err(|e| format!("Parse error: {e}"))?;
    println!("{}", serde_json::to_string_pretty(&body_val).unwrap());
    Ok(())
}

// ── Subcommand handlers ──────────────────────────────────────

/// `agentdesk status` — server health + auto-queue status
pub fn cmd_status() {
    println!("=== Server Health ===");
    if let Err(e) = get("/api/health") {
        eprintln!("  ✗ {e}");
        return;
    }
    println!("\n=== Auto-Queue Status ===");
    if let Err(e) = get("/api/auto-queue/status") {
        eprintln!("  ✗ {e}");
    }
}

/// `agentdesk cards [--status <STATUS>]`
pub fn cmd_cards(status: Option<&str>) {
    let path = match status {
        Some(s) => format!("/api/kanban-cards?status={s}"),
        None => "/api/kanban-cards".to_string(),
    };
    if let Err(e) = get(&path) {
        eprintln!("Error: {e}");
    }
}

/// `agentdesk dispatch list`
pub fn cmd_dispatch_list() {
    if let Err(e) = get("/api/dispatches") {
        eprintln!("Error: {e}");
    }
}

/// `agentdesk dispatch retry <card_id>`
pub fn cmd_dispatch_retry(card_id: &str) {
    if let Err(e) = post(
        &format!("/api/kanban-cards/{card_id}/retry"),
        Some(serde_json::json!({})),
    ) {
        eprintln!("Error: {e}");
    }
}

/// `agentdesk dispatch redispatch <card_id>`
pub fn cmd_dispatch_redispatch(card_id: &str) {
    if let Err(e) = post(
        &format!("/api/kanban-cards/{card_id}/redispatch"),
        Some(serde_json::json!({})),
    ) {
        eprintln!("Error: {e}");
    }
}

/// `agentdesk agents`
pub fn cmd_agents() {
    if let Err(e) = get("/api/agents") {
        eprintln!("Error: {e}");
    }
}

/// `agentdesk config get`
pub fn cmd_config_get() {
    if let Err(e) = get("/api/settings/runtime-config") {
        eprintln!("Error: {e}");
    }
}

/// `agentdesk config set <json>`
pub fn cmd_config_set(json_str: &str) {
    let body: serde_json::Value = match serde_json::from_str(json_str) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Invalid JSON: {e}");
            return;
        }
    };
    let url = format!("{}/api/settings/runtime-config", api_base());
    let a = agent();
    let mut req = a.put(&url);
    if let Some(token) = auth_token() {
        req = req.set("Authorization", &format!("Bearer {token}"));
    }
    match req
        .set("Content-Type", "application/json")
        .send_string(&body.to_string())
    {
        Ok(resp) => {
            let val: serde_json::Value = resp.into_json().unwrap_or(serde_json::json!({"ok": true}));
            println!("{}", serde_json::to_string_pretty(&val).unwrap());
        }
        Err(e) => eprintln!("Error: {e}"),
    }
}

/// `agentdesk api <method> <path> [body]`
pub fn cmd_api(method: &str, path: &str, body: Option<&str>) {
    if let Err(e) = api_call(method, path, body) {
        eprintln!("Error: {e}");
    }
}
