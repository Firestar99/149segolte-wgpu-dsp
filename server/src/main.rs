use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
};
use serde::Serialize;

use app::{Application, GalaxyResponse, GalaxyRules, SearchStatus, StarRules};

mod app;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let state = Arc::new(RwLock::new(Application::new()));

    let router = Router::new()
        .route("/", get(root))
        .route("/galaxy/{seed}", get(galaxy))
        .route("/find/star", post(find_star))
        .route("/find/galaxy", post(find_galaxy))
        .route("/find/star/{hash}", get(find_star_status))
        .route("/find/galaxy/{hash}", get(find_galaxy_status))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    axum::serve(listener, router).await.unwrap();
}

#[derive(Debug, Clone, Serialize)]
struct Message<T> {
    status: String,
    data: T,
}

#[derive(Debug, Clone, Serialize)]
enum SearchResponse {
    Star(Vec<(u32, u8)>),
    Galaxy(Vec<u32>),
    Text(String),
}

async fn root() -> (StatusCode, Json<Message<String>>) {
    (
        StatusCode::OK,
        Json(Message {
            status: "running".to_string(),
            data: "".to_string(),
        }),
    )
}

async fn galaxy(
    State(state): State<Arc<RwLock<Application>>>,
    Path(seed): Path<u32>,
    Query(params): Query<HashMap<String, String>>,
) -> (StatusCode, Json<Message<GalaxyResponse>>) {
    let star_count: u8 = params
        .get("star_count")
        .unwrap_or(&"64".to_string())
        .parse()
        .unwrap();
    let resource_multiplier: f32 = params
        .get("resource_multiplier")
        .unwrap_or(&"1.0".to_string())
        .parse()
        .unwrap();
    (
        StatusCode::OK,
        Json(Message {
            status: "ok".to_string(),
            data: state
                .read()
                .unwrap()
                .galaxy_details(seed, star_count, resource_multiplier),
        }),
    )
}

async fn find_star(
    State(state): State<Arc<RwLock<Application>>>,
    Json(payload): Json<StarRules>,
) -> (StatusCode, Json<Message<String>>) {
    (
        StatusCode::OK,
        Json(Message {
            status: "ok".to_string(),
            data: state.read().unwrap().find_star(payload),
        }),
    )
}

async fn find_galaxy(
    State(state): State<Arc<RwLock<Application>>>,
    Json(payload): Json<GalaxyRules>,
) -> (StatusCode, Json<Message<String>>) {
    (
        StatusCode::OK,
        Json(Message {
            status: "ok".to_string(),
            data: state.read().unwrap().find_galaxy(payload),
        }),
    )
}

async fn find_star_status(
    State(state): State<Arc<RwLock<Application>>>,
    Path(hash): Path<String>,
) -> (StatusCode, Json<Message<SearchResponse>>) {
    match state.read().unwrap().find_star_status(hash) {
        Some(x) => match x {
            SearchStatus::StarSearch(data) => (
                StatusCode::OK,
                Json(Message {
                    status: "ok".to_string(),
                    data: SearchResponse::Star(data),
                }),
            ),
            SearchStatus::Running => (
                StatusCode::ACCEPTED,
                Json(Message {
                    status: "running".to_string(),
                    data: SearchResponse::Text("".to_string()),
                }),
            ),
            SearchStatus::GalaxySearch(_) => unreachable!(),
        },
        None => (
            StatusCode::NOT_FOUND,
            Json(Message {
                status: "not found".to_string(),
                data: SearchResponse::Text("".to_string()),
            }),
        ),
    }
}

async fn find_galaxy_status(
    State(state): State<Arc<RwLock<Application>>>,
    Path(hash): Path<String>,
) -> (StatusCode, Json<Message<SearchResponse>>) {
    match state.read().unwrap().find_galaxy_status(hash) {
        Some(x) => match x {
            SearchStatus::GalaxySearch(data) => (
                StatusCode::OK,
                Json(Message {
                    status: "ok".to_string(),
                    data: SearchResponse::Galaxy(data),
                }),
            ),
            SearchStatus::Running => (
                StatusCode::ACCEPTED,
                Json(Message {
                    status: "running".to_string(),
                    data: SearchResponse::Text("".to_string()),
                }),
            ),
            SearchStatus::StarSearch(_) => unreachable!(),
        },
        None => (
            StatusCode::NOT_FOUND,
            Json(Message {
                status: "not found".to_string(),
                data: SearchResponse::Text("".to_string()),
            }),
        ),
    }
}
