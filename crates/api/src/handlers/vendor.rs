use crate::auth::Claims;
use crate::error::ApiError;
use crate::state::AppState;
use axum::{extract::State, Json};
use domain::{UserId, Vendor};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct CreateVendorRequest {
    pub name: String,
    pub slug: String,
}

/// Any logged-in user can become a vendor by registering a shop. Real
/// systems might gate this behind an approval step; skipped for v1.
pub async fn create_vendor(
    State(state): State<AppState>,
    claims: Claims,
    Json(req): Json<CreateVendorRequest>,
) -> Result<Json<Vendor>, ApiError> {
    let vendor = db::create_vendor(&state.0.pool, UserId(claims.sub), &req.name, &req.slug)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    Ok(Json(vendor))
}
