use futures::{FutureExt, StreamExt, future::OptionFuture, join};
use ruma::{EventId, RoomId, ServerName};
use tuwunel_core::{Err, Result, implement, is_false};
use tuwunel_service::Services;

pub struct AccessCheck<'a> {
	pub services: &'a Services,
	pub origin: &'a ServerName,
	pub room_id: &'a RoomId,
	pub event_id: Option<&'a EventId>,
}

#[implement(AccessCheck, params = "<'_>")]
pub async fn check(&self) -> Result {
	let acl_check = self
		.services
		.event_handler
		.acl_check(self.origin, self.room_id)
		.map(|result| result.is_ok());

	let world_readable = self
		.services
		.state_accessor
		.is_world_readable(self.room_id);

	let server_in_room = self
		.services
		.state_cache
		.server_in_room(self.origin, self.room_id);

	// if any user on our homeserver is trying to knock this room, we'll need to
	// acknowledge bans or leaves
	let user_is_knocking = self
		.services
		.state_cache
		.room_members_knocked(self.room_id)
		.count();

	let server_can_see: OptionFuture<_> = self
		.event_id
		.map(|event_id| {
			self.services
				.state_accessor
				.server_can_see_event(self.origin, self.room_id, event_id)
		})
		.into();

	let (world_readable, server_in_room, server_can_see, acl_check, user_is_knocking) =
		join!(world_readable, server_in_room, server_can_see, acl_check, user_is_knocking);

	if !acl_check {
		return Err!(Request(Forbidden("Server access denied.")));
	}

	if !world_readable && !server_in_room && user_is_knocking == 0 {
		return Err!(Request(Forbidden("Server is not in room.")));
	}

	if server_can_see.is_some_and(is_false!()) {
		return Err!(Request(Forbidden("Server is not allowed to see event.")));
	}

	Ok(())
}
