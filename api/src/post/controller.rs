use axum::{
  extract::{ConnectInfo, Path, State},
  response::IntoResponse,
  Json, Router,
};
use hyper::StatusCode;
use rand::{distributions::Alphanumeric, prelude::Distribution, thread_rng};
use serde::Deserialize;
use serde_json::json;
use std::net::SocketAddr;

use super::model::DBPost;
use crate::{
  _entry::state::AppState,
  _utils::{
    database::DBOrderDirection,
    error::{DataAccessError, SecurityError},
    string::slugify,
    vec::sort_and_dedup_vec,
  },
  account::model::{AccountNameTrait, DBAccount},
  auth::service::{ScopedToken, TokenScope},
  security::service::RateLimitConstraint,
  task::model::{DBTask, TaskName, TaskStatus, TaskType},
};

pub async fn get_all_posts_for_feed(State(app_state): State<AppState>) -> impl IntoResponse {
  let compact_posts = app_state
    .post_repository
    .get_many_published_compact_posts("published_at", DBOrderDirection::DESC, 20, 0)
    .await;
  if compact_posts.is_err() {
    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
  }
  let compact_posts = compact_posts.unwrap();

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

pub async fn get_one_post_by_id(
  State(app_state): State<AppState>,
  Path(id): Path<u32>,
) -> impl IntoResponse {
  let post = app_state.post_repository.get_one_post_by_id(id).await;

  if post.is_err() {
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

  let compact_tags = app_state
    .tag_repository
    .get_many_compact_tags_by_ids(&post.tag_ids)
    .await;
  if compact_tags.is_err() {
    // @TODO-ZM: log error reason
    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
  }
  let compact_tags = compact_tags.unwrap();

  let poster = app_state
    .account_repository
    .get_one_account_by_id(post.poster_id)
    .await;
  if poster.is_err() {
    // @TODO-ZM: log error reason
    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
  }
  let poster = poster.unwrap();

  Json(json!({
      "post": post,
      "tags": compact_tags,
      "poster": poster,
  }))
  .into_response()
}

pub async fn get_many_similar_posts_by_id(
  State(app_state): State<AppState>,
  Path(id): Path<u32>,
) -> impl IntoResponse {
  let post = app_state.post_repository.get_one_post_by_id(id).await;
  if post.is_err() {
    // @TODO-ZM: log error reason
    return StatusCode::NOT_FOUND.into_response();
  }
  let post = post.unwrap();

  let poster = app_state
    .account_repository
    .get_one_account_by_id(post.poster_id)
    .await;
  if poster.is_err() {
    // @TODO-ZM: log error reason
    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
  }
  let poster = poster.unwrap();

  let tags = app_state
    .tag_repository
    .get_many_compact_tags_by_ids(&post.tag_ids)
    .await;
  if tags.is_err() {
    // @TODO-ZM: log error reason
    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
  }
  let tags = tags.unwrap();

  let post_ids = app_state
    .search_service
    .search_posts(&format!(
      "{} {} {}",
      post.title,
      poster.get_display_name(),
      tags
        .iter()
        .map(|tag| tag.name.clone())
        .collect::<Vec<String>>()
        .join(" ")
    ))
    .await;

  if post_ids.is_err() {
    // @TODO-ZM: log error reason
    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
  }
  let post_ids = post_ids.unwrap();
  let post_ids = post_ids
    .into_iter()
    .filter(|&id| id != post.id)
    .collect::<Vec<u32>>();

  let similar_compact_posts = app_state
    .post_repository
    .get_many_compact_posts_by_ids(post_ids.clone())
    .await;

  if similar_compact_posts.is_err() {
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

  let mut unique_tag_ids: Vec<u32> = Vec::new();
  let mut unique_poster_ids: Vec<u32> = Vec::new();

  for post in similar_compact_posts.iter() {
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
      "posts": similar_compact_posts,
      "tags": compact_tags,
      "posters": compact_posters,
  }))
  .into_response()
}

pub async fn get_post_count(State(app_state): State<AppState>) -> impl IntoResponse {
  let post_count = app_state.post_repository.get_published_post_count().await;

  if post_count.is_err() {
    // @TODO-ZM: log error reason
    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
  }
  let post_count = post_count.unwrap();

  Json(json!({
      "count": post_count,
  }))
  .into_response()
}

pub async fn get_many_compact_posts_for_tag(
  State(app_state): State<AppState>,
  Path(tag_slug): Path<String>,
) -> impl IntoResponse {
  let tag = app_state
    .tag_repository
    .get_one_tag_by_slug(&tag_slug)
    .await;
  if tag.is_err() {
    // @TODO-ZM: log error reason
    return StatusCode::NOT_FOUND.into_response();
  }
  let tag = tag.unwrap();

  let post_ids = app_state.search_service.search_posts(&tag.name).await;

  if post_ids.is_err() {
    // @TODO-ZM: log error reason
    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
  }
  let post_ids = post_ids.unwrap();

  let compact_posts = app_state
    .post_repository
    .get_many_compact_posts_by_ids(post_ids)
    .await;

  if compact_posts.is_err() {
    // @TODO-ZM: log error reason
    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
  }
  let compact_posts = compact_posts.unwrap();

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
      "tag": tag,
      "posts": compact_posts,
      "tags": compact_tags,
      "posters": compact_posters,
  }))
  .into_response()
}

