use crate::{
  _entry::state::AppState,
  _utils::{database::DBOrderDirection, error::BootError},
  task::model::{DBTask, TaskName, TaskStatus, TaskType},
};
use std::{
  sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
  },
  time::Duration,
};
use tokio_cron_scheduler::Job;

pub struct SearchCronJob {
  pub app_state: AppState,
}

async fn run_indexing_cron_job(app_state: AppState) {
  let tasks = app_state
    .task_repository
    .get_many_pending_indexing_tasks("Indexing", "created_at", DBOrderDirection::DESC, 10, 0)
    .await;

  if tasks.is_err() {
    tracing::error!("Error while getting indexing tasks");
    return;
  }
  let tasks = tasks.unwrap();

  if tasks.is_empty() {
    return;
  }

  tracing::info!("Found {} indexing tasks", tasks.len());

  let mut task_ids: Vec<u32> = [].to_vec();
  let mut post_ids = vec![];
  for task in tasks {
    task_ids.push(task.id);
    if let TaskName::Indexing {
      model_name,
      model_id,
    } = task.name
    {
      if model_name == "post" {
        post_ids.push(model_id);
      }
    }
  }

  tracing::info!("indexing {} posts", post_ids.len());

  let posts = app_state
    .post_repository
    .get_many_posts_by_ids(post_ids)
    .await;
  if posts.is_err() {
    tracing::error!("Error while getting posts");
    return;
  }
  let posts = posts.unwrap();

  let tag_ids = posts
    .iter()
    .flat_map(|post| post.tag_ids.clone())
    .collect::<Vec<u32>>();

  let tags = app_state
    .tag_repository
    .get_many_compact_tags_by_ids(&tag_ids)
    .await;
  if tags.is_err() {
    tracing::error!("Error while getting tags");
    return;
  }
  let tags = tags.unwrap();

  let account_ids = posts
    .iter()
    .map(|post| post.poster_id)
    .collect::<Vec<u32>>();

  let accounts = app_state
    .account_repository
    .get_many_compact_accounts_by_ids(account_ids)
    .await;
  if accounts.is_err() {
    tracing::error!("Error while getting accounts");
    return;
  }
  let accounts = accounts.unwrap();

  let indexing_result = app_state
    .search_service
    .index_posts(posts, tags, accounts)
    .await;
  if indexing_result.is_err() {
    tracing::error!(
      "Error while indexing posts {:?}",
      indexing_result.err().unwrap()
    );
    return;
  }

  let task_status_update_result = app_state
    .task_repository
    .complete_many_tasks_by_ids(task_ids)
    .await;

  if task_status_update_result.is_err() {
    tracing::error!("Error while updating indexing tasks");
    return;
  }

  let more_tasks = app_state
    .task_repository
    .get_many_pending_indexing_tasks("Indexing", "created_at", DBOrderDirection::DESC, 1, 0)
    .await;

  if more_tasks.is_err() {
    tracing::error!("Error while getting more indexing tasks");
    return;
  }
  let more_tasks = more_tasks.unwrap();

  if more_tasks.is_empty() {
    tracing::info!("No more indexing tasks found, creating bk-tree refreshing task");

    let task_id = app_state
      .task_repository
      .create_one_task(DBTask {
        name: TaskName::RefreshingBKTree,
        status: TaskStatus::Pending,
        r#type: TaskType::Automated,
      })
      .await;

    if task_id.is_err() {
      tracing::error!("Error while creating bk-tree refreshing task");
    }
  }

  tracing::info!("Indexing done");
}

async fn run_bk_tree_refresher_cron_job(app_state: AppState, has_job_ran_once: bool) {
  let tasks = app_state
    .task_repository
    .get_many_pending_indexing_tasks(
      "RefreshingBKTree",
      "created_at",
      DBOrderDirection::DESC,
      1_000,
      0,
    )
    .await;

  if tasks.is_err() {
    tracing::error!("Error while getting bk-tree refreshing tasks");
    return;
  }
  let tasks = tasks.unwrap();

  if tasks.is_empty() && has_job_ran_once {
    tracing::info!("⏭️  No bk-tree refreshing tasks found, skipping");
    return;
  }

  tracing::info!("Found {} bk-tree refreshing tasks", tasks.len());

  let mut task_ids: Vec<u32> = [].to_vec();
  for task in tasks {
    task_ids.push(task.id);
  }

  tracing::info!("Refreshing bk-tree");

  let bk_tree_refreshing_result = app_state.search_service.refresh_bk_tree().await;
  if bk_tree_refreshing_result.is_err() {
    tracing::error!(
      "Error while indexing posts {:?}",
      bk_tree_refreshing_result.err().unwrap()
    );
    return;
  }

  let task_status_update_result = app_state
    .task_repository
    .complete_many_tasks_by_ids(task_ids)
    .await;

  if task_status_update_result.is_err() {
    tracing::error!("Error while updating indexing tasks");
    return;
  }

  tracing::info!("Refreshing bk-tree done");
}

impl SearchCronJob {
  pub fn create_indexing_cron_job(&self) -> Result<Job, BootError> {
    let app_state = self.app_state.clone();
    let is_job_running = Arc::new(AtomicBool::new(false));

    let job = Job::new_repeated_async(Duration::from_secs(5), move |_, __| {
      let app_state = app_state.clone();
      let is_job_running = is_job_running.clone();

      Box::pin(async move {
        let compare_and_swap_result =
          is_job_running.compare_exchange(false, true, Ordering::Relaxed, Ordering::Relaxed);
        if compare_and_swap_result.is_ok() && !compare_and_swap_result.unwrap() {
          run_indexing_cron_job(app_state.clone()).await;
          is_job_running.store(false, Ordering::Relaxed);
        } else {
          tracing::info!("Still indexing... ");
        }
      })
    });

    if job.is_err() {
      tracing::error!("Error while creating search cron job");
      return Err(BootError::CronJobSetupError);
    }
    let job = job.unwrap();

    Ok(job)
  }

  pub fn create_bk_tree_refresher_cron_job(&self) -> Result<Job, BootError> {
    let app_state = self.app_state.clone();
    let is_job_running = Arc::new(AtomicBool::new(false));
    let has_job_ran_once = Arc::new(AtomicBool::new(false));

    let job = Job::new_repeated_async(Duration::from_secs(5), move |_, __| {
      let app_state = app_state.clone();
      let is_job_running = is_job_running.clone();
      let has_job_ran_once = has_job_ran_once.clone();

      Box::pin(async move {
        let compare_and_swap_result =
          is_job_running.compare_exchange(false, true, Ordering::Relaxed, Ordering::Relaxed);
        if compare_and_swap_result.is_ok() && !compare_and_swap_result.unwrap() {
          run_bk_tree_refresher_cron_job(
            app_state.clone(),
            has_job_ran_once.load(Ordering::Relaxed),
          )
          .await;
          has_job_ran_once.store(true, Ordering::Relaxed);
          is_job_running.store(false, Ordering::Relaxed);
        } else {
          tracing::info!("Still refreshing bk-tree ... ");
        }
      })
    });

    if job.is_err() {
      tracing::error!("Error while creating bk-tree refresher cron job");
      return Err(BootError::CronJobSetupError);
    }
    let job = job.unwrap();

    Ok(job)
  }
}
