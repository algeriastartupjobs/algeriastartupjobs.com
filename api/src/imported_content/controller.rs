use super::model::{ImportedContentStatus, JobJsonData};
use crate::{_entry::state::AppState, _utils::query::ImportedContentStatusQuery};
use async_stream::try_stream;
use axum::{
  extract::{Query, State},
  response::sse::{Event, KeepAlive, Sse},
  Router,
};
use futures_util::stream::Stream;
use serde_json::json;
use std::convert::Infallible;

pub async fn imported_content_status(
  State(app_state): State<AppState>,
  url_query: Query<ImportedContentStatusQuery>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
  // @TODO-ZM: add a timeout
  Sse::new(try_stream! {
    let mut last_known_status = None;
    loop {
      let import_status = app_state
        .imported_content_service
        .import_job_post_and_get_status(
          &url_query.url,
          last_known_status.clone(),
          &vec![
            ImportedContentStatus::Failed {
              failure_reason: "".to_string(),
            },
            ImportedContentStatus::Completed,
          ],
        )
        .await;

      if import_status.is_err() {
        // @TODO-ZM: log error reason
        yield Event::default().data(format!(r#"{{"status": "{}"}}"#, ImportedContentStatus::Failed{failure_reason:"".to_string()}));
        break;
      }
      let (is_final_status, status, json_data) = import_status.unwrap();

      let job_json_data = serde_json::from_str::<JobJsonData>(&json_data).unwrap_or(JobJsonData {
        title: "".to_string(),
        description: "".to_string(),
        poster: "".to_string(),
      });

      yield Event::default().data(json!({
        "status": status.to_string(),
        "title": job_json_data.title,
        "description": job_json_data.description,
        "poster": job_json_data.poster,
      }).to_string());


      if is_final_status {
        break;
      }
      last_known_status = Some(status);
    }

  })
  .keep_alive(KeepAlive::default())
}

pub fn create_imported_content_router() -> Router<AppState> {
  Router::new().route("/status", axum::routing::get(imported_content_status))
}
