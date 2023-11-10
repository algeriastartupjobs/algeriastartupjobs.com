use std::sync::Arc;

use crate::{
  _utils::{
    error::EmailError,
    string::{escape_double_quote, escape_new_line, escape_new_line_with_br},
  },
  config::service::{ConfigService, Stage},
};

pub struct EmailService {
  config_service: Arc<ConfigService>,
}

impl EmailService {
  pub fn new(config_service: Arc<ConfigService>) -> Self {
    Self { config_service }
  }

  // @TODO-ZM: gmail groups emails by body, so we need to add a random string at the bottom of the body
  pub async fn send_one_email(
    &self,
    email: &String,
    subject: &String,
    body: &String,
  ) -> Result<(), EmailError> {
    match self.config_service.get_config().stage {
      Stage::Development => {
        tracing::info!(
          "\n\nEmail: {}\nSubject: {}\nBody:\n{}\n\n",
          email,
          subject,
          body
        );
        return Ok(());
      }
      _ => {}
    }

    let client = reqwest::Client::new();

    let res = client
      .post("https://api.zeptomail.com/v1.1/email")
      .header("content-type", "application/json")
      .header("accept", "application/json")
      .header(
        "Authorization",
        self.config_service.get_config().email_service_auth_token,
      )
      .body(format!(
        r#"{{
          "bounce_address": "bounce@mail.algeriastartupjobs.com",
          "from": {{ "address": "noreply@algeriastartupjobs.com", "name": "Algeria Startup Jobs" }},
          "to": [{{ "email_address": {{ "address": "{}" }} }}],
          "subject": "{}",
          "htmlbody": "{}"
        }}"#,
        escape_new_line(&escape_double_quote(email)),
        escape_new_line(&escape_double_quote(subject)),
        escape_new_line_with_br(&escape_double_quote(body))
      ))
      .send()
      .await;

    if res.is_err() {
      tracing::error!("Failed to send email: {}", res.err().unwrap());
      return Err(EmailError::InternalError);
    }
    let res = res.unwrap();

    if !res.status().is_success() {
      tracing::error!(
        "Failed to send email: {} body: {}",
        res.status(),
        res.text().await.unwrap()
      );
      return Err(EmailError::InternalError);
    }

    Ok(())
  }
}
