use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use serde_json::json;

use super::AppState;

// ── Query / Body types ─────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ListStagesQuery {
    pub repo_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateStageBody {
    pub repo_id: String,
    pub stage_name: String,
    pub stage_order: i64,
    pub trigger_after: String,
    pub entry_skill: Option<String>,
    pub timeout_minutes: Option<i64>,
    pub on_failure: Option<String>,
    pub skip_condition: Option<String>,
}

// ── Handlers ───────────────────────────────────────────────────

/// GET /api/pipeline-stages
pub async fn list_stages(
    State(state): State<AppState>,
    Query(params): Query<ListStagesQuery>,
) -> (StatusCode, Json<serde_json::Value>) {
    let conn = match state.db.lock() {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": format!("{e}")})),
            )
        }
    };

    let mut sql = String::from(
        "SELECT id, repo_id, stage_name, stage_order, trigger_after, entry_skill, timeout_minutes, on_failure, skip_condition FROM pipeline_stages WHERE 1=1",
    );
    let mut bind_values: Vec<String> = Vec::new();

    if let Some(ref repo_id) = params.repo_id {
        bind_values.push(repo_id.clone());
        sql.push_str(&format!(" AND repo_id = ?{}", bind_values.len()));
    }

    sql.push_str(" ORDER BY stage_order ASC");

    let mut stmt = match conn.prepare(&sql) {
        Ok(s) => s,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": format!("prepare: {e}")})),
            )
        }
    };

    let params_ref: Vec<&dyn rusqlite::types::ToSql> =
        bind_values.iter().map(|v| v as &dyn rusqlite::types::ToSql).collect();

    let rows = stmt
        .query_map(params_ref.as_slice(), |row| stage_row_to_json(row))
        .ok();

    let stages: Vec<serde_json::Value> = match rows {
        Some(iter) => iter.filter_map(|r| r.ok()).collect(),
        None => Vec::new(),
    };

    (StatusCode::OK, Json(json!({"stages": stages})))
}

/// POST /api/pipeline-stages
pub async fn create_stage(
    State(state): State<AppState>,
    Json(body): Json<CreateStageBody>,
) -> (StatusCode, Json<serde_json::Value>) {
    let timeout = body.timeout_minutes.unwrap_or(60);
    let on_failure = body.on_failure.unwrap_or_else(|| "fail".to_string());

    let conn = match state.db.lock() {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": format!("{e}")})),
            )
        }
    };

    let result = conn.execute(
        "INSERT INTO pipeline_stages (repo_id, stage_name, stage_order, trigger_after, entry_skill, timeout_minutes, on_failure, skip_condition)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        rusqlite::params![
            body.repo_id,
            body.stage_name,
            body.stage_order,
            body.trigger_after,
            body.entry_skill,
            timeout,
            on_failure,
            body.skip_condition,
        ],
    );

    let row_id = match result {
        Ok(_) => conn.last_insert_rowid(),
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": format!("{e}")})),
            );
        }
    };

    match conn.query_row(
        "SELECT id, repo_id, stage_name, stage_order, trigger_after, entry_skill, timeout_minutes, on_failure, skip_condition FROM pipeline_stages WHERE id = ?1",
        [row_id],
        |row| stage_row_to_json(row),
    ) {
        Ok(stage) => (StatusCode::CREATED, Json(json!({"stage": stage}))),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("{e}")})),
        ),
    }
}

/// DELETE /api/pipeline-stages/:id
pub async fn delete_stage(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> (StatusCode, Json<serde_json::Value>) {
    let conn = match state.db.lock() {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": format!("{e}")})),
            )
        }
    };

    match conn.execute("DELETE FROM pipeline_stages WHERE id = ?1", [id]) {
        Ok(0) => (StatusCode::NOT_FOUND, Json(json!({"error": "stage not found"}))),
        Ok(_) => (StatusCode::OK, Json(json!({"deleted": true, "id": id}))),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("{e}")})),
        ),
    }
}

// ── Helpers ────────────────────────────────────────────────────

fn stage_row_to_json(row: &rusqlite::Row) -> rusqlite::Result<serde_json::Value> {
    Ok(json!({
        "id": row.get::<_, i64>(0)?,
        "repo_id": row.get::<_, Option<String>>(1)?,
        "stage_name": row.get::<_, Option<String>>(2)?,
        "stage_order": row.get::<_, i64>(3)?,
        "trigger_after": row.get::<_, Option<String>>(4)?,
        "entry_skill": row.get::<_, Option<String>>(5)?,
        "timeout_minutes": row.get::<_, i64>(6)?,
        "on_failure": row.get::<_, Option<String>>(7)?,
        "skip_condition": row.get::<_, Option<String>>(8)?,
    }))
}
