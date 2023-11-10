use serde_json::json;
use sqlx::{Pool, Row, Sqlite};
use std::sync::Arc;

use super::model::{CompactPost, DBPost, Post};
use crate::_utils::{database::DBOrderDirection, error::DataAccessError};

pub struct PostRepository {
  main_sql_db: Arc<Pool<Sqlite>>,
}

impl PostRepository {
  pub fn new(main_sql_db: Arc<Pool<Sqlite>>) -> Self {
    Self { main_sql_db }
  }

  pub async fn get_many_published_compact_posts(
    &self,
    order_by: &str,
    order_direction: DBOrderDirection,
    limit: u32,
    start: u32,
  ) -> Result<Vec<CompactPost>, DataAccessError> {
    let conn = self.main_sql_db.acquire().await;
    if conn.is_err() {
      tracing::error!("Error while getting sql connection: {:?}", conn);
      return Err(DataAccessError::InternalError);
    }
    let mut conn = conn.unwrap();

    // @TODO-ZM: figure out how query $ replacement work, there is some unneeded "magic" here
    let db_result = sqlx::query(
      format!(
        r#"
      SELECT id, slug, title, poster_id, short_description, tag_ids, published_at
      FROM post
      WHERE is_published = 1 AND is_deleted = 0
      ORDER BY {} {}
      LIMIT $1
      OFFSET $2
      "#,
        order_by, order_direction,
      )
      .as_str(),
    )
    .bind(limit)
    .bind(start)
    .fetch_all(&mut *conn)
    .await;

    if db_result.is_err() {
      tracing::error!(
        "Error while getting many published compact posts: {:?}",
        db_result.err()
      );
      return Err(DataAccessError::InternalError);
    }
    let db_result = db_result.unwrap();

    let mut compact_posts = vec![];

    for row in db_result {
      let tag_ids = row.get::<String, _>("tag_ids");
      let tag_ids = tag_ids
        .split(',')
        .filter(|id| !id.is_empty())
        .map(|id| id.parse::<u32>())
        .collect::<Vec<Result<u32, _>>>();
      if tag_ids.iter().any(|id| id.is_err()) {
        tracing::error!(
          "Error while getting one post by id, on parsing tag_ids, error: {:?}",
          tag_ids
        );
        return Err(DataAccessError::InternalError);
      }

      let tag_ids = tag_ids
        .iter()
        .map(|id| id.clone().unwrap())
        .collect::<Vec<u32>>();
      let json_compact_post = json!({
        "id": row.get::<u32, _>("id"),
        "slug": row.get::<String, _>("slug"),
        "title": row.get::<String, _>("title"),
        "poster_id": row.get::<u32, _>("poster_id"),
        "short_description": row.get::<String, _>("short_description"),
        "tag_ids": tag_ids,
        "published_at": row.get::<String, _>("published_at"),
      });
      let compact_compact_post = serde_json::from_value::<CompactPost>(json_compact_post);
      if compact_compact_post.is_err() {
        tracing::error!(
          "Error while getting many published compact posts, on parsing compact_compact_post, error: {:?}",
          compact_compact_post.err()
        );
        return Err(DataAccessError::InternalError);
      }
      let compact_post = compact_compact_post.unwrap();
      compact_posts.push(compact_post);
    }

    Ok(compact_posts)
  }

  pub async fn get_many_compact_posts_by_ids(
    &self,
    ids: Vec<u32>,
  ) -> Result<Vec<CompactPost>, DataAccessError> {
    let conn = self.main_sql_db.acquire().await;
    if conn.is_err() {
      tracing::error!("Error while getting sql connection: {:?}", conn);
      return Err(DataAccessError::InternalError);
    }
    let mut conn = conn.unwrap();

    let db_result = sqlx::query(
      format!(
        r#"
      SELECT id, slug, title, poster_id, short_description, tag_ids, published_at
      FROM post
      WHERE id IN ({}) AND is_deleted = 0
      "#,
        ids
          .iter()
          .map(|id| id.to_string())
          .collect::<Vec<String>>()
          .join(",")
      )
      .as_str(),
    )
    .fetch_all(&mut *conn)
    .await;

    if db_result.is_err() {
      tracing::error!(
        "Error while getting many compact posts by ids: {:?}",
        db_result.err()
      );
      return Err(DataAccessError::InternalError);
    }

    let db_result = db_result.unwrap();

    let mut compact_posts = vec![];

    for row in db_result {
      let tag_ids = row.get::<String, _>("tag_ids");
      let tag_ids = tag_ids
        .split(',')
        .filter(|id| !id.is_empty())
        .map(|id| id.parse::<u32>())
        .collect::<Vec<Result<u32, _>>>();
      if tag_ids.iter().any(|id| id.is_err()) {
        tracing::error!(
          "Error while getting one post by id, on parsing tag_ids, error: {:?}",
          tag_ids
        );
        return Err(DataAccessError::InternalError);
      }

      let tag_ids = tag_ids
        .iter()
        .map(|id| id.clone().unwrap())
        .collect::<Vec<u32>>();

      let json_compact_post = json!({
        "id": row.get::<u32, _>("id"),
        "slug": row.get::<String, _>("slug"),
        "title": row.get::<String, _>("title"),
        "poster_id": row.get::<u32, _>("poster_id"),
        "short_description": row.get::<String, _>("short_description"),
        "tag_ids": tag_ids,
        "published_at": row.get::<String, _>("published_at"),
      });

      let compact_compact_post = serde_json::from_value::<CompactPost>(json_compact_post);
      if compact_compact_post.is_err() {
        tracing::error!(
          "Error while getting many compact posts by ids, on parsing compact_compact_post, error: {:?}",
          compact_compact_post.err()
        );
        return Err(DataAccessError::InternalError);
      }
      let compact_post = compact_compact_post.unwrap();

      compact_posts.push(compact_post);
    }

    Ok(compact_posts)
  }

