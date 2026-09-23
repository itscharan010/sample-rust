use sqlx::PgPool;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState(pub Arc<AppStateInner>);

pub struct AppStateInner {
    pub pool: PgPool,
    pub jwt_secret: String,
}

impl AppState {
    pub fn new(pool: PgPool, jwt_secret: String) -> Self {
        AppState(Arc::new(AppStateInner { pool, jwt_secret }))
    }
}
