use crate::{
  _entry::state::AppState,
  _utils::{database::DBOrderDirection, error::BootError},
  imported_content::model::ImportedContentStatus,
};
use std::{
  sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
  },
  time::Duration,
};
use tokio_cron_scheduler::Job;

pub struct ImportedContentCronJob {
  pub app_state: AppState,
}

async fn run_importing_content_cron_job(app_state: AppState) {
  tracing::info!("🚀 Importing");

  let imported_contents = app_state
    .imported_content_repository
    .get_many_pending_imported_content("created_at", DBOrderDirection::DESC, 10, 0)
    .await;

  if imported_contents.is_err() {
    tracing::error!("Error while getting imported_contents");
    return;
  }

  let imported_contents = imported_contents.unwrap();

  if imported_contents.is_empty() {
    tracing::info!("No importing_contents found");
    return;
  }

  tracing::info!("Found {} importing_contents", imported_contents.len());

  let imported_content_ids: Vec<u32> = imported_contents
    .iter()
    .map(|imported_content| imported_content.id)
    .collect();

  let imported_content_status_update_result = app_state
    .imported_content_repository
    .update_status_of_many_imported_contents_by_ids(
      imported_content_ids,
      ImportedContentStatus::InProgress,
    )
    .await;

  if imported_content_status_update_result.is_err() {
    tracing::error!("Error while updating imported_contents status");
    return;
  }

  for imported_content in imported_contents {
    let id = imported_content.id;
    let job_json_data = app_state
      .imported_content_service
      .fetch_job_post_from_url(&imported_content.source_url)
      .await;
    if job_json_data.is_err() {
      // @TODO-ZM: log error
      let imported_content_update_result = app_state
        .imported_content_repository
        .update_status_of_many_imported_contents_by_ids(
          vec![id],
          ImportedContentStatus::Failed {
            failure_reason: "Error while fetching job post".to_string(),
          },
        )
        .await;
      if imported_content_update_result.is_err() {
        tracing::error!("Error while updating imported_content");
        return;
      }
      break;
    }
    let job_json_data = job_json_data.unwrap();

    let imported_content_update_result = app_state
      .imported_content_repository
      .complete_one_imported_content_by_id(
        imported_content.id,
        serde_json::to_string(&job_json_data).unwrap_or("".to_string()),
      )
      .await;

    if imported_content_update_result.is_err() {
      tracing::error!("Error while updating imported_content");
      return;
    }
  }

  tracing::info!("✅ Importing done");
}

impl ImportedContentCronJob {
  pub fn create_importing_content_cron_job(&self) -> Result<Job, BootError> {
    let app_state = self.app_state.clone();
    let is_job_running = Arc::new(AtomicBool::new(false));

    let job = Job::new_repeated_async(Duration::from_secs(1), move |_, __| {
      let app_state = app_state.clone();
      let is_job_running = is_job_running.clone();

      Box::pin(async move {
        let compare_and_swap_result =
          is_job_running.compare_exchange(false, true, Ordering::Relaxed, Ordering::Relaxed);
        if compare_and_swap_result.is_ok() && !compare_and_swap_result.unwrap() {
          run_importing_content_cron_job(app_state.clone()).await;
          is_job_running.store(false, Ordering::Relaxed);
        } else {
          tracing::info!("⏳ Still importing_content... ");
        }
      })
    });

    if job.is_err() {
      tracing::error!("Error while creating imported content cron job");
      return Err(BootError::CronJobSetupError);
    }
    let job = job.unwrap();

    Ok(job)
  }
}
