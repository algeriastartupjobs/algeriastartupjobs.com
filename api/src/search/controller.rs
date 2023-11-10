use axum::{
  extract::{Query, State},
  response::IntoResponse,
  Json, Router,
};
use hyper::StatusCode;
use serde_json::json;

use crate::{
  _entry::state::AppState,
  _utils::{query::SearchQuery, vec::sort_and_dedup_vec},
};

pub async fn search_posts(
  State(app_state): State<AppState>,
  url_query: Query<SearchQuery>,
) -> impl IntoResponse {
  let post_ids = app_state
    .search_service
    .search_posts(&url_query.query)
    .await;
  if post_ids.is_err() {
    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
  }
  let post_ids = post_ids.unwrap();

  let compact_posts = app_state
    .post_repository
    .get_many_posts_by_ids(post_ids.clone())
    .await;
  if compact_posts.is_err() {
    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
  }
  let mut compact_posts = compact_posts.unwrap();

  compact_posts.sort_by_key(|post| post_ids.iter().position(|&id| id == post.id).unwrap());

  let mut unique_tag_ids: Vec<u32> = Vec::new();
  let mut unique_poster_ids: Vec<u32> = Vec::new();

  for post in compact_posts.iter() {
    unique_tag_ids.append(&mut post.tag_ids.clone());
    unique_poster_ids.push(post.poster_id);
  }

  sort_and_dedup_vec(&mut unique_tag_ids);
  sort_and_dedup_vec(&mut unique_poster_ids);

  let compact_tags = app_state
    .tag_repository
    .get_many_compact_tags_by_ids(&unique_tag_ids)
    .await;
  if compact_tags.is_err() {
    // @TODO-ZM: log error reason
    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
  }
  let compact_tags = compact_tags.unwrap();

  let compact_posters = app_state
    .account_repository
    .get_many_compact_accounts_by_ids(unique_poster_ids.clone())
    .await;
  if compact_posters.is_err() {
    // @TODO-ZM: log error reason
    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
  }
  let compact_posters = compact_posters.unwrap();

  Json(json!({
      "posts": compact_posts,
      "tags": compact_tags,
      "posters": compact_posters,
  }))
  .into_response()
}

pub fn create_search_router() -> Router<AppState> {
  Router::new().route("/posts", axum::routing::get(search_posts))
}
