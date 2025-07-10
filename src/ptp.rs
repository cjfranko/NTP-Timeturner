use crate::config::Config;
use crate::sync_logic::LtcState;
use statime::{
    config::{InstanceConfig, PortConfig, TimePropertiesDS},
    filters::MovingAverageFilter,
    port::{PortAction, PortState},
    PtpInstance,
};
use statime_linux::{LinuxClock, LinuxUdpHandles};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::time::{sleep, Instant};

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
    let initial_interface = interface.clone();

    // 1. Create configs
    let instance_config = InstanceConfig::default();
    let time_properties_ds = TimePropertiesDS::default();

    // 2. Create PtpInstance
    let mut ptp_instance = PtpInstance::new(instance_config, time_properties_ds);

    // 3. Create PortConfig
    let mut port_config = PortConfig::default();
    port_config.iface = interface.into();
    port_config.use_hardware_timestamping = false;

    // 4. Create Clock and Filter
    let clock = LinuxClock::new();
    let filter = MovingAverageFilter::new(20);

    // 5. Add port and run BMCA
    let mut port = ptp_instance.add_port(port_config.clone(), clock, filter)?;
    ptp_instance.run_bmca(&mut [&mut port]);
    let mut running_port = port.end_bmca()?;

    // 6. Create network handles
    let (mut event_handle, mut general_handle) = LinuxUdpHandles::new(&port_config)?;

    let mut last_state_update = Instant::now();

    loop {
        // Check for config changes that would require a restart
        let (enabled, current_interface) = {
            let cfg = config.lock().unwrap();
            (cfg.ptp_enabled, cfg.ptp_interface.clone())
        };
        if !enabled || current_interface != initial_interface {
            log::info!("PTP disabled or interface changed. Stopping PTP session.");
            return Ok(());
        }

        let timer_instant = running_port
            .get_next_timer_instant()
            .map(tokio::time::Instant::from_std)
            .unwrap_or_else(|| tokio::time::Instant::now() + Duration::from_secs(1));

        let mut actions = Vec::new();

        tokio::select! {
            _ = tokio::time::sleep_until(timer_instant) => {
                if let Some(action) = running_port.handle_timer() {
                    actions.push(action);
                }
            }
            Ok((message, source_address)) = event_handle.recv() => {
                if let Some(action) = running_port.handle_message(&message, source_address) {
                    actions.push(action);
                }
            }
            Ok((message, source_address)) = general_handle.recv() => {
                if let Some(action) = running_port.handle_message(&message, source_address) {
                    actions.push(action);
                }
            }
        }

        for action in actions {
            match action {
                PortAction::Send {
                    destination,
                    data,
                    event,
                } => {
                    let handle = if event {
                        &mut event_handle
                    } else {
                        &mut general_handle
                    };
                    if let Err(e) = handle.send(data, destination).await {
                        log::error!("Error sending PTP packet: {}", e);
                    }
                }
                PortAction::Reset => {
                    log::warn!("PTP port is resetting");
                    ptp_instance.run_bmca(&mut [&mut running_port.start_bmca()]);
                    running_port = running_port.start_bmca().end_bmca()?;
                }
                PortAction::UpdateMaster(_) => {
                    // Handled by the instance, nothing to do here
                }
            }
        }

        // Update shared state periodically
        if last_state_update.elapsed() > Duration::from_millis(500) {
            let port_ds = running_port.get_port_ds();
            let mut st = state.lock().unwrap();
            st.ptp_state = port_ds.port_state.to_string();
            if port_ds.port_state == PortState::Slave {
                st.ptp_offset = Some(port_ds.offset_from_master.mean as f64);
            } else {
                st.ptp_offset = None;
            }
            last_state_update = Instant::now();
        }
    }
}
