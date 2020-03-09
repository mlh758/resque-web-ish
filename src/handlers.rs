use crate::resque;
use actix_files as fs;
use actix_web::http::StatusCode;
use actix_web::{delete, error, get, post, web, HttpRequest, HttpResponse};
use plugin_manager::Action;
use serde_derive::{Deserialize, Serialize};
use std::path::Path;
use r2d2_redis::{RedisConnectionManager, r2d2};
use r2d2::Pool;
use std::ops::DerefMut;

type RedisPool = Pool<RedisConnectionManager>;

pub struct AppState {
  pub pool: RedisPool,
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
  pool: &RedisPool,
) -> Result<r2d2::PooledConnection<RedisConnectionManager>, error::InternalError<r2d2::Error>> {
  pool.get().map_err(resque_error_map)
}

fn resque_error_map<T>(err: T) -> error::InternalError<T> {
  error::InternalError::new(err, StatusCode::INTERNAL_SERVER_ERROR)
}

#[get("/stats")]
async fn resque_stats(state: web::Data<AppState>) -> actix_web::Result<HttpResponse> {
  let mut con = get_redis_connection(&state.pool)?;
  let response = ResqueStats {
    available_queues: resque::get_queues(con.deref_mut())
      .map_err(resque_error_map)?
      .into_iter()
      .collect(),
    success_count: resque::processed_count(con.deref_mut()),
    failure_count: resque::failure_count(con.deref_mut()),
  };
  Ok(HttpResponse::Ok().json(response))
}

#[get("/queue/{name}")]
async fn queue_details(
  query: web::Query<JobParam>,
  path: web::Path<(String,)>,
  state: web::Data<AppState>,
) -> actix_web::Result<HttpResponse> {
  let mut con = get_redis_connection(&state.pool)?;
  let start_at = query.from_job.unwrap_or(0);
  let results =
    resque::queue_details(con.deref_mut(), &path.0, start_at, start_at + 9).map_err(resque_error_map)?;
  Ok(HttpResponse::Ok().json(results))
}

#[get("/failed")]
async fn failed_jobs(
  query: web::Query<JobParam>,
  state: web::Data<AppState>,
) -> actix_web::Result<HttpResponse> {
  let mut con = get_redis_connection(&state.pool)?;
  let start_at = query.from_job.unwrap_or(0);
  let response = FailedJobs {
    jobs: resque::get_failed(con.deref_mut(), start_at, start_at + 9)
      .map_err(resque_error_map)?
      .into_iter()
      .map(
        |s| match serde_json::from_str(&s).map_err(resque_error_map) {
          Ok(v) => v,
          Err(_) => serde_json::json!({"error": "failed to parse job"}),
        },
      )
      .collect(),
    total_failed: resque::current_failures(con.deref_mut()).map_err(resque_error_map)?,
  };
  Ok(HttpResponse::Ok().json(response))
}

#[get("/active_workers")]
async fn active_workers(state: web::Data<AppState>) -> actix_web::Result<HttpResponse> {
  let mut con = get_redis_connection(&state.pool)?;
  let workers = ResqueWorkers {
    data: resque::active_workers(con.deref_mut()).map_err(resque_error_map)?,
  };
  Ok(HttpResponse::Ok().json(workers))
}

#[delete("/failed")]
async fn delete_failed_jobs(state: web::Data<AppState>) -> actix_web::Result<HttpResponse> {
  let mut con = get_redis_connection(&state.pool)?;
  let deleted = resque::clear_queue(con.deref_mut(), "failed").map_err(resque_error_map)?;
  Ok(HttpResponse::Ok().body(deleted.to_string()))
}

#[post("/retry_job")]
async fn retry_failed_job(
  job: web::Json<DeleteFailedParam>,
  state: web::Data<AppState>,
) -> actix_web::Result<HttpResponse> {
  let mut con = get_redis_connection(&state.pool)?;
  resque::retry_failed_job(con.deref_mut(), &job.id).map_err(resque_error_map)?;
  Ok(HttpResponse::Ok().body("job retried"))
}

#[post("/retry_all")]
async fn retry_all(state: web::Data<AppState>) -> actix_web::Result<HttpResponse> {
  let mut con = get_redis_connection(&state.pool)?;
  resque::retry_all_jobs(con.deref_mut()).map_err(resque_error_map)?;
  state.plugins.post_action(Action::RetryAll);
  Ok(HttpResponse::Ok().body("all jobs retried"))
}

#[delete("/failed_job")]
async fn delete_failed_job(
  job: web::Json<DeleteFailedParam>,
  state: web::Data<AppState>,
) -> actix_web::Result<HttpResponse> {
  let mut con = get_redis_connection(&state.pool)?;
  resque::delete_failed_job(con.deref_mut(), &job.id).map_err(resque_error_map)?;
  Ok(HttpResponse::Ok().body("job removed"))
}

#[delete("/queue/{name}")]
async fn delete_queue_contents(
  path: web::Path<(String,)>,
  state: web::Data<AppState>,
) -> actix_web::Result<HttpResponse> {
  let mut con = get_redis_connection(&state.pool)?;
  let queue_key = format!("queue:{}", path.0);
  let deleted = resque::clear_queue(con.deref_mut(), &queue_key).map_err(resque_error_map)?;
  state
    .plugins
    .post_action(Action::DeleteQueue(path.0.clone()));
  Ok(HttpResponse::Ok().body(deleted.to_string()))
}

#[delete("/worker/{id}")]
async fn delete_worker(
  path: web::Path<(String,)>,
  state: web::Data<AppState>,
) -> actix_web::Result<HttpResponse> {
  let mut con = get_redis_connection(&state.pool)?;
  resque::remove_worker(con.deref_mut(), &path.0).map_err(resque_error_map)?;
  Ok(HttpResponse::Ok().body("worker removed"))
}

pub async fn static_assets(req: HttpRequest) -> actix_web::Result<fs::NamedFile> {
  let path: std::path::PathBuf = req
    .match_info()
    .query("filename")
    .parse()
    .map_err(resque_error_map)?;
  let root = Path::new("./public");
  let file = fs::NamedFile::open(root.join(path)).or_else(|_e| fs::NamedFile::open("./public/index.html"))?;
  Ok(file.use_last_modified(true))
}

pub async fn home(_req: HttpRequest) -> actix_web::Result<fs::NamedFile> {
  let file = fs::NamedFile::open("./public/index.html")?;
  Ok(file.use_last_modified(true))
}
