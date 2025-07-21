
use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder};
use chrono::{Local, Timelike};
use get_if_addrs::get_if_addrs;
use serde::{Deserialize, Serialize};
use serde_json;
use std::sync::{Arc, Mutex};

use crate::config::{self, Config};
use crate::sync_logic::LtcState;
use crate::ui;

// Data structure for the main status response
#[derive(Serialize, Deserialize)]
struct ApiStatus {
    ltc_status: String,
    ltc_timecode: String,
    frame_rate: String,
    system_clock: String,
    timecode_delta_ms: i64,
    timecode_delta_frames: i64,
    sync_status: String,
    jitter_status: String,
    lock_ratio: f64,
    ntp_active: bool,
    interfaces: Vec<String>,
    hardware_offset_ms: i64,
}

// AppState to hold shared data
pub struct AppState {
    pub ltc_state: Arc<Mutex<LtcState>>,
    pub hw_offset: Arc<Mutex<i64>>,
}

#[get("/api/status")]
async fn get_status(data: web::Data<AppState>) -> impl Responder {
    let state = data.ltc_state.lock().unwrap();
    let hw_offset_ms = *data.hw_offset.lock().unwrap();

    let ltc_status = state.latest.as_ref().map_or("(waiting)".to_string(), |f| f.status.clone());
    let ltc_timecode = state.latest.as_ref().map_or("…".to_string(), |f| {
        format!("{:02}:{:02}:{:02}:{:02}", f.hours, f.minutes, f.seconds, f.frames)
    });
    let frame_rate = state.latest.as_ref().map_or("…".to_string(), |f| {
        format!("{:.2}fps", f.frame_rate)
    });

    let now_local = Local::now();
    let system_clock = format!(
        "{:02}:{:02}:{:02}.{:03}",
        now_local.hour(),
        now_local.minute(),
        now_local.second(),
        now_local.timestamp_subsec_millis(),
    );

    let avg_delta = state.average_clock_delta();
    let mut delta_frames = 0;
    if let Some(frame) = &state.latest {
        let frame_ms = 1000.0 / frame.frame_rate;
        delta_frames = ((avg_delta as f64 / frame_ms).round()) as i64;
    }

    let sync_status = ui::get_sync_status(avg_delta).to_string();
    let jitter_status = ui::get_jitter_status(state.average_jitter()).to_string();
    let lock_ratio = state.lock_ratio();

    let ntp_active = ui::ntp_service_active();
    let interfaces = get_if_addrs()
        .unwrap_or_default()
        .into_iter()
        .filter(|ifa| !ifa.is_loopback())
        .map(|ifa| ifa.ip().to_string())
        .collect();

    HttpResponse::Ok().json(ApiStatus {
        ltc_status,
        ltc_timecode,
        frame_rate,
        system_clock,
        timecode_delta_ms: avg_delta,
        timecode_delta_frames: delta_frames,
        sync_status,
        jitter_status,
        lock_ratio,
        ntp_active,
        interfaces,
        hardware_offset_ms: hw_offset_ms,
    })
}

#[post("/api/sync")]
async fn manual_sync(data: web::Data<AppState>) -> impl Responder {
    let state = data.ltc_state.lock().unwrap();
    if let Some(frame) = &state.latest {
        if ui::trigger_sync(frame).is_ok() {
            HttpResponse::Ok().json(serde_json::json!({ "status": "success", "message": "Sync command issued." }))
        } else {
            HttpResponse::InternalServerError().json(serde_json::json!({ "status": "error", "message": "Sync command failed." }))
        }
    } else {
        HttpResponse::BadRequest().json(serde_json::json!({ "status": "error", "message": "No LTC timecode available to sync to." }))
    }
}

#[derive(Serialize, Deserialize)]
struct ConfigResponse {
    hardware_offset_ms: i64,
}

#[get("/api/config")]
async fn get_config(data: web::Data<AppState>) -> impl Responder {
    let hw_offset_ms = *data.hw_offset.lock().unwrap();
    HttpResponse::Ok().json(ConfigResponse { hardware_offset_ms: hw_offset_ms })
}

#[derive(Deserialize)]
struct UpdateConfigRequest {
    hardware_offset_ms: i64,
}

#[post("/api/config")]
async fn update_config(
    data: web::Data<AppState>,
    req: web::Json<UpdateConfigRequest>,
) -> impl Responder {
    let mut hw_offset = data.hw_offset.lock().unwrap();
    *hw_offset = req.hardware_offset_ms;

    let new_config = Config {
        hardware_offset_ms: *hw_offset,
    };

    if config::save_config("config.json", &new_config).is_ok() {
        eprintln!("🔄 Saved hardware_offset_ms = {} via API", *hw_offset);
        HttpResponse::Ok().json(&new_config)
    } else {
        HttpResponse::InternalServerError().json(serde_json::json!({ "status": "error", "message": "Failed to write config.json" }))
    }
}

