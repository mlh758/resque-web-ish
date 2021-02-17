use crate::resque;
use actix_files as fs;
use actix_web::http::StatusCode;
use actix_web::{delete, error, get, post, web, HttpRequest, HttpResponse};
use plugin_manager::Action;
use redis::aio::ConnectionManager;
use serde_derive::{Deserialize, Serialize};
use std::path::Path;

pub struct AppState {
    pub redis: ConnectionManager,
    pub plugins: plugin_manager::PluginManager,
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

fn resque_error_map<T>(err: T) -> error::InternalError<T> {
    error::InternalError::new(err, StatusCode::INTERNAL_SERVER_ERROR)
}

#[get("/stats")]
async fn resque_stats(state: web::Data<AppState>) -> actix_web::Result<HttpResponse> {
    let response = resque::queue_stats(state.redis.clone())
        .await
        .map_err(resque_error_map)?;
    Ok(HttpResponse::Ok().json(response))
}

#[get("/queue/{name}")]
async fn queue_details(
    query: web::Query<JobParam>,
    path: web::Path<(String,)>,
    state: web::Data<AppState>,
) -> actix_web::Result<HttpResponse> {
    let start_at = query.from_job.unwrap_or(0);
    let results = resque::queue_details(state.redis.clone(), &path.0, start_at, start_at + 9)
        .await
        .map_err(resque_error_map)?;
    Ok(HttpResponse::Ok().json(results))
}

#[get("/failed")]
async fn failed_jobs(
    query: web::Query<JobParam>,
    state: web::Data<AppState>,
) -> actix_web::Result<HttpResponse> {
    let start_at = query.from_job.unwrap_or(0);
    let response = FailedJobs {
        jobs: resque::get_failed(state.redis.clone(), start_at, start_at + 9)
            .await
            .map_err(resque_error_map)?
            .into_iter()
            .map(
                |s| match serde_json::from_str(&s).map_err(resque_error_map) {
                    Ok(v) => v,
                    Err(_) => serde_json::json!({"error": "failed to parse job"}),
                },
            )
            .collect(),
        total_failed: resque::current_failures(state.redis.clone())
            .await
            .map_err(resque_error_map)?,
    };
    Ok(HttpResponse::Ok().json(response))
}

#[get("/active_workers")]
async fn active_workers(state: web::Data<AppState>) -> actix_web::Result<HttpResponse> {
    let workers = ResqueWorkers {
        data: resque::active_workers(state.redis.clone())
            .await
            .map_err(resque_error_map)?,
    };
    Ok(HttpResponse::Ok().json(workers))
}

#[delete("/failed")]
async fn delete_failed_jobs(state: web::Data<AppState>) -> actix_web::Result<HttpResponse> {
    let deleted = resque::clear_queue(state.redis.clone(), "failed")
        .await
        .map_err(resque_error_map)?;
    Ok(HttpResponse::Ok().body(deleted.to_string()))
}

#[post("/retry_job")]
async fn retry_failed_job(
    job: web::Json<DeleteFailedParam>,
    state: web::Data<AppState>,
) -> actix_web::Result<HttpResponse> {
    resque::retry_failed_job(state.redis.clone(), &job.id)
        .await
        .map_err(resque_error_map)?;
    Ok(HttpResponse::Ok().body("job retried"))
}

#[post("/retry_all")]
async fn retry_all(state: web::Data<AppState>) -> actix_web::Result<HttpResponse> {
    resque::retry_all_jobs(state.redis.clone())
        .await
        .map_err(resque_error_map)?;
    state.plugins.post_action(Action::RetryAll);
    Ok(HttpResponse::Ok().body("all jobs retried"))
}

#[delete("/failed_job")]
async fn delete_failed_job(
    job: web::Json<DeleteFailedParam>,
    state: web::Data<AppState>,
) -> actix_web::Result<HttpResponse> {
    resque::delete_failed_job(state.redis.clone(), &job.id)
        .await
        .map_err(resque_error_map)?;
    Ok(HttpResponse::Ok().body("job removed"))
}

#[delete("/queue/{name}")]
async fn delete_queue_contents(
    path: web::Path<(String,)>,
    state: web::Data<AppState>,
) -> actix_web::Result<HttpResponse> {
    let queue_key = format!("queue:{}", path.0);
    let deleted = resque::clear_queue(state.redis.clone(), &queue_key)
        .await
        .map_err(resque_error_map)?;
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
    resque::remove_worker(state.redis.clone(), &path.0)
        .await
        .map_err(resque_error_map)?;
    Ok(HttpResponse::Ok().body("worker removed"))
}

pub async fn static_assets(req: HttpRequest) -> actix_web::Result<fs::NamedFile> {
    let path: std::path::PathBuf = req
        .match_info()
        .query("filename")
        .parse()
        .map_err(resque_error_map)?;
    let root = Path::new("./public");
    let file = fs::NamedFile::open(root.join(path))
        .or_else(|_e| fs::NamedFile::open("./public/index.html"))?;
    Ok(file.use_last_modified(true))
}

pub async fn home(_req: HttpRequest) -> actix_web::Result<fs::NamedFile> {
    let file = fs::NamedFile::open("./public/index.html")?;
    Ok(file.use_last_modified(true))
}
