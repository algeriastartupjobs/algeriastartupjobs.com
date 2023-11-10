use super::model::{Account, AccountNameTrait, CompactAccount, DBAccount};
use crate::_utils::error::DataAccessError;
use serde_json::json;
use sqlx::{Pool, Row, Sqlite};
use std::sync::Arc;

pub struct AccountRepository {
  main_sql_db: Arc<Pool<Sqlite>>,
}

impl AccountRepository {
  pub fn new(main_sql_db: Arc<Pool<Sqlite>>) -> Self {
    Self { main_sql_db }
  }

  pub async fn get_many_compact_accounts_by_ids(
    &self,
    ids: Vec<u32>,
  ) -> Result<Vec<CompactAccount>, DataAccessError> {
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
      SELECT id, slug, type, first_name, last_name, company_name
      FROM account
      WHERE id IN ({})
      "#,
        &ids
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
        "Error while getting many compact accounts by ids: {:?}",
        result.err()
      );
      return Err(DataAccessError::InternalError);
    }
    let result = result.unwrap();

    let mut compact_accounts = vec![];

    for row in result {
      let json_account = json!({
        "id": row.get::<u32, _>("id"),
        "slug": row.get::<String, _>("slug"),
        "type": row.get::<String, _>("type"),
        "first_name": row.get::<String, _>("first_name"),
        "last_name": row.get::<String, _>("last_name"),
        "company_name": row.get::<String, _>("company_name"),
      });

      let compact_account = serde_json::from_value::<CompactAccount>(json_account);
      if compact_account.is_err() {
        tracing::error!(
          "Error while getting many compact accounts by ids: {:?}",
          compact_account.err()
        );
        return Err(DataAccessError::InternalError);
      }
      let compact_account = compact_account.unwrap();

      compact_accounts.push(compact_account);
    }

    Ok(compact_accounts)
  }

  pub async fn get_one_account_by_id(&self, id: u32) -> Result<Account, DataAccessError> {
    let conn = self.main_sql_db.acquire().await;
    if conn.is_err() {
      tracing::error!("Error while getting sql connection: {:?}", conn);
      return Err(DataAccessError::InternalError);
    }
    let mut conn = conn.unwrap();

    // @TODO-ZM: use sqlx::query!
    let result = sqlx::query(
      r#"
      SELECT id, email, slug, type, first_name, last_name, company_name, created_at
      FROM account
      WHERE id = $1
      "#,
    )
    .bind(id)
    .fetch_one(&mut *conn)
    .await;

    if result.is_err() {
      tracing::error!("Error while getting one account by id: {:?}", result.err());
      return Err(DataAccessError::InternalError);
    }
    let result = result.unwrap();

    let json_account = json!({
      "id": result.get::<u32, _>("id"),
      "email": result.get::<String, _>("email"),
      "slug": result.get::<String, _>("slug"),
      "type": result.get::<String, _>("type"),
      "first_name": result.get::<String, _>("first_name"),
      "last_name": result.get::<String, _>("last_name"),
      "company_name": result.get::<String, _>("company_name"),
      "created_at": result.get::<String, _>("created_at"),
    });

    let account = serde_json::from_value::<Account>(json_account);
    if account.is_err() {
      tracing::error!("Error while getting one account by id: {:?}", account.err());
      return Err(DataAccessError::InternalError);
    }
    let account = account.unwrap();

    Ok(account)
  }

  pub async fn get_one_account_by_email(&self, email: &String) -> Result<Account, DataAccessError> {
    let conn = self.main_sql_db.acquire().await;
    if conn.is_err() {
      tracing::error!("Error while getting sql connection: {:?}", conn);
      return Err(DataAccessError::InternalError);
    }
    let mut conn = conn.unwrap();

    // @TODO-ZM: use sqlx::query!
    let result = sqlx::query(
      r#"
      SELECT id, email, slug, type, first_name, last_name, company_name, created_at
      FROM account
      WHERE email = $1
      "#,
    )
    .bind(email)
    .fetch_one(&mut *conn)
    .await;

    if result.is_err() {
      match result.err().unwrap() {
        sqlx::Error::RowNotFound => {
          return Err(DataAccessError::NotFound);
        }
        err => {
          tracing::error!("Error while getting one account by email: {:?}", err);
          return Err(DataAccessError::InternalError);
        }
      }
    }
    let result = result.unwrap();

    let json_account = json!({
      "id": result.get::<u32, _>("id"),
      "email": result.get::<String, _>("email"),
      "slug": result.get::<String, _>("slug"),
      "type": result.get::<String, _>("type"),
      "first_name": result.get::<String, _>("first_name"),
      "last_name": result.get::<String, _>("last_name"),
      "company_name": result.get::<String, _>("company_name"),
      "created_at": result.get::<String, _>("created_at"),
    });

    let account = serde_json::from_value::<Account>(json_account);
    if account.is_err() {
      tracing::error!(
        "Error while parsing one account by email: {:?}",
        account.err()
      );
      return Err(DataAccessError::InternalError);
    }
    let account = account.unwrap();

    Ok(account)
  }

  pub async fn create_one_account(&self, account: &DBAccount) -> Result<u32, DataAccessError> {
    let conn = self.main_sql_db.acquire().await;
    if conn.is_err() {
      tracing::error!("Error while getting sql connection: {:?}", conn);
      return Err(DataAccessError::InternalError);
    }
    let mut conn = conn.unwrap();

    let (first_name, last_name, company_name) = account.get_names();

    let db_result = sqlx::query(
      r#"
      INSERT INTO account (email, slug, type, first_name, last_name, company_name, created_at)
      VALUES ($1, $2, $3, $4, $5, $6, strftime('%Y-%m-%dT%H:%M:%S.%fZ', 'now'))
      "#,
    )
    .bind(&account.email)
    .bind(&account.slug)
    .bind(&account.r#type.to_string())
    .bind(first_name)
    .bind(last_name)
    .bind(company_name)
    .execute(&mut *conn)
    .await;
    if db_result.is_err() {
      tracing::error!("Error while creating one account: {:?}", db_result);
      return Err(DataAccessError::InternalError);
    }
    let id = db_result.unwrap().last_insert_rowid() as u32;

    Ok(id)
  }
}
