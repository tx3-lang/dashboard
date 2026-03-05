use axum::{extract::{Path, Query, State}, http::StatusCode, Json};
use serde::Deserialize;

use crate::db;

#[derive(Clone)]
pub struct ApiState {
    pub sqlite_client: sqlx::SqlitePool,
}

#[derive(Debug, Deserialize)]
pub struct ListParams {
    pub limit: Option<i64>,
}

pub async fn list_txs(
    State(state): State<ApiState>,
    Query(params): Query<ListParams>,
) -> Result<Json<Vec<db::TxResponse>>, StatusCode> {
    let limit = params.limit.unwrap_or(100).clamp(1, 500);
    let rows = db::list_txs(&state.sqlite_client, limit)
        .await
        .map_err(|error| {
            tracing::error!(error = %error, "Failed to list txs");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    Ok(Json(rows))
}

pub async fn get_tx(
    State(state): State<ApiState>,
    Path(tx_hash): Path<String>,
) -> Result<Json<db::TxResponse>, StatusCode> {
    let row = db::get_tx(&state.sqlite_client, &tx_hash)
        .await
        .map_err(|error| {
            tracing::error!(error = %error, tx_hash = %tx_hash, "Failed to get tx");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    match row {
        Some(value) => Ok(Json(value)),
        None => Err(StatusCode::NOT_FOUND),
    }
}
