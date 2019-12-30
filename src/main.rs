use actix_files as fs;
use actix_web::http::StatusCode;
use actix_web::{delete, error, get, post, web, App, HttpRequest, HttpResponse, HttpServer};
mod resque;
use plugin_manager::Action;
use serde_derive::{Deserialize, Serialize};
use std::path::Path;

fn resque_error_map<T>(err: T) -> error::InternalError<T> {
    error::InternalError::new(err, StatusCode::INTERNAL_SERVER_ERROR)
}

fn get_redis_connection(
    client: &redis::Client,
) -> Result<redis::Connection, error::InternalError<redis::RedisError>> {
    client.get_connection().map_err(resque_error_map)
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

struct AppState {
    client: redis::Client,
    plugins: plugin_manager::PluginManager,
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
    let results = resque::queue_details(&mut con, &path.0, start_at, start_at + 9)
        .map_err(resque_error_map)?;
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

fn static_assets(req: HttpRequest) -> actix_web::Result<fs::NamedFile> {
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

fn get_redis_passwd() -> Option<String> {
    match std::env::var("REDIS_PASSWORD") {
        Ok(val) => Some(val),
        Err(_) => None,
    }
}

fn get_detailed_connection() -> redis::RedisResult<redis::Client> {
    let addr = std::env::var("REDIS_HOSTNAME").unwrap_or("".to_string());
    let port: u16 = std::env::var("REDIS_PORT")
        .unwrap_or("".to_string())
        .parse()
        .unwrap_or(6379);
    let passwd = get_redis_passwd();
    let db: i64 = std::env::var("REDIS_DATABASE")
        .unwrap_or("".to_string())
        .parse()
        .unwrap_or(0);
    let con_addr = redis::ConnectionAddr::Tcp(addr, port);
    let conn_info = redis::ConnectionInfo {
        db: db,
        addr: Box::new(con_addr),
        passwd: passwd,
    };
    redis::Client::open(conn_info)
}

fn create_redis_client() -> redis::RedisResult<redis::Client> {
    match std::env::var("REDIS_CONNECTION_STRING") {
        Ok(val) => redis::Client::open(val.as_ref()),
        Err(_) => get_detailed_connection(),
    }
}

fn make_plugin_manager() -> Result<plugin_manager::PluginManager, std::io::Error> {
    let mut plugin_manager = plugin_manager::PluginManager::new();
    if let Ok(val) = std::env::var("RESQUE_PLUGIN_DIR") {
        plugin_manager.load_directory(val.as_str())?;
    }
    Ok(plugin_manager)
}

fn main() {
    std::env::set_var("RUST_LOG", "info");
    env_logger::init();
    let redis_client = create_redis_client().unwrap();
    let plugin_manager = make_plugin_manager().expect("error loading plugins");
    let sub_uri = std::env::var("SUB_URI").unwrap_or("".to_string());
    let data = web::Data::new(AppState {
        client: redis_client,
        plugins: plugin_manager,
    });
    let result = HttpServer::new(move || {
        App::new()
            .wrap(actix_web::middleware::Logger::default())
            .register_data(data.clone())
            .service(
                web::scope(&sub_uri)
                    .service(home)
                    .service(resque_stats)
                    .service(failed_jobs)
                    .service(active_workers)
                    .service(queue_details)
                    .service(delete_failed_jobs)
                    .service(delete_queue_contents)
                    .service(delete_failed_job)
                    .service(retry_failed_job)
                    .service(retry_all)
                    .route("{filename:.*}", web::get().to(static_assets)),
            )
    })
    .bind("0.0.0.0:8080")
    .expect("unable to bind server")
    .run();
    match result {
        Ok(_) => println!("Server listening"),
        Err(e) => println!("Unable to start: {}", e),
    }
}
