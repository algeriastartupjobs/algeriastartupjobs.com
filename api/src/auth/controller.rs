use std::{net::SocketAddr, ops::Add};

use axum::{
  extract::{ConnectInfo, State},
  response::IntoResponse,
  Json, Router,
};
use hyper::StatusCode;
use serde::Deserialize;
use serde_json::json;

use crate::{
  _entry::state::AppState,
  _utils::error::{DataAccessError, SecurityError},
  security::service::RateLimitConstraint,
};

use super::service::{ConfirmationObject, ScopedToken, TokenScope};

#[derive(Deserialize)]
pub struct LoginBody {
  pub email: String,
}

pub async fn login(
  ConnectInfo(ip): ConnectInfo<SocketAddr>,
  State(app_state): State<AppState>,
  Json(body): Json<LoginBody>,
) -> impl IntoResponse {
  match app_state.security_service.rate_limit(vec![
    RateLimitConstraint {
      id: format!("login-1-{}", body.email),
      max_requests: 1,
      duration_ms: 10_000,
    },
    RateLimitConstraint {
      id: format!("login-2-{}", body.email),
      max_requests: 3,
      duration_ms: 5 * 60_000,
    },
    RateLimitConstraint {
      id: format!("login-3-{}", body.email),
      max_requests: 10,
      duration_ms: 60 * 60_000,
    },
    RateLimitConstraint {
      id: format!("login-4-{}", body.email),
      max_requests: 20,
      duration_ms: 24 * 60 * 60_000,
    },
    RateLimitConstraint {
      id: format!("login-ip-{}", ip.ip()),
      max_requests: 60,
      duration_ms: 60_000,
    },
  ]) {
    Ok(_) => {}
    Err(SecurityError::InternalError) => {
      // @TODO-ZM: log error reason
      return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
    Err(SecurityError::RateLimitError) => {
      return StatusCode::TOO_MANY_REQUESTS.into_response();
    }
  }

  let account = app_state
    .account_repository
    .get_one_account_by_email(&body.email)
    .await;

  if account.is_err() {
    match account.unwrap_err() {
      DataAccessError::NotFound => {
        // @TODO-ZM: log error reason
        return StatusCode::NOT_FOUND.into_response();
      }
      _ => {
        // @TODO-ZM: log error reason
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
      }
    }
  }
  let account = account.unwrap();

  let confirmation_object = app_state
    .auth_service
    .generate_confirmation_object(format!("account:{}", account.id))
    .await;
  if confirmation_object.is_err() {
    // @TODO-ZM: log error reason
    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
  }
  let confirmation_object = confirmation_object.unwrap();

  let email_result = app_state
  .email_service
  .send_one_email(
    &account.email,
    &"Confirm your login".to_string(),
    &format!(
      r#"Your email is used to login to algeriastartupjobs.com at {} (Algiers timezone).

Please confirm your login by copying the code below into the confirmation page:

<div style="width: 100%; text-align: center;">
<span style="font-size: x-large; letter-spacing: .2em; border: 1px solid #9999; border-radius: .2em; padding: .4em; display: inline-block;">{}</span>
</div>

Thank you for using our service!

ASJ Team
contact@algeriastartupjobs.com
https://www.algeriastartupjobs.com
"#,
      // format to friendly date: 12:00 AM, 1 January 2021 (UTC+01:00)
      chrono::Utc::now().add(chrono::Duration::hours(1)).format("%-I:%M %p, %-d %B %Y"),
      confirmation_object.code,
    ),
  )
  .await;

  if email_result.is_err() {
    // @TODO-ZM: log error reason
    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
  }

  Json(json!({
      "confirmation_id": confirmation_object.id,
  }))
  .into_response()
}

#[derive(Deserialize)]
pub struct ConfirmLoginBody {
  email: String,
  confirmation_id: String,
  confirmation_code: String,
}

pub async fn confirm_login(
  ConnectInfo(ip): ConnectInfo<SocketAddr>,
  State(app_state): State<AppState>,
  Json(body): Json<ConfirmLoginBody>,
) -> impl IntoResponse {
  match app_state
    .security_service
    .rate_limit(vec![RateLimitConstraint {
      id: format!("confirm_login-ip-{}", ip.ip()),
      max_requests: 60,
      duration_ms: 60_000,
    }]) {
    Ok(_) => {}
    Err(SecurityError::InternalError) => {
      // @TODO-ZM: log error reason
      return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
    Err(SecurityError::RateLimitError) => {
      return StatusCode::TOO_MANY_REQUESTS.into_response();
    }
  }

  let account = app_state
    .account_repository
    .get_one_account_by_email(&body.email)
    .await;

  if account.is_err() {
    match account.unwrap_err() {
      DataAccessError::NotFound => {
        // @TODO-ZM: log error reason
        return StatusCode::NOT_FOUND.into_response();
      }
      _ => {
        // @TODO-ZM: log error reason
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
      }
    }
  }
  let account = account.unwrap();

  let verification_result = app_state
    .auth_service
    .verify_confirmation_object(ConfirmationObject {
      id: format!("account:{}", account.id),
      code: format!("{}{}", body.confirmation_id, body.confirmation_code),
    })
    .await;

  if verification_result.is_err() {
    // @TODO-ZM: log error reason
    return StatusCode::UNAUTHORIZED.into_response();
  }

  let token = app_state
    .auth_service
    .generate_scoped_token(TokenScope::Login, account.id)
    .await;

  if token.is_err() {
    // @TODO-ZM: log error reason
    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
  }
  let token = token.unwrap();

  Json(json!({
      "auth_token": token,
      "account": account,
  }))
  .into_response()
}

pub async fn refresh_token(
  ConnectInfo(ip): ConnectInfo<SocketAddr>,
  State(app_state): State<AppState>,
  scoped_token: ScopedToken,
) -> impl IntoResponse {
  match app_state
    .security_service
    .rate_limit(vec![RateLimitConstraint {
      id: format!("confirm_login-ip-{}", ip.ip()),
      max_requests: 60,
      duration_ms: 60_000,
    }]) {
    Ok(_) => {}
    Err(SecurityError::InternalError) => {
      // @TODO-ZM: log error reason
      return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
    Err(SecurityError::RateLimitError) => {
      return StatusCode::TOO_MANY_REQUESTS.into_response();
    }
  }
  let token = app_state
    .auth_service
    .generate_scoped_token(scoped_token.scope, scoped_token.id)
    .await;
  if token.is_err() {
    // @TODO-ZM: log error reason
    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
  }
  let token = token.unwrap();

  Json(json!({
      "auth_token": token,
  }))
  .into_response()
}

pub fn create_auth_router() -> Router<AppState> {
  Router::new()
    .route("/login", axum::routing::post(login))
    .route("/confirm_login", axum::routing::post(confirm_login))
    .route("/refresh_token", axum::routing::post(refresh_token))
}
