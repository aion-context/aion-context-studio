//! Registry endpoints: list authors + epoch timelines, register, rotate, revoke, export trusted JSON.

use axum::extract::{Path, State};
use axum::http::header;
use axum::response::IntoResponse;
use axum::Json;
use serde::Deserialize;

use studio_core::registry_admin::{self, AuthorView};

use crate::app::AppState;
use crate::error::ApiResult;

/// `GET /api/registry` — every author with its epoch timeline.
pub async fn list(State(st): State<AppState>) -> ApiResult<Json<Vec<AuthorView>>> {
    Ok(Json(registry_admin::list(&st.ws)?))
}

#[derive(Deserialize, Default)]
pub struct RegisterReq {
    #[serde(default)]
    pub author_id: Option<u64>,
}

/// `POST /api/registry/register` — register a new author (optional explicit id).
pub async fn register(
    State(st): State<AppState>,
    Json(req): Json<RegisterReq>,
) -> ApiResult<Json<AuthorView>> {
    Ok(Json(registry_admin::register(&st.ws, req.author_id)?))
}

/// `POST /api/registry/{author}/rotate` — rotate an author's operational key.
pub async fn rotate(
    State(st): State<AppState>,
    Path(author): Path<u64>,
) -> ApiResult<Json<AuthorView>> {
    Ok(Json(registry_admin::rotate(&st.ws, author)?))
}

#[derive(Deserialize)]
pub struct RevokeReq {
    pub reason: String,
}

/// `POST /api/registry/{author}/revoke` — revoke an author's current epoch.
pub async fn revoke(
    State(st): State<AppState>,
    Path(author): Path<u64>,
    Json(req): Json<RevokeReq>,
) -> ApiResult<Json<AuthorView>> {
    Ok(Json(registry_admin::revoke(&st.ws, author, &req.reason)?))
}

/// `GET /api/registry/export` — the trusted-JSON registry as a downloadable file.
pub async fn export(State(st): State<AppState>) -> ApiResult<impl IntoResponse> {
    let json = registry_admin::trusted_json(&st.ws)?;
    Ok((
        [
            (header::CONTENT_TYPE, "application/json"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"registry.trusted.json\"",
            ),
        ],
        json,
    ))
}
