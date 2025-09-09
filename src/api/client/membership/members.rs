use axum::extract::State;
use futures::{FutureExt, StreamExt};
use ruma::{
	api::client::membership::{
		get_member_events::{self, v3::MembershipEventFilter},
		joined_members::{self, v3::RoomMember},
	},
	events::{
		StateEventType,
		room::member::{MembershipState, RoomMemberEventContent},
	},
};
use tuwunel_core::{Err, Result, at, matrix::Event, utils::stream::ReadyExt};

use crate::Ruma;

/// # `POST /_matrix/client/r0/rooms/{roomId}/members`
///
/// Lists all joined users in a room (TODO: at a specific point in time, with a
/// specific membership).
///
/// - Only works if the user is currently joined
pub async fn get_member_events_route(
	State(services): State<crate::State>,
	body: Ruma<get_member_events::v3::Request>,
) -> Result<get_member_events::v3::Response> {
	if !services
		.state_accessor
		.user_can_see_state_events(body.sender_user(), &body.room_id)
		.await
	{
		return Err!(Request(Forbidden("You don't have permission to view this room.")));
	}

	let membership = body.membership.as_ref();
	let not_membership = body.not_membership.as_ref();
	Ok(get_member_events::v3::Response {
		chunk: services
			.state_accessor
			.room_state_full(&body.room_id)
			.ready_filter_map(Result::ok)
			.ready_filter(|((ty, _), _)| *ty == StateEventType::RoomMember)
			.map(at!(1))
			.ready_filter_map(|pdu| membership_filter(pdu, membership, not_membership))
			.map(Event::into_format)
			.collect()
			.boxed()
			.await,
	})
}

/// # `POST /_matrix/client/r0/rooms/{roomId}/joined_members`
///
/// Lists all members of a room.
///
/// - The sender user must be in the room
/// - TODO: An appservice just needs a puppet joined
pub async fn joined_members_route(
	State(services): State<crate::State>,
	body: Ruma<joined_members::v3::Request>,
) -> Result<joined_members::v3::Response> {
	if !services
		.state_accessor
		.user_can_see_state_events(body.sender_user(), &body.room_id)
		.await
	{
		return Err!(Request(Forbidden("You don't have permission to view this room.")));
	}

	Ok(joined_members::v3::Response {
		joined: services
			.state_accessor
			.room_state_full(&body.room_id)
			.ready_filter_map(Result::ok)
			.ready_filter(|((ty, _), _)| *ty == StateEventType::RoomMember)
			.map(at!(1))
			.ready_filter_map(|pdu| {
				membership_filter(pdu, Some(&MembershipEventFilter::Join), None)
			})
			.ready_filter_map(|pdu| {
				let content = pdu.get_content::<RoomMemberEventContent>().ok()?;
				let sender = pdu.sender().to_owned();
				let member = RoomMember {
					display_name: content.displayname,
					avatar_url: content.avatar_url,
				};

				Some((sender, member))
			})
			.collect()
			.boxed()
			.await,
	})
}

fn membership_filter<Pdu: Event>(
	pdu: Pdu,
	for_membership: Option<&MembershipEventFilter>,
	not_membership: Option<&MembershipEventFilter>,
) -> Option<impl Event> {
	let membership_state_filter = match for_membership {
		| Some(MembershipEventFilter::Ban) => MembershipState::Ban,
		| Some(MembershipEventFilter::Invite) => MembershipState::Invite,
		| Some(MembershipEventFilter::Knock) => MembershipState::Knock,
		| Some(MembershipEventFilter::Leave) => MembershipState::Leave,
		| Some(_) | None => MembershipState::Join,
	};

	let not_membership_state_filter = match not_membership {
		| Some(MembershipEventFilter::Ban) => MembershipState::Ban,
		| Some(MembershipEventFilter::Invite) => MembershipState::Invite,
		| Some(MembershipEventFilter::Join) => MembershipState::Join,
		| Some(MembershipEventFilter::Knock) => MembershipState::Knock,
		| Some(_) | None => MembershipState::Leave,
	};

	let evt_membership = pdu
		.get_content::<RoomMemberEventContent>()
		.ok()?
		.membership;

	if for_membership.is_some() && not_membership.is_some() {
		if membership_state_filter != evt_membership
			|| not_membership_state_filter == evt_membership
		{
			None
		} else {
			Some(pdu)
		}
	} else if for_membership.is_some() && not_membership.is_none() {
		if membership_state_filter != evt_membership {
			None
		} else {
			Some(pdu)
		}
	} else if not_membership.is_some() && for_membership.is_none() {
		if not_membership_state_filter == evt_membership {
			None
		} else {
			Some(pdu)
		}
	} else {
		Some(pdu)
	}
}
