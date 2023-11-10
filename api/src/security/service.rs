use std::sync::Arc;

use crate::_utils::error::SecurityError;

#[derive(Debug)]
pub struct RateLimitConstraint {
  pub id: String,
  pub max_requests: u32,
  pub duration_ms: u32,
}

pub struct SecurityService {
  rate_limit_kv_db: Arc<sled::Db>,
}

impl SecurityService {
  pub fn new(rate_limit_kv_db: Arc<sled::Db>) -> Self {
    Self { rate_limit_kv_db }
  }

  pub fn rate_limit(&self, constraints: Vec<RateLimitConstraint>) -> Result<(), SecurityError> {
    for constraint in constraints {
      let id = &constraint.id;
      let duration_ms = constraint.duration_ms;
      let max_requests = constraint.max_requests;
      let is_allowed = self.rate_limit_kv_db.transaction::<_, _, bool>(|tx| {
        let value = tx.get(id);
        if value.is_err() {
          // @TODO-ZM: log error
          return Ok(false);
        }
        let value = value.unwrap();
        if value.is_none() {
          let inserted = tx.insert(
            id.as_bytes().to_vec(),
            format!("{}|{}", chrono::Utc::now().to_rfc3339(), 1).as_bytes(),
          );
          if inserted.is_err() {
            // @TODO-ZM: log error
            return Ok(false);
          } else {
            return Ok(true);
          }
        }
        let value = value.unwrap();
        let value_str = String::from_utf8_lossy(&value);
        // value is "[ISO 8601 time string]|[count]"
        let (last_time_str, count_str) = value_str.split_once('|').unwrap();
        let last_time = chrono::DateTime::parse_from_rfc3339(last_time_str);

        if last_time.is_err() {
          // @TODO-ZM: log error
          return Ok(false);
        }
        let last_time = last_time.unwrap();
        let duration_since_last_time = chrono::Utc::now().signed_duration_since(last_time);
        let constrained_duration = chrono::Duration::milliseconds(duration_ms as i64);
        if duration_since_last_time > constrained_duration {
          let inserted = tx.insert(
            id.as_bytes().to_vec(),
            format!("{}|{}", chrono::Utc::now().to_rfc3339(), 1).as_bytes(),
          );
          if inserted.is_err() {
            // @TODO-ZM: log error
            Ok(false)
          } else {
            Ok(true)
          }
        } else {
          let count = count_str.parse::<u32>();
          if count.is_err() {
            // @TODO-ZM: log error
            return Ok(false);
          }
          let count = count.unwrap();
          if count >= max_requests {
            Ok(false)
          } else {
            let inserted = tx.insert(
              id.as_bytes().to_vec(),
              format!("{}|{}", chrono::Utc::now().to_rfc3339(), count + 1).as_bytes(),
            );
            if inserted.is_err() {
              // @TODO-ZM: log error
              Ok(false)
            } else {
              Ok(true)
            }
          }
        }
      });

      if is_allowed.is_err() {
        return Err(SecurityError::InternalError);
      }
      let is_allowed = is_allowed.unwrap();

      if !is_allowed {
        return Err(SecurityError::RateLimitError);
      }
    }

    Ok(())
  }
}
