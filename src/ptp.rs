
use crate::config::Config;
use crate::sync_logic::LtcState;
use statime::{Config as PtpConfig, PtpInstance};
use statime_linux::{LinuxClock, LinuxUdpSocket};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::time::sleep;

pub async fn start_ptp_client(state: Arc<Mutex<LtcState>>, config: Arc<Mutex<Config>>) {
    loop {
        let (enabled, interface) = {
            let cfg = config.lock().unwrap();
            (cfg.ptp_enabled, cfg.ptp_interface.clone())
        };

        if !enabled {
            {
                let mut st = state.lock().unwrap();
                if st.ptp_state != "Disabled" {
                    st.ptp_state = "Disabled".to_string();
                    st.ptp_offset = None;
                    log::info!("PTP client disabled via config.");
                }
            }
            sleep(Duration::from_secs(5)).await;
            continue;
        }

        log::info!("Starting PTP client on interface {}", interface);
        {
            let mut st = state.lock().unwrap();
            st.ptp_state = format!("Starting on {}", interface);
        }

        let result = run_ptp_session(state.clone(), config.clone()).await;

        if let Err(e) = result {
            log::error!("PTP client error: {}", e);
            let mut st = state.lock().unwrap();
            st.ptp_state = format!("Error: {}", e);
            st.ptp_offset = None;
        }

        // Wait before retrying or checking config again
        sleep(Duration::from_secs(5)).await;
    }
}

async fn run_ptp_session(
    state: Arc<Mutex<LtcState>>,
    config: Arc<Mutex<Config>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let interface = config.lock().unwrap().ptp_interface.clone();

    let mut ptp_config = PtpConfig::default();
    ptp_config.set_iface(interface.clone());
    ptp_config.set_use_hardware_timestamping(false);

    let clock = LinuxClock::new();
    let socket = LinuxUdpSocket::new(ptp_config.clone())?;
    let mut instance = PtpInstance::new(ptp_config, socket, clock);

    let initial_interface = interface;

    loop {
        let (enabled, current_interface) = {
            let cfg = config.lock().unwrap();
            (cfg.ptp_enabled, cfg.ptp_interface.clone())
        };

        if !enabled || current_interface != initial_interface {
            log::info!("PTP disabled or interface changed. Stopping PTP session.");
            return Ok(());
        }

        if let Err(e) = instance.tick().await {
            log::warn!("PTP tick error: {}", e);
        }

        let summary = instance.get_summary();
        let mut st = state.lock().unwrap();
        st.ptp_offset = summary.offset;
        st.ptp_state = summary.state.to_string();

        sleep(Duration::from_millis(200)).await;
    }
}
