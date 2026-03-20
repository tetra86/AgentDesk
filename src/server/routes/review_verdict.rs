use axum::{extract::State, http::StatusCode, Json};
use serde::Deserialize;
use serde_json::json;

use super::AppState;

#[derive(Debug, Deserialize)]
pub struct VerdictItem {
    pub category: Option<String>,
    pub summary: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SubmitVerdictBody {
    pub dispatch_id: String,
    pub overall: String,
    pub items: Option<Vec<VerdictItem>>,
}

/// POST /api/review-verdict
pub async fn submit_verdict(
    State(state): State<AppState>,
    Json(body): Json<SubmitVerdictBody>,
) -> (StatusCode, Json<serde_json::Value>) {
    // Validate overall value
    if body.overall != "pass" && body.overall != "improve" {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "overall must be 'pass' or 'improve'"})),
        );
    }

    let conn = match state.db.lock() {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": format!("{e}")})),
            )
        }
    };

    // Build result JSON
    let result_json = json!({
        "overall": body.overall,
        "items": body.items.as_ref().map(|items| {
            items.iter().map(|it| json!({
                "category": it.category,
                "summary": it.summary,
            })).collect::<Vec<_>>()
        }).unwrap_or_default(),
    });
    let result_str = result_json.to_string();

    // Update task_dispatches
    let updated = match conn.execute(
        "UPDATE task_dispatches SET status = 'completed', result = ?2 WHERE id = ?1",
        rusqlite::params![body.dispatch_id, result_str],
    ) {
        Ok(n) => n,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": format!("update dispatch: {e}")})),
            )
        }
    };

    if updated == 0 {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "dispatch not found"})),
        );
    }

    // If overall is "pass", update the related kanban card to 'done'
    if body.overall == "pass" {
        let card_id: Option<String> = conn
            .query_row(
                "SELECT kanban_card_id FROM task_dispatches WHERE id = ?1",
                [&body.dispatch_id],
                |row| row.get(0),
            )
            .ok()
            .flatten();

        if let Some(cid) = card_id {
            let _ = conn.execute(
                "UPDATE kanban_cards SET status = 'done' WHERE id = ?1 AND status = 'review'",
                [&cid],
            );
        }
    }

    (
        StatusCode::OK,
        Json(json!({
            "ok": true,
            "dispatch_id": body.dispatch_id,
            "overall": body.overall,
        })),
    )
}