  pub async fn get_many_posts_by_ids(&self, ids: Vec<u32>) -> Result<Vec<Post>, DataAccessError> {
    let conn = self.main_sql_db.acquire().await;
    if conn.is_err() {
      tracing::error!("Error while getting sql connection: {:?}", conn);
      return Err(DataAccessError::InternalError);
    }
    let mut conn = conn.unwrap();

    let db_result = sqlx::query(
      format!(
      r#"
      SELECT id, slug, title, poster_id, short_description, description, tag_ids, published_at, is_published
      FROM post
      WHERE id IN ({}) AND is_deleted = 0
      "#,
      ids.iter().map(|id| id.to_string()).collect::<Vec<String>>().join(",")
      ).as_str(),
    )
    .fetch_all(&mut *conn)
    .await;

    if db_result.is_err() {
      tracing::error!(
        "Error while getting many posts by ids: {:?}",
        db_result.err()
      );
      return Err(DataAccessError::InternalError);
    }

    let db_result = db_result.unwrap();

    let mut posts = vec![];

    for row in db_result {
      let tag_ids = row.get::<String, _>("tag_ids");
      let tag_ids = tag_ids
        .split(',')
        .filter(|id| !id.is_empty())
        .map(|id| id.parse::<u32>())
        .collect::<Vec<Result<u32, _>>>();
      if tag_ids.iter().any(|id| id.is_err()) {
        tracing::error!(
          "Error while getting one post by id, on parsing tag_ids, error: {:?}",
          tag_ids
        );
        return Err(DataAccessError::InternalError);
      }

      let tag_ids = tag_ids
        .iter()
        .map(|id| id.clone().unwrap())
        .collect::<Vec<u32>>();

      let json_post = json!({
        "id": row.get::<u32, _>("id"),
        "slug": row.get::<String, _>("slug"),
        "title": row.get::<String, _>("title"),
        "poster_id": row.get::<u32, _>("poster_id"),
        "short_description": row.get::<String, _>("short_description"),
        "description": row.get::<String, _>("description"),
        "tag_ids": tag_ids,
        "published_at": row.get::<String, _>("published_at"),
        "is_published": row.get::<bool, _>("is_published"),
      });

      let post = serde_json::from_value::<Post>(json_post);
      if post.is_err() {
        tracing::error!("Error while getting many posts by ids: {:?}", post.err());
        return Err(DataAccessError::InternalError);
      }
      let post = post.unwrap();

      posts.push(post);
    }

    Ok(posts)
  }

  pub async fn get_one_post_by_id(&self, id: u32) -> Result<Post, DataAccessError> {
    let conn = self.main_sql_db.acquire().await;
    if conn.is_err() {
      tracing::error!("Error while getting sql connection: {:?}", conn);
      return Err(DataAccessError::InternalError);
    }
    let mut conn = conn.unwrap();

    // @TODO-ZM: use * instead of listing all the fields?
    let db_result = sqlx::query(
      r#"
      SELECT id, slug, title, poster_id, short_description, description, tag_ids, published_at, is_published
      FROM post
      WHERE id = $1 AND is_deleted = 0
      "#,
    )
    .bind(id)
    .fetch_one(&mut *conn)
    .await;

    if db_result.is_err() {
      match db_result.err().unwrap() {
        sqlx::Error::RowNotFound => {
          return Err(DataAccessError::NotFound);
        }
        err => {
          tracing::error!("Error while getting one post by id: {:?}", err);
          return Err(DataAccessError::InternalError);
        }
      }
    }

    let db_result = db_result.unwrap();

    let tag_ids = db_result.get::<String, _>("tag_ids");
    let tag_ids = tag_ids
      .split(',')
      .filter(|id| !id.is_empty())
      .map(|id| id.parse::<u32>())
      .collect::<Vec<Result<u32, _>>>();
    if tag_ids.iter().any(|id| id.is_err()) {
      tracing::error!(
        "Error while getting one post by id, on parsing tag_ids, error: {:?}",
        tag_ids
      );
      return Err(DataAccessError::InternalError);
    }

    let tag_ids = tag_ids
      .iter()
      .map(|id| id.clone().unwrap())
      .collect::<Vec<u32>>();

    let json_post = json!({
      "id": db_result.get::<u32, _>("id"),
      "slug": db_result.get::<String, _>("slug"),
      "title": db_result.get::<String, _>("title"),
      "poster_id": db_result.get::<u32, _>("poster_id"),
      "short_description": db_result.get::<String, _>("short_description"),
      "description": db_result.get::<String, _>("description"),
      "tag_ids": tag_ids,
      "published_at": db_result.get::<String, _>("published_at"),
      "is_published": db_result.get::<bool, _>("is_published"),
    });

    let post = serde_json::from_value::<Post>(json_post);
    if post.is_err() {
      tracing::error!("Error while getting one post by id: {:?}", post.err());
      return Err(DataAccessError::InternalError);
    }
    let post = post.unwrap();

    Ok(post)
  }

