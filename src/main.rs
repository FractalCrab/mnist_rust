mod model;

use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    routing::{get, post},
};
use burn::{prelude::*, tensor::activation::softmax};
use model::mnist_8::Model;
use serde::{Deserialize, Serialize};
use std::{env, sync::Arc};
use tower_http::services::ServeDir;

const IMAGE_SIZE: usize = 28 * 28;

struct AppState {
    device: Device,
    model: Model,
}

#[derive(Deserialize)]
struct PredictRequest {
    pixels: Vec<f32>,
}

#[derive(Serialize)]
struct PredictResponse {
    digit: usize,
    probabilities: Vec<f32>,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: &'static str,
}

async fn predict(
    State(state): State<Arc<AppState>>,
    Json(request): Json<PredictRequest>,
) -> Result<Json<PredictResponse>, (StatusCode, Json<ErrorResponse>)> {
    if request.pixels.len() != IMAGE_SIZE
        || request
            .pixels
            .iter()
            .any(|value| !value.is_finite() || !(0.0..=1.0).contains(value))
    {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "pixels must contain 784 numbers between 0 and 1",
            }),
        ));
    }

    let probabilities = tokio::task::spawn_blocking(move || {
        let data = TensorData::new(request.pixels, [1, 1, 28, 28]);
        let input = Tensor::<4>::from_data(data, &state.device);
        softmax(state.model.forward(input), 1).try_into_vec_as::<f32>()
    })
    .await
    .map_err(|_| internal_error())?
    .map_err(|_| internal_error())?;

    let digit = probabilities
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.total_cmp(b.1))
        .expect("the model has ten outputs")
        .0;
    Ok(Json(PredictResponse {
        digit,
        probabilities,
    }))
}

fn internal_error() -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse {
            error: "inference failed",
        }),
    )
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok" }))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let device = Device::flex();
    let state = Arc::new(AppState {
        model: Model::from_embedded(&device),
        device,
    });
    let app = Router::new()
        .route("/api/health", get(health))
        .route("/api/predict", post(predict))
        .fallback_service(ServeDir::new("frontend/dist"))
        .with_state(state);
    let address = env::var("BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:3000".into());
    let listener = tokio::net::TcpListener::bind(&address).await?;
    println!("Axum listening on http://{address}");
    axum::serve(listener, app).await?;
    Ok(())
}
