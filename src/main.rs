#[macro_use]
extern crate actix_web;
extern crate redis;

use actix_files as fs;
use actix_web::http::StatusCode;
use actix_web::{error, web, App, HttpRequest, HttpResponse, HttpServer};
mod resque;
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

#[get("/stats")]
fn resque_stats(client: web::Data<redis::Client>) -> actix_web::Result<HttpResponse> {
    let mut con = get_redis_connection(&client)?;
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
    client: web::Data<redis::Client>,
) -> actix_web::Result<HttpResponse> {
    let mut con = get_redis_connection(&client)?;
    let start_at = query.from_job.unwrap_or(0);
    let results = resque::queue_details(&mut con, &path.0, start_at, start_at + 9)
        .map_err(resque_error_map)?;
    Ok(HttpResponse::Ok().json(results))
}

#[get("/failed")]
fn failed_jobs(
    query: web::Query<JobParam>,
    client: web::Data<redis::Client>,
) -> actix_web::Result<HttpResponse> {
    let mut con = get_redis_connection(&client)?;
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
fn active_workers(client: web::Data<redis::Client>) -> actix_web::Result<HttpResponse> {
    let mut con = get_redis_connection(&client)?;
    let workers = ResqueWorkers {
        data: resque::active_workers(&mut con).map_err(resque_error_map)?,
    };
    Ok(HttpResponse::Ok().json(workers))
}

#[delete("/failed")]
fn delete_failed_jobs(client: web::Data<redis::Client>) -> actix_web::Result<HttpResponse> {
    let mut con = get_redis_connection(&client)?;
    let deleted = resque::clear_queue(&mut con, "failed").map_err(resque_error_map)?;
    Ok(HttpResponse::Ok().body(deleted.to_string()))
}

#[delete("/failed_job")]
fn delete_failed_job(
    job: web::Json<DeleteFailedParam>,
    client: web::Data<redis::Client>,
) -> actix_web::Result<HttpResponse> {
    let mut con = get_redis_connection(&client)?;
    resque::delete_failed_job(&mut con, &job.id).map_err(resque_error_map)?;
    Ok(HttpResponse::Ok().body("job removed"))
}

#[delete("/queue/{name}")]
fn delete_queue_contents(
    path: web::Path<(String,)>,
    client: web::Data<redis::Client>,
) -> actix_web::Result<HttpResponse> {
    let mut con = get_redis_connection(&client)?;
    let queueKey = format!("queue:{}", path.0);
    let deleted = resque::clear_queue(&mut con, &queueKey).map_err(resque_error_map)?;
    Ok(HttpResponse::Ok().body(deleted.to_string()))
}

fn index(req: HttpRequest) -> actix_web::Result<fs::NamedFile> {
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
    Ok(fs::NamedFile::open("./public/index.html")?)
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
        .unwrap_or(0);
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

fn main() {
    std::env::set_var("RUST_LOG", "actix_web=debug");
    env_logger::init();
    let redis_client = create_redis_client().unwrap();
    let sub_uri = std::env::var("SUB_URI").unwrap_or("".to_string());
    let data = web::Data::new(redis_client);
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
                    .route("{filename:.*}", web::get().to(index)),
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