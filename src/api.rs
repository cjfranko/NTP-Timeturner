
use actix_files as fs;
use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder};
use chrono::{Local, Timelike};
use get_if_addrs::get_if_addrs;
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use crate::config::{self, Config};
use crate::sync_logic::{self, LtcState};
use crate::system;

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
    pub config: Arc<Mutex<Config>>,
    pub log_buffer: Arc<Mutex<VecDeque<String>>>,
}

#[get("/api/status")]
async fn get_status(data: web::Data<AppState>) -> impl Responder {
    let state = data.ltc_state.lock().unwrap();
    let config = data.config.lock().unwrap();
    let hw_offset_ms = config.hardware_offset_ms;

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

    let avg_delta = state.get_ewma_clock_delta();
    let mut delta_frames = 0;
    if let Some(frame) = &state.latest {
        let frame_ms = 1000.0 / frame.frame_rate;
        delta_frames = ((avg_delta as f64 / frame_ms).round()) as i64;
    }

    let sync_status = sync_logic::get_sync_status(avg_delta, &config);
    let jitter_status = sync_logic::get_jitter_status(state.average_jitter());
    let lock_ratio = state.lock_ratio();

    let ntp_active = system::ntp_service_active();
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
        sync_status: sync_status.to_string(),
        jitter_status: jitter_status.to_string(),
        lock_ratio,
        ntp_active,
        interfaces,
        hardware_offset_ms: hw_offset_ms,
    })
}

#[post("/api/sync")]
async fn manual_sync(data: web::Data<AppState>) -> impl Responder {
    let state = data.ltc_state.lock().unwrap();
    let config = data.config.lock().unwrap();
    if let Some(frame) = &state.latest {
        if system::trigger_sync(frame, &config).is_ok() {
            HttpResponse::Ok().json(serde_json::json!({ "status": "success", "message": "Sync command issued." }))
        } else {
            HttpResponse::InternalServerError().json(serde_json::json!({ "status": "error", "message": "Sync command failed." }))
        }
    } else {
        HttpResponse::BadRequest().json(serde_json::json!({ "status": "error", "message": "No LTC timecode available to sync to." }))
    }
}

#[get("/api/config")]
async fn get_config(data: web::Data<AppState>) -> impl Responder {
    let config = data.config.lock().unwrap();
    HttpResponse::Ok().json(&*config)
}

#[get("/api/logs")]
async fn get_logs(data: web::Data<AppState>) -> impl Responder {
    let logs = data.log_buffer.lock().unwrap();
    HttpResponse::Ok().json(&*logs)
}

#[derive(Deserialize)]
struct NudgeRequest {
    microseconds: i64,
}

#[post("/api/nudge_clock")]
async fn nudge_clock(req: web::Json<NudgeRequest>) -> impl Responder {
    if system::nudge_clock(req.microseconds).is_ok() {
        HttpResponse::Ok().json(serde_json::json!({ "status": "success", "message": "Clock nudge command issued." }))
    } else {
        HttpResponse::InternalServerError().json(serde_json::json!({ "status": "error", "message": "Clock nudge command failed." }))
    }
}

#[post("/api/config")]
async fn update_config(
    data: web::Data<AppState>,
    req: web::Json<Config>,
) -> impl Responder {
    let mut config = data.config.lock().unwrap();
    *config = req.into_inner();

    if config::save_config("config.yml", &config).is_ok() {
        log::info!("🔄 Saved config via API: {:?}", *config);

        // If timeturner offset is active, trigger a sync immediately.
        if config.timeturner_offset.is_active() {
            let state = data.ltc_state.lock().unwrap();
            if let Some(frame) = &state.latest {
                log::info!("Timeturner offset is active, triggering sync...");
                if system::trigger_sync(frame, &config).is_ok() {
                    log::info!("Sync triggered successfully after config change.");
                } else {
                    log::error!("Sync failed after config change.");
                }
            } else {
                log::warn!("Timeturner offset is active, but no LTC frame available to sync.");
            }
        }

        HttpResponse::Ok().json(&*config)
    } else {
        log::error!("Failed to write config.yml");
        HttpResponse::InternalServerError().json(
            serde_json::json!({ "status": "error", "message": "Failed to write config.yml" }),
        )
    }
}

