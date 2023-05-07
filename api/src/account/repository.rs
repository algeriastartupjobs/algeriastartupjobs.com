use super::model::{Account, AccountType, CompactAccount, DBAccount};
use crate::_utils::{
  database::{db_thing_to_id, DBRecord},
  error::DataAccessError,
  string::escape_single_quote,
};
use std::sync::Arc;
use surrealdb::{engine::remote::ws::Client, Surreal};

pub struct AccountRepository {
  pub db: Arc<Surreal<Client>>,
}

impl AccountRepository {
  pub async fn get_many_compact_accounts_by_filter(
    &self,
    filter: &str,
    limit: u32,
    start: u32,
  ) -> Result<Vec<CompactAccount>, DataAccessError> {
    let query = format!(
      r#"
      SELECT slug, type, first_name, last_name, company_name, id.id as id FROM account WHERE {} LIMIT {} START {}
      "#,
      filter, limit, start
    );

    let query_result = self.db.query(&query).await;

    match query_result {
      Ok(mut query_result) => {
        let query_result_string = format!("{:?}", query_result);
        let accounts: Result<Vec<CompactAccount>, _> = query_result.take(0);
        if accounts.as_ref().is_err() {
          tracing::error!(
            "Error while getting many accounts by filter, error: {:?} | query: {}",
            accounts.as_ref(),
            query_result_string
          );
          return Err(DataAccessError::InternalError);
        }
        if accounts.as_ref().unwrap().len() == 0 {
          tracing::info!(
            "No accounts found with filter: {} : {:?}",
            filter,
            query_result_string
          );
          return Ok(vec![]);
        }

        let account = accounts.unwrap();

        Ok(account)
      }
      Err(_) => Err(DataAccessError::InternalError),
    }
  }

  pub async fn get_many_compact_accounts_by_ids(
    &self,
    ids: Vec<u32>,
  ) -> Result<Vec<CompactAccount>, DataAccessError> {
    self
      .get_many_compact_accounts_by_filter(
        &format!(
          "array::any([{}])",
          ids
            .iter()
            .map(|id| format!("id.id={}", id))
            .collect::<Vec<String>>()
            .join(", "),
        ),
        100,
        0,
      )
      .await
  }

  pub async fn get_one_account_by_id(&self, id: u32) -> Result<Account, DataAccessError> {
    let query = format!(
      r#"
      SELECT *, id.id as id FROM account:{{ id: {} }}
      "#,
      id
    );

    let query_result = self.db.query(&query).await;

    match query_result {
      Ok(mut query_result) => {
        let account: Result<Option<Account>, _> = query_result.take(0);
        if account.as_ref().is_err() {
          tracing::error!("Error while getting one account by id: {:?}", query_result);
          return Err(DataAccessError::InternalError);
        }
        if account.as_ref().unwrap().is_none() {
          // @TODO-ZM: stringify query_result before calling .take
          tracing::info!("No account found with id: {} : {:?}", id, query_result);
          return Err(DataAccessError::NotFound);
        }

        let account = account.unwrap().unwrap();

        Ok(account)
      }
      Err(_) => Err(DataAccessError::InternalError),
    }
  }

  pub async fn create_one_account(&self, account: DBAccount) -> Result<u32, DataAccessError> {
    let query = format!(
      r#"
      BEGIN TRANSACTION;

      LET $count = (SELECT count() FROM account GROUP BY count)[0].count || 0;

      CREATE account:{{ id: $count }} CONTENT {{
        email: '{}',
        slug: '{}',
        type: '{}',
        {}
      }};

      COMMIT TRANSACTION;
      "#,
      escape_single_quote(&account.email),
      escape_single_quote(&account.slug),
      escape_single_quote(&account.r#type.to_string()),
      match account.r#type {
        AccountType::Company { company_name } => format!("company_name: '{}'", company_name),
        AccountType::Admin {
          first_name,
          last_name,
        }
        | AccountType::Individual {
          first_name,
          last_name,
        } => {
          format!(
            "first_name: '{}', last_name: '{}'",
            escape_single_quote(&first_name),
            escape_single_quote(&last_name)
          )
        }
      },
    );

    let query_result = self.db.query(&query).await;
    match query_result {
      Ok(mut query_result) => {
        let record: Result<Option<DBRecord>, _> = query_result.take(1);

        match record {
          Ok(record) => match record {
            Some(record) => {
              let id = db_thing_to_id(&record.id);
              match id {
                Some(id) => return Ok(id),
                None => {
                  tracing::error!("failed to get created account id {:?}", record);
                  return Err(DataAccessError::InternalError);
                }
              }
            }
            None => {
              tracing::error!("failed to get created account record {:?}", record);
              return Err(DataAccessError::InternalError);
            }
          },
          Err(e) => {
            tracing::error!("failed to get created account record {:?}", e);
            return Err(DataAccessError::InternalError);
          }
        }
      }
      Err(e) => {
        tracing::error!("failed to create account {:?}, query {:?}", e, &query);
        return Err(DataAccessError::CreationError);
      }
    }
  }
}
