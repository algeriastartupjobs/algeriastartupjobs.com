use std::sync::Arc;

use axum::{
  async_trait,
  extract::FromRequestParts,
  headers::{authorization::Bearer, Authorization},
  response::{IntoResponse, Response},
  RequestPartsExt, TypedHeader,
};

use chrono::{Duration, Utc};
use hyper::{http::request::Parts, StatusCode};
use jsonwebtoken::{decode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use rand::{distributions::Alphanumeric, prelude::Distribution, thread_rng};
use serde::{Deserialize, Serialize};

use crate::{_entry::state::AppState, _utils::error::AuthError, config::service::ConfigService};

#[derive(Debug, Serialize, Deserialize)]
pub enum TokenScope {
  CreatePost,
  Login,
}

const AUTH_TOKEN_EXPIRATION_MINUTES: i64 = 5;

#[derive(Debug, Serialize, Deserialize)]
pub struct ScopedToken {
  pub exp: usize,
  pub scope: TokenScope,
  pub id: u32,
}

// @TODO-ZM: move this into middleware?
#[async_trait]
impl FromRequestParts<AppState> for ScopedToken {
  type Rejection = AuthError;

  async fn from_request_parts(
    parts: &mut Parts,
    app_state: &AppState,
  ) -> Result<Self, Self::Rejection> {
    let TypedHeader(Authorization(bearer)) = parts
      .extract::<TypedHeader<Authorization<Bearer>>>()
      .await
      .map_err(|_| AuthError::InvalidToken)?;

    let scoped_token = app_state
      .auth_service
      .decode_scoped_token(bearer.token().to_string())
      .await?;

    Ok(scoped_token)
  }
}

impl IntoResponse for AuthError {
  fn into_response(self) -> Response {
    let status = StatusCode::UNAUTHORIZED;
    status.into_response()
  }
}

pub struct ConfirmationObject {
  pub id: String,
  pub code: String,
}

pub struct AuthService {
  config_service: Arc<ConfigService>,
  main_kv_db: Arc<sled::Db>,
}

impl AuthService {
  pub fn new(config_service: Arc<ConfigService>, main_kv_db: Arc<sled::Db>) -> Self {
    Self {
      config_service,
      main_kv_db,
    }
  }

  pub async fn generate_confirmation_object(
    &self,
    key: String,
  ) -> Result<ConfirmationObject, AuthError> {
    let random_16: String = Alphanumeric
      .sample_iter(&mut thread_rng())
      .take(16)
      .map(char::from)
      .collect();
    let random_16 = random_16.to_uppercase();
    let id = &random_16[..12];
    let code = &random_16[12..];

    let kv_db_result = self.main_kv_db.insert(key, random_16.as_bytes());
    if kv_db_result.is_err() {
      // @TODO-ZM: log error reason
      return Err(AuthError::InternalError);
    }

    Ok(ConfirmationObject {
      id: id.to_string(),
      code: code.to_string(),
    })
  }

  pub async fn verify_confirmation_object(
    &self,
    confirmation_object: ConfirmationObject,
  ) -> Result<(), AuthError> {
    let kv_db_result = self.main_kv_db.compare_and_swap(
      confirmation_object.id,
      Some(confirmation_object.code),
      None as Option<&[u8]>,
    );

    if kv_db_result.is_err() || kv_db_result.unwrap().is_err() {
      // @TODO-ZM: log error reason
      return Err(AuthError::InternalError);
    }

    Ok(())
  }

  pub async fn generate_scoped_token(
    &self,
    scope: TokenScope,
    id: u32,
  ) -> Result<String, AuthError> {
    let header = Header::new(jsonwebtoken::Algorithm::HS512);
    let secret = self.config_service.get_config().jwt_secret;
    let key = EncodingKey::from_secret(secret.as_ref());

    let scoped_token = ScopedToken {
      exp: (Utc::now() + Duration::minutes(AUTH_TOKEN_EXPIRATION_MINUTES)).timestamp() as usize,
      scope,
      id,
    };

    let token = jsonwebtoken::encode(&header, &scoped_token, &key);

    if token.is_err() {
      // @TODO-ZM: log error reason
      return Err(AuthError::InternalError);
    }
    let token = token.unwrap();

    Ok(token)
  }

  pub async fn decode_scoped_token(&self, token: String) -> Result<ScopedToken, AuthError> {
    let decode_key = DecodingKey::from_secret(self.config_service.get_config().jwt_secret.as_ref());

    let token_data = decode::<ScopedToken>(&token, &decode_key, &Validation::new(Algorithm::HS512))
      .map_err(|_| AuthError::InvalidToken)?;

    if (token_data.claims.exp as i64) < Utc::now().timestamp() {
      // @TODO-ZM: log error reason
      return Err(AuthError::InvalidToken);
    }

    Ok(token_data.claims)
  }
}
