use hyper::{Body, Request};
use sentry::Hub;
use sentry_tower::{NewFromTopProvider, NewSentryLayer, SentryHttpLayer, SentryLayer};
use std::sync::Arc;
use tower::{
  layer::util::{Identity, Stack},
  ServiceBuilder,
};
use tower_http::{
  classify::{ServerErrorsAsFailures, SharedClassifier},
  trace::TraceLayer,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::config::service::ConfigService;

const SENTRY_ON_DEVELOPMENT: bool = false;

pub fn enable_tracing() -> sentry::ClientInitGuard {
  let config_service = ConfigService::new();
  let config = config_service.get_config();

  let guard = sentry::init((
    match (&config.stage, SENTRY_ON_DEVELOPMENT) {
      (crate::config::service::Stage::Development, false) => "",
      _ => "https://958ac72477eff5ba3771ca0bfdc6108b@o4505697083457536.ingest.sentry.io/4505697771651072",
    },
    sentry::ClientOptions {
      release: sentry::release_name!(),
      environment: Some(config.stage.as_str().into()),
      traces_sample_rate: match config.stage {
        crate::config::service::Stage::Development => 1.0,
        // @TODO-ZM: reduce once we have actual users
        _ => 1.0,
      },
      ..Default::default()
    },
  ));

  tracing_subscriber::registry()
    .with(
      // @TODO-ZM: set debug level in .env
      tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "debug=info,tower_http=debug,axum::rejection=trace".into()),
    )
    .with(sentry_tracing::layer())
    .with(tracing_subscriber::fmt::layer())
    .init();
  tracing::info!("enabled tracing for stage: {}", config.stage.as_str());

  guard
}

pub fn create_trace_layer() -> ServiceBuilder<
  Stack<
    SentryLayer<NewFromTopProvider, Arc<Hub>, Request<Body>>,
    Stack<SentryHttpLayer, Stack<TraceLayer<SharedClassifier<ServerErrorsAsFailures>>, Identity>>,
  >,
> {
  let layer = ServiceBuilder::new()
    .layer(TraceLayer::new_for_http())
    // continue trace from incoming request
    .layer(SentryHttpLayer::with_transaction())
    // bind a new hub for each request
    .layer(NewSentryLayer::new_from_top());

  tracing::info!("created trace layer");
  layer
}
