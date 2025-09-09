use std::sync::Arc;

use rocksdb::{ReadOptions, ReadTier, WriteOptions};

use crate::Engine;

#[inline]
pub fn cache_iter_options_default(db: &Arc<Engine>) -> ReadOptions {
	let mut options = iter_options_default(db);
	options.set_read_tier(ReadTier::BlockCache);
	options.fill_cache(false);
	options
}

#[inline]
pub fn iter_options_default(db: &Arc<Engine>) -> ReadOptions {
	let mut options = read_options_default(db);
	options.set_background_purge_on_iterator_cleanup(true);
	options
}

#[inline]
pub fn cache_read_options_default(db: &Arc<Engine>) -> ReadOptions {
	let mut options = read_options_default(db);
	options.set_read_tier(ReadTier::BlockCache);
	options.fill_cache(false);
	options
}

#[inline]
pub fn read_options_default(db: &Arc<Engine>) -> ReadOptions {
	let mut options = ReadOptions::default();
	options.set_total_order_seek(true);

	if !db.checksums {
		options.set_verify_checksums(false);
	}

	options
}

#[inline]
pub fn write_options_default(_db: &Arc<Engine>) -> WriteOptions { WriteOptions::default() }