  pub async fn create_one_post(&self, post: &DBPost) -> Result<u32, DataAccessError> {
    let conn = self.main_sql_db.acquire().await;
    if conn.is_err() {
      tracing::error!("Error while getting sql connection: {:?}", conn);
      return Err(DataAccessError::InternalError);
    }
    let mut conn = conn.unwrap();

    let db_result = sqlx::query(
      r#"
      INSERT INTO post (slug, title, poster_id, short_description, description, tag_ids, published_at, created_at)
      VALUES ($1, $2, $3, $4, $5, $6, $7, strftime('%Y-%m-%dT%H:%M:%S.%fZ', 'now'))
      "#,
    )
    .bind(&post.slug)
    .bind(&post.title)
    .bind(post.poster_id)
    .bind(&post.short_description)
    .bind(&post.description)
    .bind(&post.tag_ids.iter().map(|id| id.to_string()).collect::<Vec<String>>().join(","))
    .bind(&post.published_at)
    .execute(&mut *conn)
    .await;

    if db_result.is_err() {
      tracing::error!("Error while creating one post: {:?}", db_result);
      return Err(DataAccessError::InternalError);
    }

    let id = db_result.unwrap().last_insert_rowid() as u32;
    Ok(id)
  }

  pub async fn get_published_post_count(&self) -> Result<u32, DataAccessError> {
    let conn = self.main_sql_db.acquire().await;
    if conn.is_err() {
      tracing::error!("Error while getting sql connection: {:?}", conn);
      return Err(DataAccessError::InternalError);
    }
    let mut conn = conn.unwrap();

    let db_result = sqlx::query(
      r#"
      SELECT COUNT(*) as count
      FROM post
      WHERE is_published = 1 AND is_deleted = 0
      "#,
    )
    .fetch_one(&mut *conn)
    .await;

    if db_result.is_err() {
      tracing::error!(
        "Error while getting published post count: {:?}",
        db_result.err()
      );
      return Err(DataAccessError::InternalError);
    }

    let db_result = db_result.unwrap();
    let count = db_result.get::<i64, _>("count") as u32;

    Ok(count)
  }

  pub async fn delete_one_post_by_id(&self, id: u32) -> Result<(), DataAccessError> {
    let conn = self.main_sql_db.acquire().await;
    if conn.is_err() {
      tracing::error!("Error while getting sql connection: {:?}", conn);
      return Err(DataAccessError::InternalError);
    }
    let mut conn = conn.unwrap();

    let db_result = sqlx::query(
      r#"
      UPDATE post
      SET deleted_at = strftime('%Y-%m-%dT%H:%M:%S.%fZ', 'now')
      WHERE id = $1
      "#,
    )
    .bind(id)
    .execute(&mut *conn)
    .await;

    if db_result.is_err() {
      tracing::error!("Error while deleting one post: {:?}", db_result);
      return Err(DataAccessError::InternalError);
    }

    Ok(())
  }

  pub async fn publish_one_post_by_id(&self, id: u32) -> Result<(), DataAccessError> {
    let conn = self.main_sql_db.acquire().await;
    if conn.is_err() {
      tracing::error!("Error while getting sql connection: {:?}", conn);
      return Err(DataAccessError::InternalError);
    }
    let mut conn = conn.unwrap();

    let db_result = sqlx::query(
      r#"
      UPDATE post
      SET published_at = strftime('%Y-%m-%dT%H:%M:%S.%fZ', 'now')
      WHERE id = $1
      "#,
    )
    .bind(id)
    .execute(&mut *conn)
    .await;

    if db_result.is_err() {
      tracing::error!("Error while publishing one post: {:?}", db_result);
      return Err(DataAccessError::InternalError);
    }

    Ok(())
  }
}