#[derive(Deserialize)]
pub struct CreateOnePostWithPosterBody {
  poster: DBAccount,
  post: DBPost,
}

pub async fn create_one_post_with_poster(
  // @TODO-ZM: make sure this is a secure ip
  ConnectInfo(ip): ConnectInfo<SocketAddr>,
  State(app_state): State<AppState>,
  Json(body): Json<CreateOnePostWithPosterBody>,
) -> impl IntoResponse {
  match body.poster.r#type.to_string().as_str() {
    "Individual" | "Company" => {}
    _ => {
      return StatusCode::BAD_REQUEST.into_response();
    }
  }

  // @TODO-ZM: write a macro for this
  match app_state.security_service.rate_limit(vec![
    RateLimitConstraint {
      id: format!("create_one_post_with_poster-1-{}", body.poster.email),
      max_requests: 1,
      duration_ms: 2000,
    },
    RateLimitConstraint {
      id: format!("create_one_post_with_poster-2-{}", ip.ip()),
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

  let poster_id;

  let existing_poster = app_state
    .account_repository
    .get_one_account_by_email(&body.poster.email)
    .await;

  if existing_poster.is_err() {
    match existing_poster {
      Err(DataAccessError::NotFound) => {
        let poster_id_result = app_state
          .account_repository
          .create_one_account(&DBAccount {
            slug: slugify(&body.poster.get_display_name()),
            ..body.poster.clone()
          })
          .await;

        if poster_id_result.is_err() {
          // @TODO-ZM: log error reason
          return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }

        poster_id = poster_id_result.unwrap();
      }
      _ => {
        // @TODO-ZM: log error reason
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
      }
    }
  } else {
    poster_id = existing_poster.unwrap().id;
  }

  let compact_tags = app_state
    .tag_repository
    .get_many_compact_tags_by_ids(&body.post.tag_ids)
    .await;
  if compact_tags.is_err() {
    // @TODO-ZM: log error reason
    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
  }
  let compact_tags = compact_tags.unwrap();

  let post_id = app_state
    .post_repository
    .create_one_post(&DBPost {
      poster_id,
      slug: slugify(&body.post.title),
      is_published: false,
      // @TODO-ZM: summarize description using AI
      short_description: body
        .post
        .description
        .split_whitespace()
        .take(20)
        .collect::<Vec<&str>>()
        .join(" "),
      tag_ids: compact_tags.iter().map(|tag| tag.id).collect::<Vec<u32>>(),
      ..body.post.clone()
    })
    .await;

  if post_id.is_err() {
    // @TODO-ZM: log error reason
    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
  }
  let post_id = post_id.unwrap();

  // @TODO-ZM: use generate_confirmation_object from AuthService
  let random_16: String = Alphanumeric
    .sample_iter(&mut thread_rng())
    .take(16)
    .map(char::from)
    .collect();
  let random_16 = random_16.to_uppercase();
  let confirmation_id = &random_16[..12];
  let confirmation_code = &random_16[12..];

  let kv_db_result = app_state
    .main_kv_db
    .insert(post_id.to_be_bytes(), random_16.as_bytes());
  if kv_db_result.is_err() {
    // @TODO-ZM: log error reason
    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
  }

  let email_result = app_state
    .email_service
    .send_one_email(
      &body.poster.email,
      &"Confirm your email".to_string(),
      &format!(
        r#"Your email is used to create a FREE job post at algeriastartupjobs.com with title:

{}

Please confirm your email by copying the code below into the confirmation page:

<div style="width: 100%; text-align: center;">
  <span style="font-size: x-large; letter-spacing: .2em; border: 1px solid #9999; border-radius: .2em; padding: .4em; display: inline-block;">{}</span>
</div>

Thank you for using our service!

ASJ Team
contact@algeriastartupjobs.com
https://www.algeriastartupjobs.com
"#,
        &body.post.title, confirmation_code,
      ),
    )
    .await;

  if email_result.is_err() {
    // @TODO-ZM: log error reason
    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
  }

  Json(json!({
      "post_id": post_id,
      "poster_id": poster_id,
      "confirmation_id": confirmation_id,
  }))
  .into_response()
}

#[derive(Deserialize)]
pub struct ConfirmPostBody {
  post_id: u32,
  confirmation_id: String,
  confirmation_code: String,
}

pub async fn confirm_post(
  ConnectInfo(ip): ConnectInfo<SocketAddr>,
  State(app_state): State<AppState>,
  Json(body): Json<ConfirmPostBody>,
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

  let kv_db_result = app_state.main_kv_db.compare_and_swap(
    body.post_id.to_be_bytes(),
    Some(format!("{}{}", body.confirmation_id, body.confirmation_code).as_bytes()),
    None as Option<&[u8]>,
  );

  if kv_db_result.is_err() || kv_db_result.unwrap().is_err() {
    // @TODO-ZM: log error reason
    return StatusCode::UNAUTHORIZED.into_response();
  }

  let update_result = app_state
    .post_repository
    .publish_one_post_by_id(body.post_id)
    .await;
  if update_result.is_err() {
    // @TODO-ZM: log error reason
    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
  }

  let post = app_state
    .post_repository
    .get_one_post_by_id(body.post_id)
    .await;
  if post.is_err() {
    // @TODO-ZM: log error reason
    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
  }
  let post = post.unwrap();

  let task_id = app_state
    .task_repository
    .create_one_task(DBTask {
      name: TaskName::Indexing {
        model_name: "post".to_string(),
        model_id: post.id,
      },
      status: TaskStatus::Pending,
      r#type: TaskType::Automated,
    })
    .await;
  if task_id.is_err() {
    // @TODO-ZM: log error reason
    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
  }

  let poster = app_state
    .account_repository
    .get_one_account_by_id(post.poster_id)
    .await;
  if poster.is_err() {
    // @TODO-ZM: log error reason
    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
  }
  let poster = poster.unwrap();

  let compact_tags = app_state
    .tag_repository
    .get_many_compact_tags_by_ids(&post.tag_ids)
    .await;
  if compact_tags.is_err() {
    // @TODO-ZM: log error reason
    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
  }
  let compact_tags = compact_tags.unwrap();

  let auth_token = app_state
    .auth_service
    .generate_scoped_token(TokenScope::CreatePost, poster.id)
    .await;
  if auth_token.is_err() {
    // @TODO-ZM: log error reason
    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
  }
  let auth_token = auth_token.unwrap();

  Json(json!({
      "post": post,
      "poster": poster,
      "tags": compact_tags,
      "auth_token": auth_token,
  }))
  .into_response()
}

#[derive(Deserialize)]
pub struct CreateOnePostBody {
  post: DBPost,
}

pub async fn create_one_post(
  ConnectInfo(ip): ConnectInfo<SocketAddr>,
  State(app_state): State<AppState>,
  scoped_token: ScopedToken,
  Json(body): Json<CreateOnePostBody>,
) -> impl IntoResponse {
  let poster = app_state
    .account_repository
    .get_one_account_by_id(scoped_token.id)
    .await;

  if poster.is_err() {
    // @TODO-ZM: log error reason
    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
  }
  let poster = poster.unwrap();

  match poster.r#type.to_string().as_str() {
    "Individual" | "Company" => {}
    _ => {
      return StatusCode::BAD_REQUEST.into_response();
    }
  }

  // @TODO-ZM: write a macro for this
  match app_state.security_service.rate_limit(vec![
    RateLimitConstraint {
      id: format!("create_one_post_with_poster-1-{}", poster.email),
      max_requests: 1,
      duration_ms: 2000,
    },
    RateLimitConstraint {
      id: format!("create_one_post_with_poster-2-{}", ip.ip()),
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

  let compact_tags = app_state
    .tag_repository
    .get_many_compact_tags_by_ids(&body.post.tag_ids)
    .await;
  if compact_tags.is_err() {
    // @TODO-ZM: log error reason
    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
  }
  let compact_tags = compact_tags.unwrap();

  let post_id = app_state
    .post_repository
    .create_one_post(&DBPost {
      poster_id: poster.id,
      slug: slugify(&body.post.title),
      is_published: true,
      // @TODO-ZM: summarize description using AI
      short_description: body
        .post
        .description
        .split_whitespace()
        .take(20)
        .collect::<Vec<&str>>()
        .join(" "),
      published_at: chrono::Utc::now().to_rfc3339(),
      tag_ids: compact_tags.iter().map(|tag| tag.id).collect::<Vec<u32>>(),
      ..body.post.clone()
    })
    .await;

  if post_id.is_err() {
    // @TODO-ZM: log error reason
    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
  }
  let post_id = post_id.unwrap();

  let post = app_state.post_repository.get_one_post_by_id(post_id).await;
  if post.is_err() {
    // @TODO-ZM: log error reason
    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
  }
  let post = post.unwrap();

  let task_id = app_state
    .task_repository
    .create_one_task(DBTask {
      name: TaskName::Indexing {
        model_name: "post".to_string(),
        model_id: post.id,
      },
      status: TaskStatus::Pending,
      r#type: TaskType::Automated,
    })
    .await;
  if task_id.is_err() {
    // @TODO-ZM: log error reason
    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
  }

  let poster = app_state
    .account_repository
    .get_one_account_by_id(post.poster_id)
    .await;
  if poster.is_err() {
    // @TODO-ZM: log error reason
    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
  }
  let poster = poster.unwrap();

  Json(json!({
      "post": post,
      "poster": poster,
      "tags": compact_tags,
  }))
  .into_response()
}

pub async fn delete_one_post_by_id(
  ConnectInfo(ip): ConnectInfo<SocketAddr>,
  State(app_state): State<AppState>,
  scoped_token: ScopedToken,
  Path(id): Path<u32>,
) -> impl IntoResponse {
  let post = app_state.post_repository.get_one_post_by_id(id).await;
  if post.is_err() {
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

  if post.poster_id != scoped_token.id {
    return StatusCode::FORBIDDEN.into_response();
  }

  // @TODO-ZM: write a macro for this
  match app_state.security_service.rate_limit(vec![
    RateLimitConstraint {
      id: format!("delete_one_post_by_id-1-{}", scoped_token.id),
      max_requests: 1,
      duration_ms: 2000,
    },
    RateLimitConstraint {
      id: format!("delete_one_post_by_id-2-{}", ip.ip()),
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

  let delete_result = app_state.post_repository.delete_one_post_by_id(id).await;
  if delete_result.is_err() {
    // @TODO-ZM: log error reason
    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
  }

  let task_id = app_state
    .task_repository
    .create_one_task(DBTask {
      name: TaskName::UndoIndexing {
        model_name: "post".to_string(),
        model_id: post.id,
      },
      status: TaskStatus::Pending,
      r#type: TaskType::Automated,
    })
    .await;

  if task_id.is_err() {
    // @TODO-ZM: log error reason
    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
  }

  StatusCode::NO_CONTENT.into_response()
}

pub fn create_post_router() -> Router<AppState> {
  Router::new()
    .route("/feed", axum::routing::get(get_all_posts_for_feed))
    .route("/:post_id", axum::routing::get(get_one_post_by_id))
    .route("/:post_id", axum::routing::delete(delete_one_post_by_id))
    .route(
      "/:post_id/similar",
      axum::routing::get(get_many_similar_posts_by_id),
    )
    .route("/count", axum::routing::get(get_post_count))
    .route("/confirm", axum::routing::post(confirm_post))
    .route(
      "/via_email",
      axum::routing::post(create_one_post_with_poster),
    )
    .route(
      "/for_tag/:tag_slug",
      axum::routing::get(get_many_compact_posts_for_tag),
    )
    .route("/", axum::routing::post(create_one_post))
}