pub async fn start_api_server(
    state: Arc<Mutex<LtcState>>,
    config: Arc<Mutex<Config>>,
    log_buffer: Arc<Mutex<VecDeque<String>>>,
) -> std::io::Result<()> {
    let app_state = web::Data::new(AppState {
        ltc_state: state,
        config: config,
        log_buffer: log_buffer,
    });

    log::info!("🚀 Starting API server at http://0.0.0.0:8080");

    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .service(get_status)
            .service(manual_sync)
            .service(get_config)
            .service(update_config)
            .service(get_logs)
            .service(nudge_clock)
            // Serve frontend static files
            .service(fs::Files::new("/", "static/").index_file("index.html"))
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::TimeturnerOffset;
    use crate::sync_logic::LtcFrame;
    use actix_web::{test, App};
    use chrono::Utc;
    use std::collections::VecDeque;
    use std::fs;

    // Helper to create a default LtcState for tests
    fn get_test_ltc_state() -> LtcState {
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
            ewma_clock_delta: Some(5.0),
            last_match_status: "IN SYNC".to_string(),
            last_match_check: Utc::now().timestamp(),
        }
    }

    // Helper to create a default AppState for tests
    fn get_test_app_state() -> web::Data<AppState> {
        let ltc_state = Arc::new(Mutex::new(get_test_ltc_state()));
        let config = Arc::new(Mutex::new(Config {
            hardware_offset_ms: 10,
            timeturner_offset: TimeturnerOffset::default(),
            default_nudge_ms: 2,
            auto_sync_enabled: false,
        }));
        let log_buffer = Arc::new(Mutex::new(VecDeque::new()));
        web::Data::new(AppState {
            ltc_state,
            config,
            log_buffer,
        })
    }

    #[actix_web::test]
    async fn test_get_status() {
        let app_state = get_test_app_state();
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
        let app_state = get_test_app_state();
        app_state.config.lock().unwrap().hardware_offset_ms = 25;

        let app = test::init_service(
            App::new()
                .app_data(app_state.clone())
                .service(get_config),
        )
        .await;

        let req = test::TestRequest::get().uri("/api/config").to_request();
        let resp: Config = test::call_and_read_body_json(&app, req).await;

        assert_eq!(resp.hardware_offset_ms, 25);
    }

    #[actix_web::test]
    async fn test_update_config() {
        let app_state = get_test_app_state();
        let config_path = "config.yml";

        // This test has the side effect of writing to `config.yml`.
        // We ensure it's cleaned up after.
        let _ = fs::remove_file(config_path);

        let app = test::init_service(
            App::new()
                .app_data(app_state.clone())
                .service(update_config),
        )
        .await;

        let new_config_json = serde_json::json!({
            "hardwareOffsetMs": 55,
            "defaultNudgeMs": 2,
            "autoSyncEnabled": true,
            "timeturnerOffset": { "hours": 1, "minutes": 2, "seconds": 3, "frames": 4, "milliseconds": 5 }
        });

        let req = test::TestRequest::post()
            .uri("/api/config")
            .set_json(&new_config_json)
            .to_request();

        let resp: Config = test::call_and_read_body_json(&app, req).await;

        assert_eq!(resp.hardware_offset_ms, 55);
        assert_eq!(resp.auto_sync_enabled, true);
        assert_eq!(resp.timeturner_offset.hours, 1);
        assert_eq!(resp.timeturner_offset.milliseconds, 5);
        let final_config = app_state.config.lock().unwrap();
        assert_eq!(final_config.hardware_offset_ms, 55);
        assert_eq!(final_config.auto_sync_enabled, true);
        assert_eq!(final_config.timeturner_offset.hours, 1);
        assert_eq!(final_config.timeturner_offset.milliseconds, 5);

        // Test that the file was written
        assert!(fs::metadata(config_path).is_ok());
        let contents = fs::read_to_string(config_path).unwrap();
        assert!(contents.contains("hardwareOffsetMs: 55"));
        assert!(contents.contains("autoSyncEnabled: true"));
        assert!(contents.contains("hours: 1"));
        assert!(contents.contains("milliseconds: 5"));

        // Cleanup
        let _ = fs::remove_file(config_path);
    }

    #[actix_web::test]
    async fn test_manual_sync_no_ltc() {
        let app_state = get_test_app_state();
        // State with no LTC frame
        app_state.ltc_state.lock().unwrap().latest = None;

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
}
