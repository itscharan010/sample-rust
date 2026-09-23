use crate::auth::Claims;
use crate::error::ApiError;
use crate::state::AppState;
use axum::{
    extract::{Path, State},
    Json,
};
use domain::{Order, OrderId, OrderStatus, ProductId, UserId};
use serde::Deserialize;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct LineItemRequest {
    pub product_id: Uuid,
    pub quantity: i32,
}

#[derive(Deserialize)]
pub struct CheckoutRequest {
    pub items: Vec<LineItemRequest>,
}

/// v1: places an order. All items must belong to the same vendor — the DB
/// layer enforces this and rejects mixed-vendor carts. Payment capture
/// against Stripe is intentionally not wired up yet; the order is created
/// as `Placed` and a separate step (not implemented here) would confirm
/// payment and transition it to `Paid`.
pub async fn checkout(
    State(state): State<AppState>,
    claims: Claims,
    Json(req): Json<CheckoutRequest>,
) -> Result<Json<Order>, ApiError> {
    if req.items.is_empty() {
        return Err(ApiError::BadRequest("cart is empty".into()));
    }

    let line_items: Vec<(ProductId, i32)> = req
        .items
        .iter()
        .map(|i| (ProductId(i.product_id), i.quantity))
        .collect();

    let order = db::place_single_vendor_order(&state.0.pool, UserId(claims.sub), &line_items)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    Ok(Json(order))
}

pub async fn get_order(
    State(state): State<AppState>,
    claims: Claims,
    Path(id): Path<Uuid>,
) -> Result<Json<Order>, ApiError> {
    let order = db::get_order(&state.0.pool, OrderId(id))
        .await?
        .ok_or(ApiError::NotFound)?;

    // Only the buyer can view their own order for now (vendor-side order
    // views would need a separate "orders for my vendor" query).
    if order.buyer_user_id.0 != claims.sub {
        return Err(ApiError::Unauthorized);
    }
    Ok(Json(order))
}

#[derive(Deserialize)]
pub struct UpdateStatusRequest {
    pub status: OrderStatus,
}

pub async fn update_order_status(
    State(state): State<AppState>,
    _claims: Claims, // TODO: restrict to the owning vendor once vendor-order lookup exists
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateStatusRequest>,
) -> Result<Json<Order>, ApiError> {
    let order = db::update_order_status(&state.0.pool, OrderId(id), req.status)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    Ok(Json(order))
}
