use axum::Router;
use axum::routing::get;
use tower_http::cors::{Any, CorsLayer};

pub async fn ping() -> &'static str {
    // tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    "PONG"
}

pub fn crete_router() -> Router {
    let cors = CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any);
    Router::new().route("/ping", get(ping)).layer(cors)
}
