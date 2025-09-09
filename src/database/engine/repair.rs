use std::path::PathBuf;

use rocksdb::Options;
use tuwunel_core::{Err, Result, info, warn};

use super::Db;

pub fn repair(db_opts: &Options, path: &PathBuf) -> Result {
	warn!("Starting database repair. This may take a long time...");
	match Db::repair(db_opts, path) {
		| Ok(()) => info!("Database repair successful."),
		| Err(e) => return Err!("Repair failed: {e:?}"),
	}

	Ok(())
}
