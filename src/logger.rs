use log::{Log, Metadata, Record};

pub struct Logger;

impl Log for Logger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            println!(
                "{:<5} [{}] {}",
                record.level().to_string(),
                record.module_path().unwrap_or_default(),
                record.args());
        }
    }

    fn flush(&self) {
    }
}