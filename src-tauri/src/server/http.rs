use axum::{
    routing::{get, post},
    http::StatusCode,
    response::IntoResponse,
    Json, Router,
    extract::State,
};
use std::net::SocketAddr;
use std::sync::Arc;
use crate::compiler::php_embed::PHPEmbed;

pub struct AppState {
    pub php: Option<Arc<PHPEmbed>>,
    pub document_root: String,
}

pub async fn start_server(state: AppState) -> Result<u16, anyhow::Error> {
    let serve_dir = tower_http::services::ServeDir::new(&state.document_root);
    let app_state = Arc::new(state);

    let app = Router::new()
        .route("/", get(root_handler))
        .route("/execute", post(php_handler))
        .fallback_service(serve_dir)
        .with_state(app_state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 0)); // Dynamic port
    let server = axum::Server::bind(&addr).serve(app.into_make_service());
    
    let local_addr = server.local_addr();
    let port = local_addr.port();

    tokio::spawn(async move {
        if let Err(e) = server.await {
            eprintln!("Server error: {}", e);
        }
    });

    Ok(port)
}

async fn root_handler() -> &'static str {
    "ExeOutput PHP server is running"
}

async fn php_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<PhpRequest>,
) -> impl IntoResponse {
    if let Some(php) = &state.php {
        match php.eval(&payload.code) {
            Ok(result) => (StatusCode::OK, result),
            Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        }
    } else {
        (StatusCode::INTERNAL_SERVER_ERROR, "PHP not loaded".to_string())
    }
}

#[derive(serde::Deserialize)]
struct PhpRequest {
    code: String,
}
