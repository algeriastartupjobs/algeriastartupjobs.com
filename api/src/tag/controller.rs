use crate::_entry::state::AppState;
use crate::_utils::error::SecurityError;
use crate::_utils::string::slugify;
use crate::ai::service::PostToSuggestTagsFor;
use crate::security::service::RateLimitConstraint;
use crate::tag::model::{CompactTag, DBTag};

use axum::extract::ConnectInfo;
use axum::{extract::State, response::IntoResponse, Json, Router};
use hyper::StatusCode;
use serde_json::json;
use std::net::SocketAddr;

pub async fn get_many_suggested_tags_for_post(
  ConnectInfo(ip): ConnectInfo<SocketAddr>,
  State(app_state): State<AppState>,
  Json(body): Json<PostToSuggestTagsFor>,
) -> impl IntoResponse {
  // @TODO-ZM: write a macro for this
  match app_state.security_service.rate_limit(vec![
    RateLimitConstraint {
      id: format!("get_many_suggested_tags_for_post-1{}", ip.ip()),
      max_requests: 1,
      duration_ms: 2000,
    },
    RateLimitConstraint {
      id: format!("get_many_suggested_tags_for_post-2-{}", ip.ip()),
      max_requests: 30,
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

  let keywords = app_state.ai_service.suggest_tags_for_post(body).await;
  if keywords.is_err() {
    // @TODO-ZM: log error reason
    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
  }
  let keywords = keywords.unwrap();

  let existing_tags = app_state
    .tag_repository
    .get_many_compact_tags_by_names(keywords.clone())
    .await;

  if existing_tags.is_err() {
    // @TODO-ZM: log error reason
    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
  }
  let existing_tags = existing_tags.unwrap();

  let new_keywords = keywords
    .iter()
    .filter(|keyword| {
      !existing_tags
        .iter().any(|tag| tag.name == **keyword)
    })
    .collect::<Vec<&String>>();

  let new_db_tags = new_keywords
    .iter()
    .map(|keyword| DBTag {
      name: keyword.to_string(),
      slug: slugify(keyword),
    })
    .collect::<Vec<DBTag>>();

  let mut new_tag_ids = vec![];
  for new_db_tag in &new_db_tags {
    let tag_id = app_state
      .tag_repository
      .create_one_tag(new_db_tag.clone())
      .await;
    if tag_id.is_err() {
      // @TODO-ZM: log error reason
      return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
    let tag_id = tag_id.unwrap();
    new_tag_ids.push(tag_id);
  }

  let new_compact_tags = new_db_tags
    .iter()
    .zip(new_tag_ids.iter())
    .map(|(db_tag, tag_id)| CompactTag {
      id: *tag_id,
      name: db_tag.name.clone(),
      slug: db_tag.slug.clone(),
    })
    .collect::<Vec<_>>();

  let all_compact_tags = existing_tags
    .iter()
    .chain(new_compact_tags.iter()).cloned()
    .collect::<Vec<_>>();

  Json(json!({
      "tags": all_compact_tags,
  }))
  .into_response()
}

pub fn create_tag_router() -> Router<AppState> {
  Router::new().route(
    "/suggestions_for_post",
    axum::routing::post(get_many_suggested_tags_for_post),
  )
}
