use crate::resque;
use actix_files as fs;
use actix_web::http::StatusCode;
use actix_web::{delete, error, get, post, web, HttpRequest, HttpResponse};
use plugin_manager::Action;
use serde_derive::{Deserialize, Serialize};
use std::path::Path;

pub struct AppState {
  pub client: redis::Client,
  pub plugins: plugin_manager::PluginManager,
}

#[derive(Serialize)]
struct ResqueStats {
  success_count: u64,
  failure_count: u64,
  available_queues: Vec<String>,
}

#[derive(Serialize)]
struct FailedJobs {
  jobs: serde_json::Value,
  total_failed: u64,
}

#[derive(Deserialize)]
struct JobParam {
  from_job: Option<isize>,
}

#[derive(Serialize)]
struct ResqueWorkers {
  data: Vec<resque::Worker>,
}

#[derive(Deserialize)]
struct DeleteFailedParam {
  id: String,
}

fn get_redis_connection(
  client: &redis::Client,
) -> Result<redis::Connection, error::InternalError<redis::RedisError>> {
  client.get_connection().map_err(resque_error_map)
}

fn resque_error_map<T>(err: T) -> error::InternalError<T> {
  error::InternalError::new(err, StatusCode::INTERNAL_SERVER_ERROR)
}

#[get("/stats")]
fn resque_stats(state: web::Data<AppState>) -> actix_web::Result<HttpResponse> {
  let mut con = get_redis_connection(&state.client)?;
  let response = ResqueStats {
    available_queues: resque::get_queues(&mut con)
      .map_err(resque_error_map)?
      .into_iter()
      .collect(),
    success_count: resque::processed_count(&mut con),
    failure_count: resque::failure_count(&mut con),
  };
  Ok(HttpResponse::Ok().json(response))
}

#[get("/queue/{name}")]
fn queue_details(
  query: web::Query<JobParam>,
  path: web::Path<(String,)>,
  state: web::Data<AppState>,
) -> actix_web::Result<HttpResponse> {
  let mut con = get_redis_connection(&state.client)?;
  let start_at = query.from_job.unwrap_or(0);
  let results =
    resque::queue_details(&mut con, &path.0, start_at, start_at + 9).map_err(resque_error_map)?;
  Ok(HttpResponse::Ok().json(results))
}

#[get("/failed")]
fn failed_jobs(
  query: web::Query<JobParam>,
  state: web::Data<AppState>,
) -> actix_web::Result<HttpResponse> {
  let mut con = get_redis_connection(&state.client)?;
  let start_at = query.from_job.unwrap_or(0);
  let response = FailedJobs {
    jobs: resque::get_failed(&mut con, start_at, start_at + 9)
      .map_err(resque_error_map)?
      .into_iter()
      .map(
        |s| match serde_json::from_str(&s).map_err(resque_error_map) {
          Ok(v) => v,
          Err(_) => serde_json::json!({"error": "failed to parse job"}),
        },
      )
      .collect(),
    total_failed: resque::current_failures(&mut con).map_err(resque_error_map)?,
  };
  Ok(HttpResponse::Ok().json(response))
}

#[get("/active_workers")]
fn active_workers(state: web::Data<AppState>) -> actix_web::Result<HttpResponse> {
  let mut con = get_redis_connection(&state.client)?;
  let workers = ResqueWorkers {
    data: resque::active_workers(&mut con).map_err(resque_error_map)?,
  };
  Ok(HttpResponse::Ok().json(workers))
}

#[delete("/failed")]
fn delete_failed_jobs(state: web::Data<AppState>) -> actix_web::Result<HttpResponse> {
  let mut con = get_redis_connection(&state.client)?;
  let deleted = resque::clear_queue(&mut con, "failed").map_err(resque_error_map)?;
  Ok(HttpResponse::Ok().body(deleted.to_string()))
}

#[post("/retry_job")]
fn retry_failed_job(
  job: web::Json<DeleteFailedParam>,
  state: web::Data<AppState>,
) -> actix_web::Result<HttpResponse> {
  let mut con = get_redis_connection(&state.client)?;
  resque::retry_failed_job(&mut con, &job.id).map_err(resque_error_map)?;
  Ok(HttpResponse::Ok().body("job retried"))
}

#[post("/retry_all")]
fn retry_all(state: web::Data<AppState>) -> actix_web::Result<HttpResponse> {
  let mut con = get_redis_connection(&state.client)?;
  resque::retry_all_jobs(&mut con).map_err(resque_error_map)?;
  state.plugins.post_action(Action::RetryAll);
  Ok(HttpResponse::Ok().body("all jobs retried"))
}

#[delete("/failed_job")]
fn delete_failed_job(
  job: web::Json<DeleteFailedParam>,
  state: web::Data<AppState>,
) -> actix_web::Result<HttpResponse> {
  let mut con = get_redis_connection(&state.client)?;
  resque::delete_failed_job(&mut con, &job.id).map_err(resque_error_map)?;
  Ok(HttpResponse::Ok().body("job removed"))
}

#[delete("/queue/{name}")]
fn delete_queue_contents(
  path: web::Path<(String,)>,
  state: web::Data<AppState>,
) -> actix_web::Result<HttpResponse> {
  let mut con = get_redis_connection(&state.client)?;
  let queueKey = format!("queue:{}", path.0);
  let deleted = resque::clear_queue(&mut con, &queueKey).map_err(resque_error_map)?;
  state
    .plugins
    .post_action(Action::DeleteQueue(path.0.clone()));
  Ok(HttpResponse::Ok().body(deleted.to_string()))
}

pub fn static_assets(req: HttpRequest) -> actix_web::Result<fs::NamedFile> {
  let path: std::path::PathBuf = req
    .match_info()
    .query("filename")
    .parse()
    .map_err(resque_error_map)?;
  let root = Path::new("./public");
  let file = fs::NamedFile::open(root.join(path))?;
  Ok(file.use_last_modified(true))
}

#[get("/")]
fn home() -> actix_web::Result<fs::NamedFile> {
  let file = fs::NamedFile::open("./public/index.html")?;
  Ok(file.use_last_modified(true))
}
