use std::{borrow::Borrow, iter::once};

use axum::extract::State;
use futures::{FutureExt, StreamExt, TryStreamExt};
use ruma::{OwnedEventId, api::federation::event::get_room_state};
use tuwunel_core::{Result, at, err, utils::IterStream};

use super::AccessCheck;
use crate::Ruma;

/// # `GET /_matrix/federation/v1/state/{roomId}`
///
/// Retrieves a snapshot of a room's state at a given event.
pub async fn get_room_state_route(
	State(services): State<crate::State>,
	body: Ruma<get_room_state::v1::Request>,
) -> Result<get_room_state::v1::Response> {
	AccessCheck {
		services: &services,
		origin: body.origin(),
		room_id: &body.room_id,
		event_id: None,
	}
	.check()
	.await?;

	let shortstatehash = services
		.state_accessor
		.pdu_shortstatehash(&body.event_id)
		.await
		.map_err(|_| err!(Request(NotFound("PDU state not found."))))?;

	let state_ids: Vec<OwnedEventId> = services
		.state_accessor
		.state_full_ids(shortstatehash)
		.map(at!(1))
		.collect()
		.await;

	let pdus = state_ids
		.iter()
		.try_stream()
		.and_then(|id| services.timeline.get_pdu_json(id))
		.and_then(|pdu| {
			services
				.federation
				.format_pdu_into(pdu, None)
				.map(Ok)
		})
		.try_collect()
		.await?;

	let auth_chain = services
		.auth_chain
		.event_ids_iter(&body.room_id, once(body.event_id.borrow()))
		.and_then(async |id| services.timeline.get_pdu_json(&id).await)
		.and_then(|pdu| {
			services
				.federation
				.format_pdu_into(pdu, None)
				.map(Ok)
		})
		.try_collect()
		.await?;

	Ok(get_room_state::v1::Response { auth_chain, pdus })
}
