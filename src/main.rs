use std::{collections::HashMap, sync::Arc};

use axum::{
    extract::{Path, State},
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq, PartialOrd)]
pub struct Note {
    title: String,
    note: String,
}

impl Note {
    pub fn new(title: String, note: String) -> Self {
        Self { title, note }
    }
}

#[derive(Default)]
pub struct AppState {
    id: u32,
    data: HashMap<u32, Note>,
}

#[tokio::main]
async fn main() {
    let app_state: Arc<Mutex<AppState>> = Arc::new(Mutex::new(AppState::default()));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    axum::serve(listener, app(app_state)).await.unwrap();
}

fn app(app_state: Arc<Mutex<AppState>>) -> Router {
    Router::new()
        .route("/", get(root_handler))
        .route("/get/:id", get(read_note))
        .route("/create", post(create_note))
        .route("/update/:id", put(update_note))
        .route("/delete/:id", delete(delete_note))
        .with_state(app_state)
}

async fn root_handler() -> Json<String> {
    Json(format!("Available methods are create, get, update, delete"))
}

pub async fn create_note(
    state: State<Arc<Mutex<AppState>>>,
    Json(payload): Json<Note>,
) -> Json<String> {
    let mut state = state.lock().await;
    let new_id = state.id + 1;
    state.data.insert(new_id, payload);
    state.id = new_id;
    Json(format!("Note created with id: {}", new_id))
}

async fn delete_note(state: State<Arc<Mutex<AppState>>>, Path(id): Path<u32>) -> Json<String> {
    let mut state = state.lock().await;
    state.data.remove(&id);
    Json(format!("User deleted with id: {}", id))
}

async fn update_note(
    state: State<Arc<Mutex<AppState>>>,
    Path(id): Path<u32>,
    Json(payload): Json<Note>,
) -> Json<String> {
    let mut state = state.lock().await;
    state.data.insert(id, payload);
    Json("Updated note".into())
}

async fn read_note(
    state: State<Arc<Mutex<AppState>>>,
    Path(id): Path<u32>,
) -> Result<Json<Note>, String> {
    let state = state.lock().await;
    let note = state.data.get(&id).ok_or("Note not found")?.clone();
    Ok(Json(note))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, extract::Request, http::StatusCode};
    use lazy_static::lazy_static;
    use sequential_test::sequential;
    use tower::ServiceExt;

    lazy_static! {
        static ref GLOBAL_STATE: Arc<Mutex<AppState>> = Arc::new(Mutex::new(AppState::default()));
    }

    #[tokio::test]
    #[sequential]
    async fn create() {
        let app_state: Arc<Mutex<AppState>> = GLOBAL_STATE.clone();
        let app = app(app_state.clone());
        let note = Note::new("test_title".into(), "test".into());
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/create")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&note).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let received_note = app_state.lock().await.data.get(&1).unwrap().clone();
        assert_eq!(received_note, note);
    }

    #[tokio::test]
    #[sequential]
    async fn get() {
        let app_state: Arc<Mutex<AppState>> = GLOBAL_STATE.clone();
        let app = app(app_state.clone());
        let note = Note::new("test_title".into(), "test".into());
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/get/1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    #[sequential]
    async fn update() {
        let app_state: Arc<Mutex<AppState>> = GLOBAL_STATE.clone();
        let app = app(app_state.clone());
        let note = Note::new("test_title".into(), "test_updated".into());
        let response = app
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/update/1")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&note).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let received_note = app_state.lock().await.data.get(&1).unwrap().clone();
        assert_eq!(received_note, note);
    }

    #[tokio::test]
    #[sequential]
    async fn delete() {
        let app_state: Arc<Mutex<AppState>> = GLOBAL_STATE.clone();
        let app = app(app_state.clone());
        let response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/delete/1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(app_state.lock().await.data.get(&1), None);
    }
}
