use axum::extract::State;
use axum_client_ip::InsecureClientIp;
use futures::{FutureExt, StreamExt};
use ruma::api::client::{
	account::{
		ThirdPartyIdRemovalStatus, change_password, deactivate, get_3pids,
		request_3pid_management_token_via_email, request_3pid_management_token_via_msisdn,
		whoami,
	},
	uiaa::{AuthFlow, AuthType, UiaaInfo},
};
use tuwunel_core::{Err, Error, Result, err, info, utils, utils::ReadyExt};

use super::SESSION_ID_LENGTH;
use crate::Ruma;

/// # `POST /_matrix/client/r0/account/password`
///
/// Changes the password of this account.
///
/// - Requires UIAA to verify user password
/// - Changes the password of the sender user
/// - The password hash is calculated using argon2 with 32 character salt, the
///   plain password is
/// not saved
///
/// If logout_devices is true it does the following for each device except the
/// sender device:
/// - Invalidates access token
/// - Deletes device metadata (device id, device display name, last seen ip,
///   last seen ts)
/// - Forgets to-device events
/// - Triggers device list updates
#[tracing::instrument(skip_all, fields(%client), name = "change_password")]
pub async fn change_password_route(
	State(services): State<crate::State>,
	InsecureClientIp(client): InsecureClientIp,
	body: Ruma<change_password::v3::Request>,
) -> Result<change_password::v3::Response> {
	// Authentication for this endpoint was made optional, but we need
	// authentication currently
	let sender_user = body
		.sender_user
		.as_ref()
		.ok_or_else(|| err!(Request(MissingToken("Missing access token."))))?;

	let mut uiaainfo = UiaaInfo {
		flows: vec![AuthFlow { stages: vec![AuthType::Password] }],
		..Default::default()
	};

	match &body.auth {
		| Some(auth) => {
			let (worked, uiaainfo) = services
				.uiaa
				.try_auth(sender_user, body.sender_device(), auth, &uiaainfo)
				.await?;

			if !worked {
				return Err(Error::Uiaa(uiaainfo));
			}

			// Success!
		},
		| _ => match body.json_body {
			| Some(ref json) => {
				uiaainfo.session = Some(utils::random_string(SESSION_ID_LENGTH));
				services
					.uiaa
					.create(sender_user, body.sender_device(), &uiaainfo, json);

				return Err(Error::Uiaa(uiaainfo));
			},
			| _ => {
				return Err!(Request(NotJson("JSON body is not valid")));
			},
		},
	}

	services
		.users
		.set_password(sender_user, Some(&body.new_password))
		.await?;

	if body.logout_devices {
		// Logout all devices except the current one
		services
			.users
			.all_device_ids(sender_user)
			.ready_filter(|id| *id != body.sender_device())
			.for_each(|id| services.users.remove_device(sender_user, id))
			.await;
	}

	info!("User {sender_user} changed their password.");

	if services.server.config.admin_room_notices {
		services
			.admin
			.notice(&format!("User {sender_user} changed their password."))
			.await;
	}

	Ok(change_password::v3::Response {})
}

/// # `GET _matrix/client/r0/account/whoami`
///
/// Get `user_id` of the sender user.
///
/// Note: Also works for Application Services
pub async fn whoami_route(
	State(services): State<crate::State>,
	body: Ruma<whoami::v3::Request>,
) -> Result<whoami::v3::Response> {
	Ok(whoami::v3::Response {
		user_id: body.sender_user().to_owned(),
		device_id: body.sender_device.clone(),
		is_guest: services
			.users
			.is_deactivated(body.sender_user())
			.await? && body.appservice_info.is_none(),
	})
}

/// # `POST /_matrix/client/r0/account/deactivate`
///
/// Deactivate sender user account.
///
/// - Leaves all rooms and rejects all invitations
/// - Invalidates all access tokens
/// - Deletes all device metadata (device id, device display name, last seen ip,
///   last seen ts)
/// - Forgets all to-device events
/// - Triggers device list updates
/// - Removes ability to log in again
#[tracing::instrument(skip_all, fields(%client), name = "deactivate")]
pub async fn deactivate_route(
	State(services): State<crate::State>,
	InsecureClientIp(client): InsecureClientIp,
	body: Ruma<deactivate::v3::Request>,
) -> Result<deactivate::v3::Response> {
	// Authentication for this endpoint was made optional, but we need
	// authentication currently
	let sender_user = body
		.sender_user
		.as_ref()
		.ok_or_else(|| err!(Request(MissingToken("Missing access token."))))?;

	let mut uiaainfo = UiaaInfo {
		flows: vec![AuthFlow { stages: vec![AuthType::Password] }],
		..Default::default()
	};

	match &body.auth {
		| Some(auth) => {
			let (worked, uiaainfo) = services
				.uiaa
				.try_auth(sender_user, body.sender_device(), auth, &uiaainfo)
				.await?;

			if !worked {
				return Err(Error::Uiaa(uiaainfo));
			}
			// Success!
		},
		| _ => match body.json_body {
			| Some(ref json) => {
				uiaainfo.session = Some(utils::random_string(SESSION_ID_LENGTH));
				services
					.uiaa
					.create(sender_user, body.sender_device(), &uiaainfo, json);

				return Err(Error::Uiaa(uiaainfo));
			},
			| _ => {
				return Err!(Request(NotJson("JSON body is not valid")));
			},
		},
	}

	services
		.deactivate
		.full_deactivate(sender_user)
		.boxed()
		.await?;

	info!("User {sender_user} deactivated their account.");

	if services.server.config.admin_room_notices {
		services
			.admin
			.notice(&format!("User {sender_user} deactivated their account."))
			.await;
	}

	Ok(deactivate::v3::Response {
		id_server_unbind_result: ThirdPartyIdRemovalStatus::NoSupport,
	})
}

/// # `GET _matrix/client/v3/account/3pid`
///
/// Get a list of third party identifiers associated with this account.
///
/// - Currently always returns empty list
pub async fn third_party_route(
	body: Ruma<get_3pids::v3::Request>,
) -> Result<get_3pids::v3::Response> {
	let _sender_user = body
		.sender_user
		.as_ref()
		.expect("user is authenticated");

	Ok(get_3pids::v3::Response::new(Vec::new()))
}

/// # `POST /_matrix/client/v3/account/3pid/email/requestToken`
///
/// "This API should be used to request validation tokens when adding an email
/// address to an account"
///
/// - 403 signals that The homeserver does not allow the third party identifier
///   as a contact option.
pub async fn request_3pid_management_token_via_email_route(
	_body: Ruma<request_3pid_management_token_via_email::v3::Request>,
) -> Result<request_3pid_management_token_via_email::v3::Response> {
	Err!(Request(ThreepidDenied("Third party identifiers are not implemented")))
}

/// # `POST /_matrix/client/v3/account/3pid/msisdn/requestToken`
///
/// "This API should be used to request validation tokens when adding an phone
/// number to an account"
///
/// - 403 signals that The homeserver does not allow the third party identifier
///   as a contact option.
pub async fn request_3pid_management_token_via_msisdn_route(
	_body: Ruma<request_3pid_management_token_via_msisdn::v3::Request>,
) -> Result<request_3pid_management_token_via_msisdn::v3::Response> {
	Err!(Request(ThreepidDenied("Third party identifiers are not implemented")))
}
