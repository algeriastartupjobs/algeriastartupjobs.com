use axum::{
  extract::{Path, Query, State},
  response::IntoResponse,
  Json, Router,
};
use hyper::StatusCode;
use serde_json::json;

use crate::{
  _entry::state::AppState,
  _utils::{
    error::DataAccessError,
    query::{PaginationQuery, PaginationQueryTrait},
    vec::sort_and_dedup_vec,
  },
  account::model::{AccountTrait, CompactAccount},
  category::model::CategoryTrait,
  tag::model::{CompactTag, TagTrait},
};

use super::model::{CompactPost, PostTrait};

pub async fn get_all_posts_for_feed(State(app_state): State<AppState>) -> impl IntoResponse {
  let compact_posts = app_state
    .post_repository
    .get_many_compact_posts_by_filter("true", 20, 0)
    .await;
  if !compact_posts.is_ok() {
    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
  }
  let compact_posts = compact_posts.unwrap();

  let mut unique_category_ids: Vec<u32> = Vec::new();
  let mut unique_tag_ids: Vec<u32> = Vec::new();
  let mut unique_poster_ids: Vec<u32> = Vec::new();

  for post in compact_posts.iter() {
    unique_category_ids.push(post.category_id);
    unique_tag_ids.append(&mut post.tag_ids.clone());
    unique_poster_ids.push(post.poster_id);
  }

  sort_and_dedup_vec(&mut unique_category_ids);
  sort_and_dedup_vec(&mut unique_tag_ids);
  sort_and_dedup_vec(&mut unique_poster_ids);

  let compact_categories = app_state
    .category_repository
    .get_many_compact_categories_by_ids(unique_category_ids.clone())
    .await;
  if !compact_categories.is_ok() {
    // @TODO-ZM: log error reason
    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
  }
  let compact_categories = compact_categories.unwrap();

  let compact_tags = app_state
    .tag_repository
    .get_many_compact_tags_by_ids(unique_tag_ids.clone())
    .await;
  if !compact_tags.is_ok() {
    // @TODO-ZM: log error reason
    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
  }
  let compact_tags = compact_tags.unwrap();

  let compact_posters = app_state
    .account_repository
    .get_many_compact_accounts_by_ids(unique_poster_ids.clone())
    .await;
  if !compact_posters.is_ok() {
    // @TODO-ZM: log error reason
    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
  }
  let compact_posters = compact_posters.unwrap();

  Json(json!({
      "posts": compact_posts,
      "categories": compact_categories,
      "tags": compact_tags,
      "posters": compact_posters,
  }))
  .into_response()
}

pub async fn get_one_post_by_id(
  State(app_state): State<AppState>,
  Path(id): Path<u32>,
) -> impl IntoResponse {
  let post = app_state.post_repository.get_one_post_by_id(id).await;

  if !post.is_ok() {
    match post {
      Err(DataAccessError::NotFound) => {
        return StatusCode::NOT_FOUND.into_response();
      }
      _ => {
        // @TODO-ZM: log error reason
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
      }
    }
  }
  let post = post.unwrap();

  let category = app_state
    .category_repository
    .get_one_category_by_id(post.category_id)
    .await;
  if !category.is_ok() {
    // @TODO-ZM: log error reason
    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
  }
  let category = category.unwrap();

  let compact_tags = app_state
    .tag_repository
    .get_many_compact_tags_by_ids(post.tag_ids.clone())
    .await;
  if !compact_tags.is_ok() {
    // @TODO-ZM: log error reason
    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
  }
  let compact_tags = compact_tags.unwrap();

  let poster = app_state
    .account_repository
    .get_one_account_by_id(post.poster_id)
    .await;
  if !poster.is_ok() {
    // @TODO-ZM: log error reason
    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
  }
  let poster = poster.unwrap();

  let compact_category = category.to_compact_category();
  let compact_poster = poster.to_compact_account();

  Json(json!({
      "post": post,
      "category": compact_category,
      "tags": compact_tags,
      "poster": compact_poster,
  }))
  .into_response()
}

pub async fn get_many_similar_posts_by_id(
  State(app_state): State<AppState>,
  Path(id): Path<u32>,
  pagination: Query<PaginationQuery>,
) -> impl IntoResponse {
  let db_pagination = pagination.to_db_query();
  let similar_compact_posts = app_state
    .post_repository
    .get_many_similar_compact_posts_by_id(id, db_pagination.limit, db_pagination.limit)
    .await;

  if !similar_compact_posts.is_ok() {
    match similar_compact_posts {
      Err(DataAccessError::NotFound) => {
        return StatusCode::NOT_FOUND.into_response();
      }
      _ => {
        // @TODO-ZM: log error reason
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
      }
    }
  }
  let similar_compact_posts = similar_compact_posts.unwrap();

  let mut unique_category_ids: Vec<u32> = Vec::new();
  let mut unique_tag_ids: Vec<u32> = Vec::new();
  let mut unique_poster_ids: Vec<u32> = Vec::new();

  for post in similar_compact_posts.iter() {
    unique_category_ids.push(post.category_id);
    unique_tag_ids.append(&mut post.tag_ids.clone());
    unique_poster_ids.push(post.poster_id);
  }

  sort_and_dedup_vec(&mut unique_category_ids);
  sort_and_dedup_vec(&mut unique_tag_ids);
  sort_and_dedup_vec(&mut unique_poster_ids);

  let compact_categories = app_state
    .category_repository
    .get_many_compact_categories_by_ids(unique_category_ids.clone())
    .await;
  if !compact_categories.is_ok() {
    // @TODO-ZM: log error reason
    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
  }
  let compact_categories = compact_categories.unwrap();

  let compact_tags = app_state
    .tag_repository
    .get_many_compact_tags_by_ids(unique_tag_ids.clone())
    .await;
  if !compact_tags.is_ok() {
    // @TODO-ZM: log error reason
    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
  }
  let compact_tags = compact_tags.unwrap();

  let compact_posters = app_state
    .account_repository
    .get_many_compact_accounts_by_ids(unique_poster_ids.clone())
    .await;
  if !compact_posters.is_ok() {
    // @TODO-ZM: log error reason
    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
  }
  let compact_posters = compact_posters.unwrap();

  Json(json!({
      "posts": similar_compact_posts,
      "categories": compact_categories,
      "tags": compact_tags,
      "posters": compact_posters,
  }))
  .into_response()
}

pub fn create_post_router() -> Router<AppState> {
  Router::new()
    .route("/feed", axum::routing::get(get_all_posts_for_feed))
    .route("/:post_id", axum::routing::get(get_one_post_by_id))
    .route(
      "/:post_id/similar",
      axum::routing::get(get_many_similar_posts_by_id),
    )
}