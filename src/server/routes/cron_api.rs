use axum::{Json, extract::State, http::StatusCode};
use serde_json::json;

use super::AppState;

/// Build cron job list from policy engine's onTick handlers.
fn build_cron_jobs(state: &AppState, agent_filter: Option<&str>) -> Vec<serde_json::Value> {
    let policies = state.engine.list_policies();
    let now_ms = chrono::Utc::now().timestamp_millis();

    policies
        .iter()
        .filter(|p| p.hooks.iter().any(|h| h == "onTick"))
        .filter(|p| {
            if let Some(agent_id) = agent_filter {
                // Only include policies relevant to this agent
                // All onTick policies are global, so include all unless we add agent-scoping later
                let _ = agent_id;
                true
            } else {
                true
            }
        })
        .map(|p| {
            let (description, description_ko) = match p.name.as_str() {
                "timeouts" => (
                    "Timeout detection — auto-handle stale requested/in_progress cards",
                    "타임아웃 감지 — requested/in_progress 스테일 카드 자동 처리",
                ),
                "auto-queue" => (
                    "Auto-queue progression — sequential dispatch from queue",
                    "자동 큐 진행 — 큐 엔트리 순차 디스패치",
                ),
                "triage-rules" => (
                    "Auto-triage — GitHub issue label-based agent assignment",
                    "자동 분류 — GitHub 이슈 라벨 기반 에이전트 할당",
                ),
                _ => ("", ""),
            };
            json!({
                "id": format!("policy:{}", p.name),
                "name": format!("policy/{} → onTick", p.name),
                "description_ko": description_ko,
                "enabled": true,
                "schedule": {
                    "kind": "every",
                    "everyMs": 60000,
                },
                "state": {
                    "status": "active",
                    "lastStatus": "ok",
                    "lastRunAtMs": now_ms - 30000, // approximate
                    "nextRunAtMs": now_ms + 30000,
                },
            })
        })
        .collect()
}

/// GET /api/cron-jobs
pub async fn list_cron_jobs(
    State(state): State<AppState>,
) -> (StatusCode, Json<serde_json::Value>) {
    let jobs = build_cron_jobs(&state, None);
    (StatusCode::OK, Json(json!({ "jobs": jobs })))
}

/// GET /api/agents/{id}/cron — agent-specific cron jobs
pub async fn agent_cron_jobs(
    State(state): State<AppState>,
    axum::extract::Path(agent_id): axum::extract::Path<String>,
) -> (StatusCode, Json<serde_json::Value>) {
    let jobs = build_cron_jobs(&state, Some(&agent_id));
    (StatusCode::OK, Json(json!({ "jobs": jobs })))
}
