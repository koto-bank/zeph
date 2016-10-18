extern crate log;
extern crate ansi_term;

use self::log::{LogRecord, LogLevel, LogMetadata, LogLevelFilter, SetLoggerError};
use self::ansi_term::Color::{Green,Red};

pub struct ZephLogger;

impl ::log::Log for ZephLogger {
    fn enabled(&self, _: &LogMetadata) -> bool { true }

    fn log(&self, record: &LogRecord) {
        if self.enabled(record.metadata()) {
            match record.level() {
                LogLevel::Info  => println!(r"[{}] {}", Green.paint("INFO"), record.args()),
                LogLevel::Error => println!(r"[{}] {}", Red.paint("ERROR"), record.args()),
                _               => println!(r"[{}] {}", record.level(), record.args())
            }
        }
    }
}

impl ZephLogger {
    pub fn init() -> Result<(), SetLoggerError> {
        log::set_logger(|max_log_level| {
            max_log_level.set(LogLevelFilter::Info);
            Box::new(ZephLogger)
        })
    }
}