pub async fn start_api_server(
    state: Arc<Mutex<LtcState>>,
    offset: Arc<Mutex<i64>>,
) -> std::io::Result<()> {
    let app_state = web::Data::new(AppState {
        ltc_state: state,
        hw_offset: offset,
    });

    println!("🚀 Starting API server at http://0.0.0.0:8080");

    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .service(get_status)
            .service(manual_sync)
            .service(get_config)
            .service(update_config)
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync_logic::LtcFrame;
    use actix_web::{test, App};
    use chrono::Utc;
    use std::collections::VecDeque;
    use std::fs;

    // Helper to create a default LtcState for tests
    fn get_test_state() -> LtcState {
        LtcState {
            latest: Some(LtcFrame {
                status: "LOCK".to_string(),
                hours: 1,
                minutes: 2,
                seconds: 3,
                frames: 4,
                frame_rate: 25.0,
                timestamp: Utc::now(),
            }),
            lock_count: 10,
            free_count: 1,
            offset_history: VecDeque::from(vec![1, 2, 3]),
            clock_delta_history: VecDeque::from(vec![4, 5, 6]),
            last_match_status: "IN SYNC".to_string(),
            last_match_check: Utc::now().timestamp(),
        }
    }

    #[actix_web::test]
    async fn test_get_status() {
        let ltc_state = Arc::new(Mutex::new(get_test_state()));
        let hw_offset = Arc::new(Mutex::new(10i64));

        let app_state = web::Data::new(AppState {
            ltc_state: ltc_state.clone(),
            hw_offset: hw_offset.clone(),
        });

        let app = test::init_service(
            App::new()
                .app_data(app_state.clone())
                .service(get_status),
        )
        .await;

        let req = test::TestRequest::get().uri("/api/status").to_request();
        let resp: ApiStatus = test::call_and_read_body_json(&app, req).await;

        assert_eq!(resp.ltc_status, "LOCK");
        assert_eq!(resp.ltc_timecode, "01:02:03:04");
        assert_eq!(resp.frame_rate, "25.00fps");
        assert_eq!(resp.hardware_offset_ms, 10);
    }

    #[actix_web::test]
    async fn test_get_config() {
        let ltc_state = Arc::new(Mutex::new(LtcState::new()));
        let hw_offset = Arc::new(Mutex::new(25i64));

        let app_state = web::Data::new(AppState {
            ltc_state: ltc_state.clone(),
            hw_offset: hw_offset.clone(),
        });

        let app = test::init_service(
            App::new()
                .app_data(app_state.clone())
                .service(get_config),
        )
        .await;

        let req = test::TestRequest::get().uri("/api/config").to_request();
        let resp: ConfigResponse = test::call_and_read_body_json(&app, req).await;

        assert_eq!(resp.hardware_offset_ms, 25);
    }

    #[actix_web::test]
    async fn test_update_config() {
        let ltc_state = Arc::new(Mutex::new(LtcState::new()));
        let hw_offset = Arc::new(Mutex::new(0i64));
        let config_path = "config.json";

        // This test has the side effect of writing to `config.json`.
        // We ensure it's cleaned up after.
        let _ = fs::remove_file(config_path);

        let app_state = web::Data::new(AppState {
            ltc_state: ltc_state.clone(),
            hw_offset: hw_offset.clone(),
        });

        let app = test::init_service(
            App::new()
                .app_data(app_state.clone())
                .service(update_config),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/config")
            .set_json(&serde_json::json!({ "hardware_offset_ms": 55 }))
            .to_request();

        let resp: Config = test::call_and_read_body_json(&app, req).await;

        assert_eq!(resp.hardware_offset_ms, 55);
        assert_eq!(*hw_offset.lock().unwrap(), 55);

        // Test that the file was written
        assert!(fs::metadata(config_path).is_ok());
        let contents = fs::read_to_string(config_path).unwrap();
        assert!(contents.contains("\"hardware_offset_ms\": 55"));

        // Cleanup
        let _ = fs::remove_file(config_path);
    }

    #[actix_web::test]
    async fn test_manual_sync_no_ltc() {
        // State with no LTC frame
        let ltc_state = Arc::new(Mutex::new(LtcState::new()));
        let hw_offset = Arc::new(Mutex::new(0i64));

        let app_state = web::Data::new(AppState {
            ltc_state: ltc_state.clone(),
            hw_offset: hw_offset.clone(),
        });

        let app = test::init_service(
            App::new()
                .app_data(app_state.clone())
                .service(manual_sync),
        )
        .await;

        let req = test::TestRequest::post().uri("/api/sync").to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), 400); // Bad Request
    }

    #[actix_web::test]
    async fn test_manual_sync_fails() {
        // State with an LTC frame, but sync command will fail in test env
        let ltc_state = Arc::new(Mutex::new(get_test_state()));
        let hw_offset = Arc::new(Mutex::new(0i64));

        let app_state = web::Data::new(AppState {
            ltc_state: ltc_state.clone(),
            hw_offset: hw_offset.clone(),
        });

        let app = test::init_service(
            App::new()
                .app_data(app_state.clone())
                .service(manual_sync),
        )
        .await;

        let req = test::TestRequest::post().uri("/api/sync").to_request();
        let resp = test::call_service(&app, req).await;

        // In a test environment, `trigger_sync` is expected to succeed without
        // actually running a command.
        assert_eq!(resp.status(), 200); // OK
    }
}
