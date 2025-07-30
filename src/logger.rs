use chrono::Local;
use log::{LevelFilter, Log, Metadata, Record};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

const MAX_LOG_ENTRIES: usize = 100;

struct RingBufferLogger {
    buffer: Arc<Mutex<VecDeque<String>>>,
}

impl Log for RingBufferLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= LevelFilter::Info
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let msg = format!(
                "{} [{}] {}",
                Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.level(),
                record.args()
            );

            // Also print to stderr for console/daemon logging
            eprintln!("{}", msg);

            let mut buffer = self.buffer.lock().unwrap();
            if buffer.len() == MAX_LOG_ENTRIES {
                buffer.pop_front();
            }
            buffer.push_back(msg);
        }
    }

    fn flush(&self) {}
}

pub fn setup_logger() -> Arc<Mutex<VecDeque<String>>> {
    let buffer = Arc::new(Mutex::new(VecDeque::with_capacity(MAX_LOG_ENTRIES)));
    let logger = RingBufferLogger {
        buffer: buffer.clone(),
    };

    // We use `set_boxed_logger` to install our custom logger.
    // The `log` crate will then route all log messages to it.
    log::set_boxed_logger(Box::new(logger)).expect("Failed to set logger");
    log::set_max_level(LevelFilter::Info);

    buffer
}
