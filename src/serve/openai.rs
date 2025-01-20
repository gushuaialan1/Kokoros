use axum::{
    routing::post,
    Router,
    Json,
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::get,
};
use serde::{Deserialize, Serialize};
use tower_http::cors::{Any, CorsLayer};
use crate::tts::koko::TTSKoko;
use std::sync::Arc;

#[derive(Deserialize)]
struct TTSRequest {
    model: String,
    input: String,
    voice: Option<String>,
    language: Option<String>,
}

#[derive(Serialize)]
struct TTSResponse {
    status: String,
    file_path: String,
}

#[derive(Clone)]
pub struct AppState {
    tts: Arc<TTSKoko>,
}

async fn health_check() -> &'static str {
    "OK"
}

async fn text_to_speech(
    State(state): State<AppState>,
    Json(payload): Json<TTSRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    let voice = payload.voice.unwrap_or_else(|| "af_sky".to_string());
    
    // 获取语言设置，如果未指定则自动检测
    let lang = if let Some(lang) = payload.language {
        lang
    } else {
        // 自动检测语言
        if payload.input.chars().any(|c| c as u32 >= 0x4E00 && c as u32 <= 0x9FFF) {
            "zh-cn".to_string()
        } else if payload.input.chars().any(|c| c as u32 >= 0x3040 && c as u32 <= 0x30FF) {
            "ja-jp".to_string()
        } else if payload.input.chars().any(|c| c as u32 >= 0x0400 && c as u32 <= 0x04FF) {
            "de-de".to_string()
        } else {
            "en-us".to_string()
        }
    };
    
    // Generate unique output filename
    let output_path = format!("output_{}.wav", std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs());

    // Process TTS request with language
    if let Err(_) = state.tts.tts(&payload.input, &lang, &voice) {
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    Ok(Json(TTSResponse {
        status: "success".to_string(),
        file_path: output_path,
    }))
}

pub async fn create_server(tts: TTSKoko) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app_state = AppState { 
        tts: Arc::new(tts)
    };

    Router::new()
        .route("/", get(health_check))
        .route("/v1/audio/speech", post(text_to_speech))
        .layer(cors)
        .with_state(app_state)
}
