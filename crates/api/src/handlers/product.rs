use crate::auth::Claims;
use crate::error::ApiError;
use crate::state::AppState;
use axum::{
    extract::{Path, State},
    Json,
};
use domain::{Product, ProductId, VendorId};
use serde::Deserialize;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct CreateProductRequest {
    pub vendor_id: Uuid,
    pub title: String,
    pub description: String,
    pub price_cents: i64,
    pub inventory_count: i32,
}

pub async fn create_product(
    State(state): State<AppState>,
    claims: Claims,
    Json(req): Json<CreateProductRequest>,
) -> Result<Json<Product>, ApiError> {
    let vendor = db::get_vendor(&state.0.pool, VendorId(req.vendor_id))
        .await?
        .ok_or(ApiError::NotFound)?;

    if vendor.owner_user_id.0 != claims.sub {
        return Err(ApiError::Unauthorized);
    }
    if req.price_cents < 0 {
        return Err(ApiError::BadRequest("price_cents must be >= 0".into()));
    }

    let product = db::create_product(
        &state.0.pool,
        VendorId(req.vendor_id),
        &req.title,
        &req.description,
        req.price_cents,
        req.inventory_count,
    )
    .await?;
    Ok(Json(product))
}

/// Public — no auth required to browse the catalog.
pub async fn list_products(
    State(state): State<AppState>,
) -> Result<Json<Vec<Product>>, ApiError> {
    let products = db::list_active_products(&state.0.pool).await?;
    Ok(Json(products))
}

pub async fn get_product(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Product>, ApiError> {
    let product = db::get_product(&state.0.pool, ProductId(id))
        .await?
        .ok_or(ApiError::NotFound)?;
    Ok(Json(product))
}
