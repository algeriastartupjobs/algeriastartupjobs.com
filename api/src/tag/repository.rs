use serde_json::json;
use sqlx::{Pool, Row, Sqlite};
use std::sync::Arc;

use super::model::{CompactTag, DBTag, Tag};
use crate::_utils::error::DataAccessError;

pub struct TagRepository {
  main_sql_db: Arc<Pool<Sqlite>>,
}

impl TagRepository {
  pub fn new(main_sql_db: Arc<Pool<Sqlite>>) -> Self {
    Self { main_sql_db }
  }

  pub async fn get_many_compact_tags_by_names(
    &self,
    names: Vec<String>,
  ) -> Result<Vec<CompactTag>, DataAccessError> {
    let conn = self.main_sql_db.acquire().await;
    if conn.is_err() {
      tracing::error!("Error while getting sql connection: {:?}", conn);
      return Err(DataAccessError::InternalError);
    }
    let mut conn = conn.unwrap();

    // @TODO-ZM: use sqlx::query!
    let result = sqlx::query(
      format!(
        r#"
      SELECT id, name, slug
      FROM tag
      WHERE name IN ({})
      "#,
        names
          .iter()
          .map(|name| format!("'{}'", name))
          .collect::<Vec<String>>()
          .join(",")
      )
      .as_str(),
    )
    .fetch_all(&mut *conn)
    .await;

    if result.is_err() {
      tracing::error!(
        "Error while getting many compact tags by names: {:?}",
        result.err()
      );
      return Err(DataAccessError::InternalError);
    }
    let result = result.unwrap();

    let mut compact_tags = vec![];

    for row in result {
      let json_tag = json!({
        "id": row.get::<u32, _>("id"),
        "name": row.get::<String, _>("name"),
        "slug": row.get::<String, _>("slug"),
      });
      let compact_tag: CompactTag = serde_json::from_value(json_tag).unwrap();
      compact_tags.push(compact_tag);
    }

    Ok(compact_tags)
  }

  pub async fn get_many_compact_tags_by_ids(
    &self,
    ids: &Vec<u32>,
  ) -> Result<Vec<CompactTag>, DataAccessError> {
    let conn = self.main_sql_db.acquire().await;
    if conn.is_err() {
      tracing::error!("Error while getting sql connection: {:?}", conn);
      return Err(DataAccessError::InternalError);
    }
    let mut conn = conn.unwrap();

    // @TODO-ZM: use sqlx::query!
    let result = sqlx::query(
      format!(
        r#"
      SELECT id, name, slug
      FROM tag
      WHERE id IN ({})
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

    if result.is_err() {
      tracing::error!(
        "Error while getting many compact tags by ids: {:?}",
        result.err()
      );
      return Err(DataAccessError::InternalError);
    }
    let result = result.unwrap();

    let mut compact_tags = vec![];

    for row in result {
      let json_tag = json!({
        "id": row.get::<u32, _>("id"),
        "name": row.get::<String, _>("name"),
        "slug": row.get::<String, _>("slug"),
      });
      let compact_tag: CompactTag = serde_json::from_value(json_tag).unwrap();
      compact_tags.push(compact_tag);
    }

    Ok(compact_tags)
  }

  pub async fn get_one_tag_by_slug(&self, slug: &str) -> Result<Tag, DataAccessError> {
    let conn = self.main_sql_db.acquire().await;
    if conn.is_err() {
      tracing::error!("Error while getting sql connection: {:?}", conn);
      return Err(DataAccessError::InternalError);
    }
    let mut conn = conn.unwrap();

    let db_result = sqlx::query(
      r#"
      SELECT id, name, slug, created_at
      FROM tag
      WHERE slug = $1
      "#,
    )
    .bind(slug)
    .fetch_one(&mut *conn)
    .await;
    if db_result.is_err() {
      tracing::error!("Error while getting one tag by slug: {:?}", db_result.err());
      return Err(DataAccessError::InternalError);
    }
    let db_result = db_result.unwrap();

    let tag = Tag {
      id: db_result.get::<u32, _>("id"),
      name: db_result.get::<String, _>("name"),
      slug: db_result.get::<String, _>("slug"),
      created_at: db_result.get::<String, _>("created_at"),
    };

    Ok(tag)
  }

  pub async fn create_one_tag(&self, tag: DBTag) -> Result<u32, DataAccessError> {
    let conn = self.main_sql_db.acquire().await;
    if conn.is_err() {
      tracing::error!("Error while getting sql connection: {:?}", conn);
      return Err(DataAccessError::InternalError);
    }
    let mut conn = conn.unwrap();

    let db_result = sqlx::query(
      r#"
      INSERT INTO tag (name, slug, created_at)
      VALUES ($1, $2, strftime('%Y-%m-%dT%H:%M:%S.%fZ', 'now'))
      "#,
    )
    .bind(&tag.name)
    .bind(&tag.slug)
    .execute(&mut *conn)
    .await;
    if db_result.is_err() {
      tracing::error!("Error while creating one account: {:?}", db_result);
      return Err(DataAccessError::InternalError);
    }
    let id = db_result.unwrap().last_insert_rowid() as u32;

    Ok(id)
  }

  pub async fn get_many_compact_tags(&self) -> Result<Vec<Tag>, DataAccessError> {
    let conn = self.main_sql_db.acquire().await;
    if conn.is_err() {
      tracing::error!("Error while getting sql connection: {:?}", conn);
      return Err(DataAccessError::InternalError);
    }
    let mut conn = conn.unwrap();

    let db_result = sqlx::query(
      r#"
      SELECT id, name, slug, created_at
      FROM tag
      "#,
    )
    .fetch_all(&mut *conn)
    .await;
    if db_result.is_err() {
      tracing::error!(
        "Error while getting many compact tags: {:?}",
        db_result.err()
      );
      return Err(DataAccessError::InternalError);
    }
    let db_result = db_result.unwrap();

    let mut tags = vec![];

    for row in db_result {
      let tag = Tag {
        id: row.get::<u32, _>("id"),
        name: row.get::<String, _>("name"),
        slug: row.get::<String, _>("slug"),
        created_at: row.get::<String, _>("created_at"),
      };
      tags.push(tag);
    }

    Ok(tags)
  }
}
