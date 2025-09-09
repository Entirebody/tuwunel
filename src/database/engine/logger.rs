use rocksdb::LogLevel;
use tuwunel_core::{debug, error, warn};

#[tracing::instrument(
	parent = None,
	name = "rocksdb",
	level = "trace"
	skip(msg),
)]
pub fn handle(level: LogLevel, msg: &str) {
	let msg = msg.trim();
	if msg.starts_with("Options") {
		return;
	}

	match level {
		| LogLevel::Header | LogLevel::Debug => debug!("{msg}"),
		| LogLevel::Error | LogLevel::Fatal => error!("{msg}"),
		| LogLevel::Info => debug!("{msg}"),
		| LogLevel::Warn => warn!("{msg}"),
	}
}
