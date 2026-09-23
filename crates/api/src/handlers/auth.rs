use crate::auth::{hash_password, issue_token, verify_password};
use crate::error::ApiError;
use crate::state::AppState;
use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct AuthResponse {
    pub token: String,
}

pub async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<AuthResponse>, ApiError> {
    if req.password.len() < 8 {
        return Err(ApiError::BadRequest(
            "password must be at least 8 characters".into(),
        ));
    }
    if db::find_user_by_email(&state.0.pool, &req.email)
        .await?
        .is_some()
    {
        return Err(ApiError::BadRequest("email already registered".into()));
    }

    let hash = hash_password(&req.password)?;
    let user = db::create_user(&state.0.pool, &req.email, &hash).await?;
    let token = issue_token(&state.0.jwt_secret, user.id.0, user.is_vendor)?;
    Ok(Json(AuthResponse { token }))
}

pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, ApiError> {
    let user = db::find_user_by_email(&state.0.pool, &req.email)
        .await?
        .ok_or(ApiError::Unauthorized)?;

    if !verify_password(&req.password, &user.password_hash) {
        return Err(ApiError::Unauthorized);
    }

    let token = issue_token(&state.0.jwt_secret, user.id.0, user.is_vendor)?;
    Ok(Json(AuthResponse { token }))
}
