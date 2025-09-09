#![allow(clippy::wildcard_imports)]
#![allow(clippy::enum_glob_use)]
#![allow(clippy::too_many_arguments)]

pub mod admin;
pub mod context;
pub mod processor;
mod tests;
pub mod utils;

pub mod appservice;
pub mod check;
pub mod debug;
pub mod federation;
pub mod media;
pub mod query;
pub mod room;
pub mod server;
pub mod user;

pub use tuwunel_macros::{admin_command, admin_command_dispatch};

pub use crate::{context::Context, utils::get_room_info};

pub const PAGE_SIZE: usize = 100;

tuwunel_core::mod_ctor! {}
tuwunel_core::mod_dtor! {}
tuwunel_core::rustc_flags_capture! {}

/// Install the admin command processor
pub async fn init(admin_service: &tuwunel_service::admin::Service) {
	_ = admin_service
		.complete
		.write()
		.expect("locked for writing")
		.insert(processor::complete);
	_ = admin_service
		.handle
		.write()
		.await
		.insert(processor::dispatch);
}

/// Uninstall the admin command handler
pub async fn fini(admin_service: &tuwunel_service::admin::Service) {
	_ = admin_service.handle.write().await.take();
	_ = admin_service
		.complete
		.write()
		.expect("locked for writing")
		.take();
}
