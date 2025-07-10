use crate::config::Config;
use crate::sync_logic::LtcState;
use rand::thread_rng;
use statime::{
    config::{
        AcceptAnyMaster, ClockIdentity, DelayMechanism, InstanceConfig, PortConfig,
        PtpMinorVersion, TimePropertiesDS, TimeSource,
    },
    filters::BasicFilter,
    port::PortAction,
    time::{Duration as PtpDuration, Interval},
    OverlayClock, PtpInstance, SharedClock,
};
use socket2::{Domain, Protocol, Socket, Type};
use statime_linux::clock::LinuxClock;
use std::net::SocketAddrV4;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::net::UdpSocket;
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
    let instance_config = InstanceConfig {
        clock_identity: ClockIdentity::from_mac_address([0x00, 0x11, 0x22, 0x33, 0x44, 0x55]),
        priority_1: 128,
        priority_2: 128,
        domain_number: 0,
        slave_only: false,
        sdo_id: Default::default(),
        path_trace: false,
    };
    let time_properties_ds =
        TimePropertiesDS::new_arbitrary_time(false, false, TimeSource::InternalOscillator);

    // 2. Create PtpInstance
    let mut ptp_instance = PtpInstance::<BasicFilter>::new(instance_config, time_properties_ds);

    // 3. Create PortConfig
    let port_config = PortConfig {
        acceptable_master_list: AcceptAnyMaster,
        delay_mechanism: DelayMechanism::E2E {
            interval: Interval::from_log_2(0),
        },
        announce_interval: Interval::from_log_2(0),
        announce_receipt_timeout: 2,
        sync_interval: Interval::from_log_2(0),
        master_only: false,
        delay_asymmetry: PtpDuration::default(),
        minor_ptp_version: PtpMinorVersion::One,
    };

    // 4. Create Clock and Filter
    let clock = SharedClock::new(OverlayClock::new(LinuxClock::open("/dev/ptp0")?));
    let filter_config = 0.1; // Filter coefficient

    // 5. Create network handles
    fn create_socket(
        interface: &str,
        port: u16,
    ) -> Result<UdpSocket, Box<dyn std::error::Error + Send + Sync>> {
        let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
        #[cfg(target_os = "linux")]
        if let Err(e) = socket.bind_device(Some(interface.as_bytes())) {
            log::warn!(
                "Failed to bind to device '{}', maybe you need to be root? Error: {}",
                interface,
                e
            );
        }
        socket.set_reuse_address(true)?;
        let address = SocketAddrV4::new("0.0.0.0".parse().unwrap(), port);
        socket.bind(&address.into())?;
        Ok(UdpSocket::from_std(socket.into())?)
    }

    let event_socket = create_socket(&interface, 319)?;
    let general_socket = create_socket(&interface, 320)?;

    // 6. Add port and run BMCA
    let mut port = ptp_instance.add_port(port_config, filter_config, clock, thread_rng())?;
    ptp_instance.bmca(&mut [&mut port]);
    let (mut running_port, initial_actions) = port.end_bmca();

    let mut last_state_update = Instant::now();

    let mut actions: Vec<_> = initial_actions.collect();
    let mut event_buf = [0u8; 1500];
    let mut general_buf = [0u8; 1500];

    loop {
        for action in actions {
            match action {
                PortAction::SendEvent {
                    data,
                    destination,
                    ..
                } => {
                    if let Err(e) = event_socket.send_to(data, destination.into()).await {
                        log::error!("Error sending PTP event packet: {}", e);
                    }
                }
                PortAction::SendGeneral {
                    data,
                    destination,
                    ..
                } => {
                    if let Err(e) = general_socket.send_to(data, destination.into()).await {
                        log::error!("Error sending PTP general packet: {}", e);
                    }
                }
                _ => {}
            }
        }

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

        actions = Vec::new();

        tokio::select! {
            _ = tokio::time::sleep_until(timer_instant) => {
                actions.extend(running_port.handle_timer());
            }
            Ok((len, source_address)) = event_socket.recv_from(&mut event_buf) => {
                actions.extend(running_port.handle_message(&event_buf[..len], source_address));
            }
            Ok((len, source_address)) = general_socket.recv_from(&mut general_buf) => {
                actions.extend(running_port.handle_message(&general_buf[..len], source_address));
            }
        }

        // Update shared state periodically
        if last_state_update.elapsed() > Duration::from_millis(500) {
            let port_ds = running_port.get_port_ds();
            let mut st = state.lock().unwrap();
            st.ptp_state = port_ds.port_state_string().to_string();
            if port_ds.is_slave() {
                st.ptp_offset = Some(port_ds.offset_from_master.mean as f64);
            } else {
                st.ptp_offset = None;
            }
            last_state_update = Instant::now();
        }
    }
}
