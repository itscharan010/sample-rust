mod auth;
mod error;
mod handlers;
mod state;

use axum::{
    routing::{get, post},
    Router,
};
use state::AppState;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost/marketplace".to_string());
    let jwt_secret =
        std::env::var("JWT_SECRET").unwrap_or_else(|_| "dev-secret-change-me".to_string());

    let pool = db::connect(&database_url).await?;
    sqlx::migrate!("../../migrations").run(&pool).await?;

    let state = AppState::new(pool, jwt_secret);

    let app = Router::new()
        .route("/auth/register", post(handlers::auth::register))
        .route("/auth/login", post(handlers::auth::login))
        .route("/vendors", post(handlers::vendor::create_vendor))
        .route(
            "/products",
            get(handlers::product::list_products).post(handlers::product::create_product),
        )
        .route("/products/:id", get(handlers::product::get_product))
        .route("/orders", post(handlers::order::checkout))
        .route(
            "/orders/:id",
            get(handlers::order::get_order).patch(handlers::order::update_order_status),
        )
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
    tracing::info!("listening on http://0.0.0.0:8080");
    axum::serve(listener, app).await?;

    Ok(())
}
