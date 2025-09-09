use std::{fmt, time::SystemTime};

use futures::{
	Future, FutureExt, TryFutureExt,
	io::{AsyncWriteExt, BufWriter},
	lock::Mutex,
};
use ruma::EventId;
use tuwunel_core::Result;
use tuwunel_service::Services;

pub struct Context<'a> {
	pub services: &'a Services,
	pub body: &'a [&'a str],
	pub timer: SystemTime,
	pub reply_id: Option<&'a EventId>,
	pub output: Mutex<BufWriter<Vec<u8>>>,
}

impl Context<'_> {
	pub fn write_fmt(
		&self,
		arguments: fmt::Arguments<'_>,
	) -> impl Future<Output = Result> + Send + '_ + use<'_> {
		let buf = format!("{arguments}");
		self.output.lock().then(async move |mut output| {
			output
				.write_all(buf.as_bytes())
				.map_err(Into::into)
				.await
		})
	}

	pub fn write_str<'a>(
		&'a self,
		s: &'a str,
	) -> impl Future<Output = Result> + Send + 'a {
		self.output.lock().then(async move |mut output| {
			output
				.write_all(s.as_bytes())
				.map_err(Into::into)
				.await
		})
	}
}
